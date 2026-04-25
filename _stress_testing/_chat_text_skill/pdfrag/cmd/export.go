package cmd

import (
	"archive/zip"
	"context"
	"database/sql"
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"time"

	"pdfrag/logging"
	"pdfrag/storage"

	"github.com/spf13/cobra"
	"go.uber.org/zap"
)

type markdownEntry struct {
	Filename string
	Path     string
}

func newExportCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:          "export <output.zip>",
		Short:        "Export database and markdown files to a backup zip",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			outputPath := strings.TrimSpace(args[0])
			if outputPath == "" {
				return fmt.Errorf("output path is required")
			}
			if err := exportBackup(cmd.Context(), outputPath); err != nil {
				return err
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Exported backup to %s\n", outputPath)
			return nil
		},
	}

	return cmd
}

func exportBackup(ctx context.Context, outputPath string) error {
	if ctx == nil {
		ctx = context.Background()
	}
	outputDir := filepath.Dir(outputPath)
	if outputDir != "." && outputDir != "" {
		if err := os.MkdirAll(outputDir, 0o755); err != nil {
			return fmt.Errorf("create output directory: %w", err)
		}
	}

	dbPath := strings.TrimSpace(appConfig.Database.Path)
	if dbPath == "" {
		dbPath = storage.DefaultDuckDBPath
	}
	if _, err := os.Stat(dbPath); err != nil {
		if os.IsNotExist(err) {
			return fmt.Errorf("database not found at %s", dbPath)
		}
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

	markdownFiles, err := fetchMarkdownEntries(ctx, db)
	if err != nil {
		return err
	}

	zipFile, err := os.Create(outputPath)
	if err != nil {
		return fmt.Errorf("create backup file: %w", err)
	}
	defer func() {
		if closeErr := zipFile.Close(); closeErr != nil {
			logging.L().Warn("failed to close backup file", zap.String("path", outputPath), zap.Error(closeErr))
		}
	}()

	zipWriter := zip.NewWriter(zipFile)
	defer func() {
		if closeErr := zipWriter.Close(); closeErr != nil {
			logging.L().Warn("failed to close zip writer", zap.Error(closeErr))
		}
	}()

	manifest := backupManifest{
		Version:   backupVersion,
		CreatedAt: time.Now().UTC(),
		Metadata:  map[string]string{"db_path": dbPath},
	}

	dbArchivePath, err := buildArchivePath("db", filepath.Base(dbPath))
	if err != nil {
		return err
	}
	dbEntry, err := addFileToZip(zipWriter, dbPath, dbArchivePath)
	if err != nil {
		return err
	}
	manifest.Database = dbEntry

	archiveNames := make(map[string]struct{})
	archiveNames[dbEntry.ArchivePath] = struct{}{}

	for _, entry := range markdownFiles {
		if entry.Path == "" {
			return fmt.Errorf("missing markdown path for %s", entry.Filename)
		}
		if _, err := os.Stat(entry.Path); err != nil {
			if os.IsNotExist(err) {
				return fmt.Errorf("markdown file not found: %s", entry.Path)
			}
			return fmt.Errorf("stat markdown file %s: %w", entry.Path, err)
		}
		archivePath, err := buildArchivePath("markdown", entry.Path)
		if err != nil {
			return err
		}
		if _, exists := archiveNames[archivePath]; exists {
			return fmt.Errorf("duplicate markdown archive name: %s", archivePath)
		}
		archiveNames[archivePath] = struct{}{}
		mdEntry, err := addFileToZip(zipWriter, entry.Path, archivePath)
		if err != nil {
			return err
		}
		mdEntry.Filename = entry.Filename
		manifest.Markdown = append(manifest.Markdown, mdEntry)
	}

	if len(manifest.Markdown) > 1 {
		sort.Slice(manifest.Markdown, func(i, j int) bool {
			return strings.ToLower(manifest.Markdown[i].Filename) < strings.ToLower(manifest.Markdown[j].Filename)
		})
	}

	if err := writeManifest(zipWriter, manifest); err != nil {
		return err
	}

	return nil
}

func fetchMarkdownEntries(ctx context.Context, db *sql.DB) ([]markdownEntry, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	rows, err := db.QueryContext(ctx, "SELECT filename, markdown_path FROM documents ORDER BY filename ASC")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var entries []markdownEntry
	for rows.Next() {
		var filename string
		var markdownPath string
		if err := rows.Scan(&filename, &markdownPath); err != nil {
			return nil, err
		}
		entries = append(entries, markdownEntry{Filename: filename, Path: markdownPath})
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	return entries, nil
}
