package discover

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"strings"
	"time"
)

// DocumentKind distinguishes supported file types.
type DocumentKind string

const (
	DocumentPDF      DocumentKind = "pdf"
	DocumentMarkdown DocumentKind = "markdown"
)

// Document represents a discovered file with its type.
type Document struct {
	Path string
	Kind DocumentKind
}

// Options controls PDF discovery behavior.
type Options struct {
	// Progress receives periodic updates during scanning.
	Progress ProgressFunc
	// ProgressEvery controls how often progress updates are emitted.
	ProgressEvery int
}

// Stats tracks discovery progress.
type Stats struct {
	Visited  int
	PDFs     int
	Markdown int
	Skipped  int
	Errors   int
	Elapsed  time.Duration
	Done     bool
}

// ProgressFunc receives discovery updates.
type ProgressFunc func(stats Stats)

// FindDocuments recursively scans a directory for PDF and Markdown files.
func FindDocuments(ctx context.Context, root string, opts Options) ([]Document, Stats, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	stats := Stats{}
	if strings.TrimSpace(root) == "" {
		return nil, stats, errors.New("root path is required")
	}

	info, err := os.Stat(root)
	if err != nil {
		return nil, stats, err
	}
	if !info.IsDir() {
		if kind, ok := detectDocumentKind(root); ok {
			stats.Visited = 1
			switch kind {
			case DocumentPDF:
				stats.PDFs = 1
			case DocumentMarkdown:
				stats.Markdown = 1
			}
			stats.Elapsed = 0
			stats.Done = true
			return []Document{{Path: root, Kind: kind}}, stats, nil
		}
		stats.Visited = 1
		stats.Skipped = 1
		stats.Done = true
		return nil, stats, nil
	}

	progressEvery := opts.ProgressEvery
	if progressEvery <= 0 {
		progressEvery = 200
	}

	start := time.Now()
	documents := make([]Document, 0, 32)
	var processed int
	report := func(done bool) {
		if opts.Progress == nil {
			return
		}
		stats.Elapsed = time.Since(start)
		stats.Done = done
		opts.Progress(stats)
	}

	walkErr := filepath.WalkDir(root, func(path string, entry os.DirEntry, err error) error {
		if ctx.Err() != nil {
			return ctx.Err()
		}
		stats.Visited++
		processed++
		if err != nil {
			stats.Errors++
			if processed%progressEvery == 0 {
				report(false)
			}
			return nil
		}

		if entry.Type()&os.ModeSymlink != 0 {
			resolved, statErr := os.Stat(path)
			if statErr != nil {
				stats.Errors++
				if processed%progressEvery == 0 {
					report(false)
				}
				return nil
			}
			if resolved.IsDir() {
				stats.Skipped++
				if processed%progressEvery == 0 {
					report(false)
				}
				return nil
			}
			if kind, ok := detectDocumentKind(path); ok {
				documents = append(documents, Document{Path: path, Kind: kind})
				if kind == DocumentPDF {
					stats.PDFs++
				} else {
					stats.Markdown++
				}
			} else {
				stats.Skipped++
			}
			if processed%progressEvery == 0 {
				report(false)
			}
			return nil
		}

		if entry.IsDir() {
			if processed%progressEvery == 0 {
				report(false)
			}
			return nil
		}

		if kind, ok := detectDocumentKind(path); ok {
			documents = append(documents, Document{Path: path, Kind: kind})
			if kind == DocumentPDF {
				stats.PDFs++
			} else {
				stats.Markdown++
			}
		} else {
			stats.Skipped++
		}
		if processed%progressEvery == 0 {
			report(false)
		}
		return nil
	})

	report(true)
	stats.Done = true
	stats.Elapsed = time.Since(start)
	return documents, stats, walkErr
}

// FindPDFs recursively scans a directory for PDF files.
func FindPDFs(ctx context.Context, root string, opts Options) ([]string, Stats, error) {
	documents, stats, err := FindDocuments(ctx, root, opts)
	if err != nil {
		return nil, stats, err
	}
	pdfs := make([]string, 0, len(documents))
	for _, doc := range documents {
		if doc.Kind == DocumentPDF {
			pdfs = append(pdfs, doc.Path)
		}
	}
	return pdfs, stats, nil
}

func detectDocumentKind(path string) (DocumentKind, bool) {
	if isPDF(path) {
		return DocumentPDF, true
	}
	if isMarkdown(path) {
		return DocumentMarkdown, true
	}
	return "", false
}

func isPDF(path string) bool {
	ext := strings.ToLower(filepath.Ext(path))
	return ext == ".pdf"
}

func isMarkdown(path string) bool {
	ext := strings.ToLower(filepath.Ext(path))
	switch ext {
	case ".md", ".markdown":
		return true
	default:
		return false
	}
}
