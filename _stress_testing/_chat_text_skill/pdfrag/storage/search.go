package storage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"math"
	"sort"

	"pdfrag/embeddings"
	"pdfrag/logging"

	"go.uber.org/zap"
)

// SearchOptions controls query-time semantic search behavior.
type SearchOptions struct {
	TopK           int
	MinSimilarity  float64
	MaxPerDocument int
}

// SearchResult represents a matched chunk.
type SearchResult struct {
	DocumentID   int64
	ChunkID      int64
	Filename     string
	Content      string
	PageNumber   int
	SectionTitle string
	Similarity   float64
}

// SearchEmbeddings computes cosine similarity between query embedding and stored embeddings.
func SearchEmbeddings(ctx context.Context, db *sql.DB, queryEmbedding []float32, opts SearchOptions) ([]SearchResult, error) {
	if db == nil {
		return nil, errors.New("database is nil")
	}
	if len(queryEmbedding) == 0 {
		return nil, errors.New("query embedding is empty")
	}
	resolvedTopK := opts.TopK
	if resolvedTopK <= 0 {
		resolvedTopK = 10
	}
	minSimilarity := opts.MinSimilarity
	if minSimilarity < 0 {
		minSimilarity = 0
	}
	maxPerDocument := opts.MaxPerDocument
	if maxPerDocument <= 0 {
		maxPerDocument = 3
	}

	queryNorm, err := vectorNorm(queryEmbedding)
	if err != nil {
		return nil, err
	}

	rows, err := db.QueryContext(ctx, `SELECT d.id, d.filename, c.id, c.content, c.page_number, c.section_title, e.embedding
		FROM embeddings e
		JOIN chunks c ON e.chunk_id = c.id
		JOIN documents d ON c.document_id = d.id`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	results := make([]SearchResult, 0)
	for rows.Next() {
		var (
			docID     int64
			filename  string
			chunkID   int64
			content   string
			pageNum   sql.NullInt64
			section   sql.NullString
			embedding []byte
		)
		if err := rows.Scan(&docID, &filename, &chunkID, &content, &pageNum, &section, &embedding); err != nil {
			return nil, err
		}
		vec, err := embeddings.DecodeEmbedding(embedding)
		if err != nil {
			return nil, err
		}
		if len(vec) == 0 {
			continue
		}
		if len(vec) != len(queryEmbedding) {
			logging.L().Warn("embedding dimension mismatch",
				zap.Int("query_dim", len(queryEmbedding)),
				zap.Int("doc_dim", len(vec)),
				zap.Int64("chunk_id", chunkID),
			)
			continue
		}
		similarity, err := cosineSimilarity(queryEmbedding, vec, queryNorm)
		if err != nil {
			continue
		}
		if similarity < minSimilarity {
			continue
		}
		res := SearchResult{
			DocumentID: docID,
			ChunkID:    chunkID,
			Filename:   filename,
			Content:    content,
			Similarity: similarity,
		}
		if pageNum.Valid {
			res.PageNumber = int(pageNum.Int64)
		}
		if section.Valid {
			res.SectionTitle = section.String
		}
		results = append(results, res)
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}

	sort.Slice(results, func(i, j int) bool {
		return results[i].Similarity > results[j].Similarity
	})

	filtered := make([]SearchResult, 0, resolvedTopK)
	counts := make(map[int64]int)
	for _, result := range results {
		if counts[result.DocumentID] >= maxPerDocument {
			continue
		}
		counts[result.DocumentID]++
		filtered = append(filtered, result)
		if len(filtered) >= resolvedTopK {
			break
		}
	}
	return filtered, nil
}

func cosineSimilarity(query []float32, candidate []float32, queryNorm float64) (float64, error) {
	if len(query) == 0 || len(candidate) == 0 {
		return 0, errors.New("empty embedding vector")
	}
	var dot float64
	var candidateNorm float64
	for i, q := range query {
		c := float64(candidate[i])
		dot += float64(q) * c
		candidateNorm += c * c
	}
	if candidateNorm == 0 {
		return 0, errors.New("invalid candidate embedding norm")
	}
	if queryNorm == 0 {
		return 0, errors.New("invalid query embedding norm")
	}
	return dot / (math.Sqrt(candidateNorm) * queryNorm), nil
}

func vectorNorm(vec []float32) (float64, error) {
	var sum float64
	for _, v := range vec {
		f := float64(v)
		sum += f * f
	}
	if sum == 0 || math.IsNaN(sum) || math.IsInf(sum, 0) {
		return 0, fmt.Errorf("invalid embedding norm")
	}
	return math.Sqrt(sum), nil
}
