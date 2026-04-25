package convert

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"pdfrag/logging"

	"github.com/gen2brain/go-fitz"
	"go.uber.org/zap"
)

// DocumentMetadata captures optional metadata extracted from a PDF.
type DocumentMetadata struct {
	Title           *string
	Authors         *string
	PublicationDate *string
	DOI             *string
	Filename        string
	FileSize        int64
	PageCount       int
	FileModTime     time.Time
	FileHash        string
}

var doiRegex = regexp.MustCompile(`10\.\d{4,}/[-._;()/:a-zA-Z0-9]+`)

// ExtractPDFMetadata extracts metadata and DOI information from a PDF.
func ExtractPDFMetadata(ctx context.Context, pdfPath string) (DocumentMetadata, error) {
	info, err := os.Stat(pdfPath)
	if err != nil {
		return DocumentMetadata{}, fmt.Errorf("stat pdf: %w", err)
	}

	doc, err := fitz.New(pdfPath)
	if err != nil {
		return DocumentMetadata{}, fmt.Errorf("open pdf: %w", err)
	}
	defer doc.Close()

	meta := doc.Metadata()
	title := cleanMetadataValue(meta["title"])
	authors := cleanMetadataValue(meta["author"])
	pubDate := cleanMetadataValue(meta["creationDate"])
	if pubDate == "" {
		pubDate = cleanMetadataValue(meta["modDate"])
	}

	doi, err := findDOI(ctx, doc, pdfPath)
	if err != nil {
		return DocumentMetadata{}, err
	}

	return DocumentMetadata{
		Title:           optionalString(title),
		Authors:         optionalString(authors),
		PublicationDate: optionalString(pubDate),
		DOI:             optionalString(doi),
		Filename:        filepath.Base(pdfPath),
		FileSize:        info.Size(),
		PageCount:       doc.NumPage(),
		FileModTime:     info.ModTime(),
	}, nil
}

func optionalString(value string) *string {
	value = strings.TrimSpace(value)
	if value == "" {
		return nil
	}
	copy := value
	return &copy
}

func cleanMetadataValue(value string) string {
	if value == "" {
		return ""
	}
	if idx := strings.IndexByte(value, 0); idx >= 0 {
		value = value[:idx]
	}
	return strings.TrimSpace(value)
}

func findDOI(ctx context.Context, doc *fitz.Document, pdfPath string) (string, error) {
	pageCount := doc.NumPage()
	pagesToScan := pageCount
	if pagesToScan > 2 {
		pagesToScan = 2
	}
	for page := 0; page < pagesToScan; page++ {
		if err := checkContext(ctx); err != nil {
			return "", err
		}
		text, err := doc.Text(page)
		if err != nil {
			logging.L().Warn("failed to extract page text for DOI", zap.String("path", pdfPath), zap.Int("page", page+1), zap.Error(err))
			continue
		}
		if doi := doiRegex.FindString(text); doi != "" {
			return doi, nil
		}
	}
	return "", nil
}
