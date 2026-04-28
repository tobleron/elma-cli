package convert

import (
	"bufio"
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"unicode"

	"pdfrag/logging"

	"github.com/gen2brain/go-fitz"
	"go.uber.org/zap"
)

const defaultMarkdownOutputDir = "./output/markdown"

// DefaultMarkdownOutputDir returns the default output directory for markdown files.
func DefaultMarkdownOutputDir() string {
	return defaultMarkdownOutputDir
}

// MarkdownOptions controls PDF-to-markdown conversion behavior.
type MarkdownOptions struct {
	OutputDir string
}

// PDFToMarkdown converts a PDF to markdown and writes it to disk.
// It returns the markdown output path on success.
func PDFToMarkdown(ctx context.Context, pdfPath string, opts MarkdownOptions) (string, error) {
	outputDir := opts.OutputDir
	if outputDir == "" {
		outputDir = defaultMarkdownOutputDir
	}
	if err := os.MkdirAll(outputDir, 0o755); err != nil {
		return "", fmt.Errorf("create output dir: %w", err)
	}

	doc, err := fitz.New(pdfPath)
	if err != nil {
		return "", fmt.Errorf("open pdf: %w", err)
	}
	defer doc.Close()

	outputPath := filepath.Join(outputDir, markdownFilename(pdfPath))
	file, err := os.Create(outputPath)
	if err != nil {
		return "", fmt.Errorf("create markdown file: %w", err)
	}
	defer func() {
		if closeErr := file.Close(); closeErr != nil {
			logging.L().Warn("failed to close markdown file", zap.String("path", outputPath), zap.Error(closeErr))
		}
	}()

	writer := bufio.NewWriter(file)
	if _, err := fmt.Fprintf(writer, "# %s\n\n", strings.TrimSuffix(filepath.Base(pdfPath), filepath.Ext(pdfPath))); err != nil {
		return "", fmt.Errorf("write markdown header: %w", err)
	}

	pageCount := doc.NumPage()
	for page := 0; page < pageCount; page++ {
		if err := checkContext(ctx); err != nil {
			return "", err
		}
		if _, err := fmt.Fprintf(writer, "## Page %d\n\n", page+1); err != nil {
			return "", fmt.Errorf("write page header: %w", err)
		}
		text, err := doc.Text(page)
		if err != nil {
			logging.L().Warn("failed to extract page text", zap.String("path", pdfPath), zap.Int("page", page+1), zap.Error(err))
			if _, err := fmt.Fprintln(writer); err != nil {
				return "", fmt.Errorf("write page placeholder: %w", err)
			}
			continue
		}
		normalized := normalizeText(text)
		if normalized != "" {
			if _, err := fmt.Fprintln(writer, normalized); err != nil {
				return "", fmt.Errorf("write page content: %w", err)
			}
		}
		if _, err := fmt.Fprintln(writer); err != nil {
			return "", fmt.Errorf("write page spacing: %w", err)
		}
	}

	if err := writer.Flush(); err != nil {
		return "", fmt.Errorf("flush markdown file: %w", err)
	}

	return outputPath, nil
}

func markdownFilename(pdfPath string) string {
	name := strings.TrimSuffix(filepath.Base(pdfPath), filepath.Ext(pdfPath))
	if name == "" {
		name = "document"
	}
	return name + ".md"
}

func normalizeText(input string) string {
	if input == "" {
		return ""
	}
	input = strings.ReplaceAll(input, "\r\n", "\n")
	input = strings.ReplaceAll(input, "\r", "\n")
	lines := strings.Split(input, "\n")
	paragraphs := make([]string, 0, len(lines)/4)
	current := make([]string, 0, 8)

	flushParagraph := func() {
		if len(current) == 0 {
			return
		}
		paragraph := strings.Join(current, " ")
		paragraphs = append(paragraphs, paragraph)
		current = current[:0]
	}

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" {
			flushParagraph()
			continue
		}
		if len(current) == 0 {
			current = append(current, trimmed)
			continue
		}
		last := current[len(current)-1]
		if strings.HasSuffix(last, "-") && startsWithLower(trimmed) {
			current[len(current)-1] = strings.TrimSuffix(last, "-") + trimmed
			continue
		}
		current = append(current, trimmed)
	}
	flushParagraph()

	return strings.Join(paragraphs, "\n\n")
}

func startsWithLower(value string) bool {
	for _, r := range value {
		if unicode.IsSpace(r) {
			continue
		}
		return unicode.IsLower(r)
	}
	return false
}

func checkContext(ctx context.Context) error {
	if ctx == nil {
		return nil
	}
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
		return nil
	}
}
