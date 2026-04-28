package storage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"sort"
	"strings"

	"pdfrag/embeddings"
	"pdfrag/logging"

	"go.uber.org/zap"
)

// RelatedSearchOptions controls document similarity queries.
type RelatedSearchOptions struct {
	TopK          int
	MinSimilarity float64
}

// RelatedDocument captures metadata and similarity for a related document.
type RelatedDocument struct {
	DocumentID   int64   `json:"document_id"`
	Filename     string  `json:"filename"`
	Title        *string `json:"title,omitempty"`
	Authors      *string `json:"authors,omitempty"`
	Similarity   float64 `json:"similarity"`
	Summary      string  `json:"summary,omitempty"`
	MarkdownPath string  `json:"markdown_path,omitempty"`
}

// FindRelatedDocuments returns top related documents by document-level embeddings.
func FindRelatedDocuments(ctx context.Context, db *sql.DB, filename string, opts RelatedSearchOptions) ([]RelatedDocument, error) {
	if db == nil {
		return nil, errors.New("database is nil")
	}
	if ctx == nil {
		ctx = context.Background()
	}
	filename = strings.TrimSpace(filename)
	if filename == "" {
		return nil, errors.New("filename is required")
	}

	resolvedTopK := opts.TopK
	if resolvedTopK <= 0 {
		resolvedTopK = 10
	}
	minSimilarity := opts.MinSimilarity
	if minSimilarity < 0 {
		minSimilarity = 0
	}

	var (
		seedID        int64
		seedEmbedding []byte
	)
	row := db.QueryRowContext(ctx, `SELECT d.id, de.embedding FROM documents d
		JOIN document_embeddings de ON de.document_id = d.id
		WHERE d.filename = ?`, filename)
	if err := row.Scan(&seedID, &seedEmbedding); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return nil, ErrDocumentNotFound
		}
		return nil, err
	}
	seedVec, err := embeddings.DecodeEmbedding(seedEmbedding)
	if err != nil {
		return nil, err
	}
	if len(seedVec) == 0 {
		return nil, fmt.Errorf("document embedding missing for %q", filename)
	}
	seedNorm, err := vectorNorm(seedVec)
	if err != nil {
		return nil, err
	}

	rows, err := db.QueryContext(ctx, `SELECT d.id, d.filename, d.title, d.authors, d.markdown_path, de.embedding, c.content
		FROM document_embeddings de
		JOIN documents d ON de.document_id = d.id
		LEFT JOIN chunks c ON c.document_id = d.id AND c.chunk_index = 0`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	results := make([]RelatedDocument, 0)
	for rows.Next() {
		var (
			docID        int64
			docFilename  string
			title        sql.NullString
			authors      sql.NullString
			markdownPath string
			embedding    []byte
			summary      sql.NullString
		)
		if err := rows.Scan(&docID, &docFilename, &title, &authors, &markdownPath, &embedding, &summary); err != nil {
			return nil, err
		}
		if docID == seedID {
			continue
		}
		vec, err := embeddings.DecodeEmbedding(embedding)
		if err != nil {
			return nil, err
		}
		if len(vec) == 0 {
			continue
		}
		if len(vec) != len(seedVec) {
			logging.L().Warn("embedding dimension mismatch",
				zap.Int("seed_dim", len(seedVec)),
				zap.Int("doc_dim", len(vec)),
				zap.Int64("document_id", docID),
			)
			continue
		}
		similarity, err := cosineSimilarity(seedVec, vec, seedNorm)
		if err != nil {
			continue
		}
		if similarity < minSimilarity {
			continue
		}
		results = append(results, RelatedDocument{
			DocumentID:   docID,
			Filename:     docFilename,
			Title:        nullableString(title),
			Authors:      nullableString(authors),
			Similarity:   similarity,
			Summary:      summary.String,
			MarkdownPath: markdownPath,
		})
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}

	sortRelated(results)
	if len(results) > resolvedTopK {
		results = results[:resolvedTopK]
	}
	return results, nil
}

func sortRelated(results []RelatedDocument) {
	sort.Slice(results, func(i, j int) bool {
		if results[i].Similarity == results[j].Similarity {
			return results[i].Filename < results[j].Filename
		}
		return results[i].Similarity > results[j].Similarity
	})
}
