package storage

import (
	"context"
	"database/sql"
	"errors"
	"time"
)

// ErrDocumentNotFound indicates a document lookup failed.
var ErrDocumentNotFound = errors.New("document not found")

// DocumentSummary represents document metadata for listing.
type DocumentSummary struct {
	ID              int64      `json:"id"`
	Filename        string     `json:"filename"`
	Title           *string    `json:"title,omitempty"`
	Authors         *string    `json:"authors,omitempty"`
	PublicationDate *time.Time `json:"publication_date,omitempty"`
	PageCount       int        `json:"page_count"`
	FileSizeBytes   int64      `json:"file_size_bytes"`
	IndexedAt       *time.Time `json:"indexed_at,omitempty"`
}

// DocumentDetail represents full document metadata and counts.
type DocumentDetail struct {
	ID              int64      `json:"id"`
	Filename        string     `json:"filename"`
	Title           *string    `json:"title,omitempty"`
	Authors         *string    `json:"authors,omitempty"`
	PublicationDate *time.Time `json:"publication_date,omitempty"`
	DOI             *string    `json:"doi,omitempty"`
	MarkdownPath    string     `json:"markdown_path"`
	PageCount       int        `json:"page_count"`
	FileSizeBytes   int64      `json:"file_size_bytes"`
	IndexedAt       *time.Time `json:"indexed_at,omitempty"`
	ChunkCount      int64      `json:"chunk_count"`
	EmbeddingCount  int64      `json:"embedding_count"`
}

// DeleteResult captures deleted document details.
type DeleteResult struct {
	DocumentID     int64  `json:"document_id"`
	Filename       string `json:"filename"`
	ChunkCount     int64  `json:"chunk_count"`
	EmbeddingCount int64  `json:"embedding_count"`
}

// DatabaseStats captures summary counts for the database.
type DatabaseStats struct {
	DocumentCount  int64      `json:"document_count"`
	ChunkCount     int64      `json:"chunk_count"`
	EmbeddingCount int64      `json:"embedding_count"`
	TotalBytes     int64      `json:"total_bytes"`
	LastIndexedAt  *time.Time `json:"last_indexed_at,omitempty"`
}

// ListDocuments returns all documents sorted by most recent index time.
func ListDocuments(ctx context.Context, db *sql.DB) ([]DocumentSummary, error) {
	if db == nil {
		return nil, errors.New("database is nil")
	}
	if ctx == nil {
		ctx = context.Background()
	}
	rows, err := db.QueryContext(ctx, `SELECT id, filename, title, authors, publication_date, page_count, file_size_bytes, indexed_at
		FROM documents
		ORDER BY indexed_at DESC, filename ASC`)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var docs []DocumentSummary
	for rows.Next() {
		var (
			id        int64
			filename  string
			title     sql.NullString
			authors   sql.NullString
			pubDate   sql.NullTime
			pageCount sql.NullInt64
			fileSize  sql.NullInt64
			indexedAt sql.NullTime
		)
		if err := rows.Scan(&id, &filename, &title, &authors, &pubDate, &pageCount, &fileSize, &indexedAt); err != nil {
			return nil, err
		}
		docs = append(docs, DocumentSummary{
			ID:              id,
			Filename:        filename,
			Title:           nullableString(title),
			Authors:         nullableString(authors),
			PublicationDate: nullableTime(pubDate),
			PageCount:       intFromNull(pageCount),
			FileSizeBytes:   int64FromNull(fileSize),
			IndexedAt:       nullableTime(indexedAt),
		})
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	return docs, nil
}

// GetDocumentDetail returns metadata for a specific filename.
func GetDocumentDetail(ctx context.Context, db *sql.DB, filename string) (DocumentDetail, error) {
	if db == nil {
		return DocumentDetail{}, errors.New("database is nil")
	}
	if ctx == nil {
		ctx = context.Background()
	}
	var (
		id        int64
		title     sql.NullString
		authors   sql.NullString
		pubDate   sql.NullTime
		doi       sql.NullString
		mdPath    string
		pageCount sql.NullInt64
		fileSize  sql.NullInt64
		indexedAt sql.NullTime
	)
	row := db.QueryRowContext(ctx, `SELECT id, title, authors, publication_date, doi, markdown_path, page_count, file_size_bytes, indexed_at
		FROM documents
		WHERE filename = ?`, filename)
	if err := row.Scan(&id, &title, &authors, &pubDate, &doi, &mdPath, &pageCount, &fileSize, &indexedAt); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return DocumentDetail{}, ErrDocumentNotFound
		}
		return DocumentDetail{}, err
	}

	chunkCount, err := countChunks(ctx, db, id)
	if err != nil {
		return DocumentDetail{}, err
	}
	embeddingCount, err := countEmbeddings(ctx, db, id)
	if err != nil {
		return DocumentDetail{}, err
	}

	return DocumentDetail{
		ID:              id,
		Filename:        filename,
		Title:           nullableString(title),
		Authors:         nullableString(authors),
		PublicationDate: nullableTime(pubDate),
		DOI:             nullableString(doi),
		MarkdownPath:    mdPath,
		PageCount:       intFromNull(pageCount),
		FileSizeBytes:   int64FromNull(fileSize),
		IndexedAt:       nullableTime(indexedAt),
		ChunkCount:      chunkCount,
		EmbeddingCount:  embeddingCount,
	}, nil
}

