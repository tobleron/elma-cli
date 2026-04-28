package cmd

import (
	"archive/zip"
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"pdfrag/convert"
	"pdfrag/logging"
	"pdfrag/storage"

	"github.com/spf13/cobra"
	"go.uber.org/zap"
)

func newImportCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:          "import <backup.zip>",
		Short:        "Import database and markdown files from a backup zip",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			inputPath := strings.TrimSpace(args[0])
			if inputPath == "" {
				return fmt.Errorf("backup path is required")
			}
			if err := importBackup(cmd.Context(), inputPath); err != nil {
				return err
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Imported backup from %s\n", inputPath)
			return nil
		},
	}

	return cmd
}

func importBackup(ctx context.Context, inputPath string) error {
	if ctx == nil {
		ctx = context.Background()
	}
	stat, err := os.Stat(inputPath)
	if err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("backup not found at %s", inputPath)
		}
		return fmt.Errorf("stat backup: %w", err)
	}
	file, err := os.Open(inputPath)
	if err != nil {
		return fmt.Errorf("open backup: %w", err)
	}
	defer file.Close()

	zipReader, err := zip.NewReader(file, stat.Size())
	if err != nil {
		return fmt.Errorf("read backup zip: %w", err)
	}

	manifest, err := loadManifest(zipReader)
	if err != nil {
		return err
	}
	if manifest.Version > backupVersion {
		return fmt.Errorf("backup version %d is newer than supported %d", manifest.Version, backupVersion)
	}

	files := buildZipFileIndex(zipReader)
	if err := restoreDatabase(ctx, files, manifest); err != nil {
		return err
	}
	if err := restoreMarkdown(ctx, files, manifest); err != nil {
		return err
	}

	return nil
}

func restoreDatabase(ctx context.Context, files map[string]*zip.File, manifest backupManifest) error {
	entry := manifest.Database
	if entry.ArchivePath == "" {
		return fmt.Errorf("manifest missing database entry")
	}
	zipFile, ok := files[entry.ArchivePath]
	if !ok {
		return fmt.Errorf("database entry %s not found in backup", entry.ArchivePath)
	}

	targetPath := strings.TrimSpace(appConfig.Database.Path)
	if targetPath == "" {
		targetPath = storage.DefaultDuckDBPath
	}
	if err := ensureBackupTarget(targetPath); err != nil {
		return err
	}

	tempPath := targetPath + ".importing"
	checksum, _, err := extractZipFile(zipFile, tempPath)
	if err != nil {
		return err
	}
	if entry.SHA256 != "" && checksum != entry.SHA256 {
		_ = os.Remove(tempPath)
		return fmt.Errorf("checksum mismatch for database entry")
	}
	if err := moveFile(tempPath, targetPath); err != nil {
		return err
	}

	// Touch the database to ensure schema upgrades are applied after restore.
	db, err := storage.InitDuckDB(ctx, targetPath)
	if err != nil {
		return err
	}
	if err := db.Close(); err != nil {
		return fmt.Errorf("close database after restore: %w", err)
	}

	return nil
}

func restoreMarkdown(ctx context.Context, files map[string]*zip.File, manifest backupManifest) error {
	if len(manifest.Markdown) == 0 {
		return nil
	}
	markdownDir := convert.DefaultMarkdownOutputDir()
	if markdownDir == "" {
		return fmt.Errorf("markdown output directory is empty")
	}
	if err := os.MkdirAll(markdownDir, 0o755); err != nil {
		return fmt.Errorf("create markdown directory: %w", err)
	}

	for _, entry := range manifest.Markdown {
		if entry.ArchivePath == "" {
			return fmt.Errorf("manifest markdown entry missing archive path")
		}
		zipFile, ok := files[entry.ArchivePath]
		if !ok {
			return fmt.Errorf("markdown entry %s not found in backup", entry.ArchivePath)
		}
		targetPath := filepath.Join(markdownDir, filepath.Base(entry.ArchivePath))
		checksum, _, err := extractZipFile(zipFile, targetPath)
		if err != nil {
			return err
		}
		if entry.SHA256 != "" && checksum != entry.SHA256 {
			return fmt.Errorf("checksum mismatch for markdown entry %s", entry.ArchivePath)
		}
	}

	if err := updateMarkdownPaths(ctx, manifest, markdownDir); err != nil {
		return err
	}

	return nil
}

func updateMarkdownPaths(ctx context.Context, manifest backupManifest, markdownDir string) error {
	dbPath := strings.TrimSpace(appConfig.Database.Path)
	if dbPath == "" {
		dbPath = storage.DefaultDuckDBPath
	}
	if _, err := os.Stat(dbPath); err != nil {
		return fmt.Errorf("stat database: %w", err)
	}
	db, err := storage.InitDuckDB(ctx, dbPath)
	if err != nil {
		return err
	}
	defer func() {
		if closeErr := db.Close(); closeErr != nil {
			logging.L().Warn("failed to close database", zap.Error(closeErr))
		}
	}()

	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	for _, entry := range manifest.Markdown {
		if entry.Filename == "" {
			return rollback(tx, fmt.Errorf("manifest markdown entry missing filename"))
		}
		newPath := filepath.Join(markdownDir, filepath.Base(entry.ArchivePath))
		if _, err := tx.ExecContext(ctx, "UPDATE documents SET markdown_path = ? WHERE filename = ?", newPath, entry.Filename); err != nil {
			return rollback(tx, err)
		}
	}
	if err := tx.Commit(); err != nil {
		return err
	}
	return nil
}

func ensureBackupTarget(targetPath string) error {
	if err := os.MkdirAll(filepath.Dir(targetPath), 0o755); err != nil {
		return fmt.Errorf("create database directory: %w", err)
	}
	if _, err := os.Stat(targetPath); err == nil {
		backupName := fmt.Sprintf("%s.bak-%s", targetPath, time.Now().UTC().Format("20060102T150405"))
		if err := os.Rename(targetPath, backupName); err != nil {
			return fmt.Errorf("backup existing database: %w", err)
		}
		logging.L().Info("backed up existing database", zap.String("backup", backupName))
	}
	return nil
}
