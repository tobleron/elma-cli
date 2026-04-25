package storage

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	_ "github.com/marcboeker/go-duckdb"
)

// DefaultDuckDBPath is the default location for the DuckDB file.
const DefaultDuckDBPath = "./data/pdfrag.db"

// InitDuckDB opens the DuckDB file and ensures the schema exists.
func InitDuckDB(ctx context.Context, path string) (*sql.DB, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	if path == "" {
		path = DefaultDuckDBPath
	}
	if err := ensureDBDir(path); err != nil {
		return nil, err
	}

	db, err := sql.Open("duckdb", path)
	if err != nil {
		return nil, err
	}
	if err := db.PingContext(ctx); err != nil {
		_ = db.Close()
		return nil, err
	}
	if err := ensureSchema(ctx, db); err != nil {
		_ = db.Close()
		return nil, err
	}
	return db, nil
}

func ensureDBDir(path string) error {
	dir := filepath.Dir(path)
	if dir == "." || dir == "" {
		return nil
	}
	return os.MkdirAll(dir, 0o755)
}

func ensureSchema(ctx context.Context, db *sql.DB) error {
	return ensureSchemaWithRetry(ctx, db, false)
}

func ensureSchemaWithRetry(ctx context.Context, db *sql.DB, triedReset bool) error {
	statements := []string{
		`CREATE SEQUENCE IF NOT EXISTS documents_id_seq START 1;`,
		`CREATE SEQUENCE IF NOT EXISTS chunks_id_seq START 1;`,
		`CREATE TABLE IF NOT EXISTS documents (
			id BIGINT PRIMARY KEY DEFAULT nextval('documents_id_seq'),
			filename TEXT NOT NULL,
			title TEXT,
			authors TEXT,
			publication_date DATE,
			doi TEXT,
			markdown_path TEXT NOT NULL,
			page_count INTEGER,
			file_size_bytes INTEGER,
			file_mtime TIMESTAMP,
			file_hash TEXT,
			indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			UNIQUE(filename)
		);`,
		`CREATE TABLE IF NOT EXISTS chunks (
			id BIGINT PRIMARY KEY DEFAULT nextval('chunks_id_seq'),
			document_id BIGINT NOT NULL,
			chunk_index INTEGER NOT NULL,
			content TEXT NOT NULL,
			page_number INTEGER,
			section_title TEXT,
			token_count INTEGER,
			FOREIGN KEY (document_id) REFERENCES documents(id)
		);`,
		`CREATE TABLE IF NOT EXISTS embeddings (
			chunk_id BIGINT PRIMARY KEY,
			embedding BLOB,
			FOREIGN KEY (chunk_id) REFERENCES chunks(id)
		);`,
		`CREATE TABLE IF NOT EXISTS document_embeddings (
			document_id BIGINT PRIMARY KEY,
			embedding BLOB,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			FOREIGN KEY (document_id) REFERENCES documents(id)
		);`,
	}

	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	for _, stmt := range statements {
		if _, err := tx.ExecContext(ctx, stmt); err != nil {
			return rollback(tx, err)
		}
	}
	if err := tx.Commit(); err != nil {
		return err
	}

	reset, err := maybeResetSchema(ctx, db)
	if err != nil {
		return err
	}
	if reset {
		if triedReset {
			return errors.New("schema reset did not resolve migration issues")
		}
		return ensureSchemaWithRetry(ctx, db, true)
	}

	if err := ensureEmbeddingTables(ctx, db); err != nil {
		return err
	}

	if err := createIndex(ctx, db, "idx_document_filename", "documents", "filename"); err != nil {
		return err
	}
	if err := createIndex(ctx, db, "idx_document_doi", "documents", "doi"); err != nil {
		return err
	}
	if err := createIndex(ctx, db, "idx_chunk_document", "chunks", "document_id"); err != nil {
		return err
	}
	if err := createIndex(ctx, db, "idx_document_embeddings_document", "document_embeddings", "document_id"); err != nil {
		return err
	}
	if err := addColumn(ctx, db, "documents", "file_mtime TIMESTAMP"); err != nil {
		return err
	}
	if err := addColumn(ctx, db, "documents", "file_hash TEXT"); err != nil {
		return err
	}
	return nil
}

func createIndex(ctx context.Context, db *sql.DB, name, table, column string) error {
	withIf := fmt.Sprintf("CREATE INDEX IF NOT EXISTS %s ON %s(%s);", name, table, column)
	fallback := fmt.Sprintf("CREATE INDEX %s ON %s(%s);", name, table, column)
	return execWithFallback(ctx, db, withIf, fallback, shouldRetryIndex)
}

func createOptionalIndex(ctx context.Context, db *sql.DB, name, table, column string) error {
	if err := createIndex(ctx, db, name, table, column); err != nil {
		if isUnsupportedIndexError(err) {
			return nil
		}
		return err
	}
	return nil
}

func shouldRetryIndex(err error) bool {
	msg := strings.ToLower(err.Error())
	return strings.Contains(msg, "parser") || strings.Contains(msg, "syntax") || strings.Contains(msg, "if not exists")
}

func isUnsupportedIndexError(err error) bool {
	msg := strings.ToLower(err.Error())
	return strings.Contains(msg, "not supported") ||
		strings.Contains(msg, "unsupported") ||
		strings.Contains(msg, "cannot create index") ||
		strings.Contains(msg, "index on") ||
		strings.Contains(msg, "index key")
}

