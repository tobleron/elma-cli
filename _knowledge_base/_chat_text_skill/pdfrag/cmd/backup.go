package cmd

import (
	"archive/zip"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"path"
	"path/filepath"
	"strings"
	"time"
)

const backupManifestName = "manifest.json"
const backupVersion = 1

// backupManifest describes the exported files and their checksums.
type backupManifest struct {
	Version   int               `json:"version"`
	CreatedAt time.Time         `json:"created_at"`
	Database  backupFileEntry   `json:"database"`
	Markdown  []backupFileEntry `json:"markdown"`
	Metadata  map[string]string `json:"metadata,omitempty"`
}

type backupFileEntry struct {
	OriginalPath string `json:"original_path"`
	ArchivePath  string `json:"archive_path"`
	SHA256       string `json:"sha256"`
	SizeBytes    int64  `json:"size_bytes"`
	Filename     string `json:"filename,omitempty"`
}

func sanitizeArchivePath(value string) (string, error) {
	if value == "" {
		return "", fmt.Errorf("archive path is empty")
	}
	clean := path.Clean(value)
	clean = strings.TrimPrefix(clean, "/")
	if clean == "." || clean == "" {
		return "", fmt.Errorf("archive path is invalid")
	}
	if strings.Contains(clean, "..") {
		return "", fmt.Errorf("archive path contains '..'")
	}
	return clean, nil
}

func addFileToZip(zipWriter *zip.Writer, srcPath, archivePath string) (backupFileEntry, error) {
	info, err := os.Stat(srcPath)
	if err != nil {
		return backupFileEntry{}, fmt.Errorf("stat %s: %w", srcPath, err)
	}
	clean, err := sanitizeArchivePath(archivePath)
	if err != nil {
		return backupFileEntry{}, err
	}
	header, err := zip.FileInfoHeader(info)
	if err != nil {
		return backupFileEntry{}, fmt.Errorf("zip header for %s: %w", srcPath, err)
	}
	header.Name = clean
	header.Method = zip.Deflate
	writer, err := zipWriter.CreateHeader(header)
	if err != nil {
		return backupFileEntry{}, fmt.Errorf("create zip entry %s: %w", clean, err)
	}
	file, err := os.Open(srcPath)
	if err != nil {
		return backupFileEntry{}, fmt.Errorf("open %s: %w", srcPath, err)
	}
	defer file.Close()

	hasher := sha256.New()
	if _, err := io.Copy(io.MultiWriter(writer, hasher), file); err != nil {
		return backupFileEntry{}, fmt.Errorf("write %s to zip: %w", srcPath, err)
	}

	return backupFileEntry{
		OriginalPath: srcPath,
		ArchivePath:  clean,
		SHA256:       hex.EncodeToString(hasher.Sum(nil)),
		SizeBytes:    info.Size(),
		Filename:     filepath.Base(srcPath),
	}, nil
}

func writeManifest(zipWriter *zip.Writer, manifest backupManifest) error {
	payload, err := json.MarshalIndent(manifest, "", "  ")
	if err != nil {
		return fmt.Errorf("encode manifest: %w", err)
	}
	entry, err := zipWriter.Create(backupManifestName)
	if err != nil {
		return fmt.Errorf("write manifest entry: %w", err)
	}
	if _, err := entry.Write(payload); err != nil {
		return fmt.Errorf("write manifest payload: %w", err)
	}
	return nil
}

func loadManifest(zipReader *zip.Reader) (backupManifest, error) {
	for _, file := range zipReader.File {
		if file.Name != backupManifestName {
			continue
		}
		rc, err := file.Open()
		if err != nil {
			return backupManifest{}, fmt.Errorf("open manifest: %w", err)
		}
		defer rc.Close()
		var manifest backupManifest
		if err := json.NewDecoder(rc).Decode(&manifest); err != nil {
			return backupManifest{}, fmt.Errorf("decode manifest: %w", err)
		}
		return manifest, nil
	}
	return backupManifest{}, fmt.Errorf("manifest not found in archive")
}

func buildZipFileIndex(zipReader *zip.Reader) map[string]*zip.File {
	files := make(map[string]*zip.File, len(zipReader.File))
	for _, file := range zipReader.File {
		files[file.Name] = file
	}
	return files
}

func extractZipFile(file *zip.File, destPath string) (string, int64, error) {
	rc, err := file.Open()
	if err != nil {
		return "", 0, fmt.Errorf("open %s: %w", file.Name, err)
	}
	defer rc.Close()

	if err := os.MkdirAll(filepath.Dir(destPath), 0o755); err != nil {
		return "", 0, fmt.Errorf("create dir for %s: %w", destPath, err)
	}

	out, err := os.Create(destPath)
	if err != nil {
		return "", 0, fmt.Errorf("create %s: %w", destPath, err)
	}
	defer func() {
		_ = out.Close()
	}()

	hasher := sha256.New()
	written, err := io.Copy(io.MultiWriter(out, hasher), rc)
	if err != nil {
		return "", 0, fmt.Errorf("extract %s: %w", file.Name, err)
	}

	return hex.EncodeToString(hasher.Sum(nil)), written, nil
}

func copyFile(src, dst string) error {
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return fmt.Errorf("create dir for %s: %w", dst, err)
	}
	in, err := os.Open(src)
	if err != nil {
		return fmt.Errorf("open %s: %w", src, err)
	}
	defer in.Close()
	out, err := os.Create(dst)
	if err != nil {
		return fmt.Errorf("create %s: %w", dst, err)
	}
	if _, err := io.Copy(out, in); err != nil {
		_ = out.Close()
		return fmt.Errorf("copy to %s: %w", dst, err)
	}
	if err := out.Close(); err != nil {
		return fmt.Errorf("close %s: %w", dst, err)
	}
	return nil
}

func moveFile(src, dst string) error {
	if err := os.MkdirAll(filepath.Dir(dst), 0o755); err != nil {
		return fmt.Errorf("create dir for %s: %w", dst, err)
	}
	if err := os.Rename(src, dst); err == nil {
		return nil
	}
	if err := copyFile(src, dst); err != nil {
		return err
	}
	return os.Remove(src)
}

func cleanArchivePath(name string) (string, error) {
	clean, err := sanitizeArchivePath(name)
	if err != nil {
		return "", err
	}
	return clean, nil
}

func buildArchivePath(dir, filename string) (string, error) {
	cleanName := filepath.Base(filename)
	if cleanName == "." || cleanName == string(filepath.Separator) || cleanName == "" {
		return "", fmt.Errorf("invalid filename for archive")
	}
	return cleanArchivePath(path.Join(dir, cleanName))
}