// DeleteDocument removes a document and its related chunks/embeddings.
func DeleteDocument(ctx context.Context, db *sql.DB, filename string) (DeleteResult, error) {
	if db == nil {
		return DeleteResult{}, errors.New("database is nil")
	}
	if ctx == nil {
		ctx = context.Background()
	}
	var id int64
	row := db.QueryRowContext(ctx, `SELECT id FROM documents WHERE filename = ?`, filename)
	if err := row.Scan(&id); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return DeleteResult{}, ErrDocumentNotFound
		}
		return DeleteResult{}, err
	}

	chunkCount, err := countChunks(ctx, db, id)
	if err != nil {
		return DeleteResult{}, err
	}
	embeddingCount, err := countEmbeddings(ctx, db, id)
	if err != nil {
		return DeleteResult{}, err
	}

	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		return DeleteResult{}, err
	}

	if _, err := tx.ExecContext(ctx, `DELETE FROM document_embeddings WHERE document_id = ?;`, id); err != nil {
		return DeleteResult{}, rollback(tx, err)
	}
	if _, err := tx.ExecContext(ctx, `DELETE FROM embeddings WHERE chunk_id IN (SELECT id FROM chunks WHERE document_id = ?);`, id); err != nil {
		return DeleteResult{}, rollback(tx, err)
	}
	if _, err := tx.ExecContext(ctx, `DELETE FROM chunks WHERE document_id = ?;`, id); err != nil {
		return DeleteResult{}, rollback(tx, err)
	}
	res, err := tx.ExecContext(ctx, `DELETE FROM documents WHERE id = ?;`, id)
	if err != nil {
		return DeleteResult{}, rollback(tx, err)
	}
	rows, err := res.RowsAffected()
	if err != nil {
		return DeleteResult{}, rollback(tx, err)
	}
	if rows == 0 {
		return DeleteResult{}, rollback(tx, ErrDocumentNotFound)
	}
	if err := tx.Commit(); err != nil {
		return DeleteResult{}, err
	}

	return DeleteResult{
		DocumentID:     id,
		Filename:       filename,
		ChunkCount:     chunkCount,
		EmbeddingCount: embeddingCount,
	}, nil
}

// GetDatabaseStats returns aggregate counts for the database.
func GetDatabaseStats(ctx context.Context, db *sql.DB) (DatabaseStats, error) {
	if db == nil {
		return DatabaseStats{}, errors.New("database is nil")
	}
	if ctx == nil {
		ctx = context.Background()
	}
	var (
		count      int64
		totalBytes sql.NullInt64
		lastIndex  sql.NullTime
	)
	row := db.QueryRowContext(ctx, `SELECT COUNT(*), SUM(file_size_bytes), MAX(indexed_at) FROM documents`)
	if err := row.Scan(&count, &totalBytes, &lastIndex); err != nil {
		return DatabaseStats{}, err
	}
	chunkCount, err := countAll(ctx, db, "chunks")
	if err != nil {
		return DatabaseStats{}, err
	}
	embeddingCount, err := countAll(ctx, db, "embeddings")
	if err != nil {
		return DatabaseStats{}, err
	}

	return DatabaseStats{
		DocumentCount:  count,
		ChunkCount:     chunkCount,
		EmbeddingCount: embeddingCount,
		TotalBytes:     int64FromNull(totalBytes),
		LastIndexedAt:  nullableTime(lastIndex),
	}, nil
}

func countChunks(ctx context.Context, db *sql.DB, documentID int64) (int64, error) {
	return countWithArg(ctx, db, `SELECT COUNT(*) FROM chunks WHERE document_id = ?`, documentID)
}

func countEmbeddings(ctx context.Context, db *sql.DB, documentID int64) (int64, error) {
	return countWithArg(ctx, db, `SELECT COUNT(*) FROM embeddings e JOIN chunks c ON e.chunk_id = c.id WHERE c.document_id = ?`, documentID)
}

func countAll(ctx context.Context, db *sql.DB, table string) (int64, error) {
	query := "SELECT COUNT(*) FROM " + table
	return countWithArg(ctx, db, query)
}

func countWithArg(ctx context.Context, db *sql.DB, query string, args ...any) (int64, error) {
	row := db.QueryRowContext(ctx, query, args...)
	var count int64
	if err := row.Scan(&count); err != nil {
		return 0, err
	}
	return count, nil
}

func nullableString(value sql.NullString) *string {
	if !value.Valid {
		return nil
	}
	copy := value.String
	return &copy
}

func nullableTime(value sql.NullTime) *time.Time {
	if !value.Valid {
		return nil
	}
	copy := value.Time
	return &copy
}

func intFromNull(value sql.NullInt64) int {
	if !value.Valid {
		return 0
	}
	return int(value.Int64)
}

func int64FromNull(value sql.NullInt64) int64 {
	if !value.Valid {
		return 0
	}
	return value.Int64
}