func addColumn(ctx context.Context, db *sql.DB, table, columnDef string) error {
	columnName := strings.Fields(columnDef)
	if len(columnName) == 0 {
		return fmt.Errorf("invalid column definition: %q", columnDef)
	}
	if _, ok, err := columnType(ctx, db, table, columnName[0]); err != nil {
		return err
	} else if ok {
		return nil
	}
	withIf := fmt.Sprintf("ALTER TABLE %s ADD COLUMN IF NOT EXISTS %s;", table, columnDef)
	fallback := fmt.Sprintf("ALTER TABLE %s ADD COLUMN %s;", table, columnDef)
	return execWithFallback(ctx, db, withIf, fallback, shouldRetryIndex)
}

func execWithFallback(ctx context.Context, db *sql.DB, primary, fallback string, shouldRetry func(error) bool) error {
	if err := execInTx(ctx, db, primary); err == nil {
		return nil
	} else if isAlreadyExistsError(err) {
		return nil
	} else if !shouldRetry(err) {
		return err
	}

	if err := execInTx(ctx, db, fallback); err != nil {
		if isAlreadyExistsError(err) {
			return nil
		}
		return err
	}
	return nil
}

func execInTx(ctx context.Context, db *sql.DB, stmt string) error {
	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		return err
	}
	if _, err := tx.ExecContext(ctx, stmt); err != nil {
		_ = tx.Rollback()
		return err
	}
	return tx.Commit()
}

func maybeResetSchema(ctx context.Context, db *sql.DB) (bool, error) {
	missing, err := documentsMissingColumns(ctx, db)
	if err != nil {
		return false, err
	}
	if !missing {
		return false, nil
	}
	count, err := countDocuments(ctx, db)
	if err != nil {
		return false, err
	}
	if count > 0 {
		return false, fmt.Errorf("documents schema requires migration; %d documents indexed. export and reindex with a fresh database", count)
	}

	stmts := []string{
		"DROP TABLE IF EXISTS embeddings;",
		"DROP TABLE IF EXISTS chunks;",
		"DROP TABLE IF EXISTS document_embeddings;",
		"DROP TABLE IF EXISTS documents;",
		"DROP SEQUENCE IF EXISTS documents_id_seq;",
		"DROP SEQUENCE IF EXISTS chunks_id_seq;",
	}
	for _, stmt := range stmts {
		if err := execInTx(ctx, db, stmt); err != nil {
			return false, err
		}
	}
	return true, nil
}

func documentsMissingColumns(ctx context.Context, db *sql.DB) (bool, error) {
	required := []string{"file_mtime", "file_hash"}
	for _, column := range required {
		if _, ok, err := columnType(ctx, db, "documents", column); err != nil {
			return false, err
		} else if !ok {
			return true, nil
		}
	}
	return false, nil
}

func countDocuments(ctx context.Context, db *sql.DB) (int64, error) {
	row := db.QueryRowContext(ctx, "SELECT COUNT(*) FROM documents")
	var count int64
	if err := row.Scan(&count); err != nil {
		return 0, err
	}
	return count, nil
}

func ensureEmbeddingTables(ctx context.Context, db *sql.DB) error {
	embeddingType, ok, err := columnType(ctx, db, "embeddings", "embedding")
	if err != nil {
		return err
	}
	if !ok || !strings.EqualFold(embeddingType, "BLOB") {
		if err := execInTx(ctx, db, "DROP TABLE IF EXISTS embeddings;"); err != nil {
			return err
		}
		if err := execInTx(ctx, db, `CREATE TABLE IF NOT EXISTS embeddings (
			chunk_id BIGINT PRIMARY KEY,
			embedding BLOB,
			FOREIGN KEY (chunk_id) REFERENCES chunks(id)
		);`); err != nil {
			return err
		}
	}

	docEmbeddingType, ok, err := columnType(ctx, db, "document_embeddings", "embedding")
	if err != nil {
		return err
	}
	if !ok || !strings.EqualFold(docEmbeddingType, "BLOB") {
		if err := execInTx(ctx, db, "DROP TABLE IF EXISTS document_embeddings;"); err != nil {
			return err
		}
		if err := execInTx(ctx, db, `CREATE TABLE IF NOT EXISTS document_embeddings (
			document_id BIGINT PRIMARY KEY,
			embedding BLOB,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			FOREIGN KEY (document_id) REFERENCES documents(id)
		);`); err != nil {
			return err
		}
	}
	return nil
}

func columnType(ctx context.Context, db *sql.DB, table, column string) (string, bool, error) {
	stmt := fmt.Sprintf("PRAGMA table_info('%s');", table)
	rows, err := db.QueryContext(ctx, stmt)
	if err != nil {
		return "", false, err
	}
	defer rows.Close()
	for rows.Next() {
		var (
			cid      int
			name     string
			colType  string
			notNull  bool
			defaultV sql.NullString
			primary  bool
		)
		if err := rows.Scan(&cid, &name, &colType, &notNull, &defaultV, &primary); err != nil {
			return "", false, err
		}
		if strings.EqualFold(name, column) {
			return colType, true, nil
		}
	}
	if err := rows.Err(); err != nil {
		return "", false, err
	}
	return "", false, nil
}

func isAlreadyExistsError(err error) bool {
	msg := strings.ToLower(err.Error())
	return strings.Contains(msg, "already exists") || strings.Contains(msg, "exists")
}

func rollback(tx *sql.Tx, err error) error {
	rbErr := tx.Rollback()
	if rbErr == nil || errors.Is(rbErr, sql.ErrTxDone) {
		return err
	}
	return errors.Join(err, rbErr)
}
