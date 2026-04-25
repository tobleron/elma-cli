package cmd

import (
	"encoding/json"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const indexCheckpointVersion = 1

func defaultIndexCheckpointPath() string {
	return filepath.Join("output", "index.checkpoint.json")
}

type indexCheckpointEntry struct {
	Path      string    `json:"path,omitempty"`
	Chunks    int       `json:"chunks"`
	IndexedAt time.Time `json:"indexed_at"`
}

type indexCheckpoint struct {
	Version      int                             `json:"version"`
	RootDir      string                          `json:"root_dir"`
	DatabasePath string                          `json:"database_path"`
	CreatedAt    time.Time                       `json:"created_at"`
	UpdatedAt    time.Time                       `json:"updated_at"`
	TotalPDFs    int                             `json:"total_pdfs"`
	TotalChunks  int                             `json:"total_chunks"`
	Processed    map[string]indexCheckpointEntry `json:"processed"`
}

func newIndexCheckpoint(rootDir, dbPath string, totalPDFs int) *indexCheckpoint {
	return &indexCheckpoint{
		Version:      indexCheckpointVersion,
		RootDir:      rootDir,
		DatabasePath: dbPath,
		CreatedAt:    time.Now(),
		UpdatedAt:    time.Now(),
		TotalPDFs:    totalPDFs,
		Processed:    make(map[string]indexCheckpointEntry),
	}
}

func prepareIndexCheckpoint(path string, resume bool, rootDir, dbPath string, pdfs []string) (*indexCheckpoint, error) {
	if path == "" {
		return nil, nil
	}
	var checkpoint *indexCheckpoint
	if resume {
		loaded, err := loadIndexCheckpoint(path)
		if err != nil {
			return nil, err
		}
		if loaded != nil && checkpointMatches(loaded, rootDir, dbPath) {
			checkpoint = loaded
		}
	}
	if checkpoint == nil {
		checkpoint = newIndexCheckpoint(rootDir, dbPath, len(pdfs))
	}
	checkpoint.TotalPDFs = len(pdfs)
	checkpoint.UpdatedAt = time.Now()
	available := make(map[string]struct{}, len(pdfs))
	for _, path := range pdfs {
		available[checkpointKey(path)] = struct{}{}
	}
	for name := range checkpoint.Processed {
		if _, ok := available[name]; !ok {
			delete(checkpoint.Processed, name)
		}
	}
	checkpoint.TotalChunks = checkpoint.ProcessedChunks()
	return checkpoint, nil
}

func (cp *indexCheckpoint) ProcessedCount() int {
	if cp == nil {
		return 0
	}
	return len(cp.Processed)
}

func (cp *indexCheckpoint) ProcessedChunks() int {
	if cp == nil {
		return 0
	}
	chunks := 0
	for _, entry := range cp.Processed {
		chunks += entry.Chunks
	}
	return chunks
}

func (cp *indexCheckpoint) MarkProcessed(filename, path string, chunks int) {
	if cp == nil {
		return
	}
	key := checkpointKey(filename)
	cp.Processed[key] = indexCheckpointEntry{
		Path:      path,
		Chunks:    chunks,
		IndexedAt: time.Now(),
	}
	cp.TotalChunks += chunks
	cp.UpdatedAt = time.Now()
}

func checkpointMatches(cp *indexCheckpoint, rootDir, dbPath string) bool {
	if cp == nil {
		return false
	}
	if cp.Version != indexCheckpointVersion {
		return false
	}
	if !pathsMatch(cp.RootDir, rootDir) {
		return false
	}
	if !pathsMatch(cp.DatabasePath, dbPath) {
		return false
	}
	return true
}

func checkpointKey(path string) string {
	if path == "" {
		return ""
	}
	return strings.TrimSpace(filepath.Base(path))
}

func pathsMatch(a, b string) bool {
	if a == "" || b == "" {
		return false
	}
	return filepath.Clean(a) == filepath.Clean(b)
}

func loadIndexCheckpoint(path string) (*indexCheckpoint, error) {
	if path == "" {
		return nil, nil
	}
	file, err := os.Open(path)
	if err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil, nil
		}
		return nil, err
	}
	defer file.Close()

	var checkpoint indexCheckpoint
	decoder := json.NewDecoder(file)
	if err := decoder.Decode(&checkpoint); err != nil {
		return nil, err
	}
	if checkpoint.Processed == nil {
		checkpoint.Processed = make(map[string]indexCheckpointEntry)
	}
	return &checkpoint, nil
}

func saveIndexCheckpoint(path string, checkpoint *indexCheckpoint) error {
	if path == "" || checkpoint == nil {
		return nil
	}
	dir := filepath.Dir(path)
	if dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			return err
		}
	}

	tmpFile, err := os.CreateTemp(dir, "checkpoint-*.json")
	if err != nil {
		return err
	}
	encoder := json.NewEncoder(tmpFile)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(checkpoint); err != nil {
		_ = tmpFile.Close()
		_ = os.Remove(tmpFile.Name())
		return err
	}
	if err := tmpFile.Close(); err != nil {
		_ = os.Remove(tmpFile.Name())
		return err
	}
	return os.Rename(tmpFile.Name(), path)
}

func clearIndexCheckpoint(path string) error {
	if path == "" {
		return nil
	}
	if err := os.Remove(path); err != nil {
		if errors.Is(err, os.ErrNotExist) {
			return nil
		}
		return err
	}
	return nil
}
