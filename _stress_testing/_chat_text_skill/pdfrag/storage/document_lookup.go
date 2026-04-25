package storage

import (
	"context"
	"database/sql"
	"errors"
	"strings"
)

// LookupDocumentIDByDOI returns a document ID for a DOI, if present.
func LookupDocumentIDByDOI(ctx context.Context, db *sql.DB, doi string) (int64, bool, error) {
	if db == nil {
		return 0, false, errors.New("database is nil")
	}
	doi = strings.TrimSpace(doi)
	if doi == "" {
		return 0, false, nil
	}
	if ctx == nil {
		ctx = context.Background()
	}
	row := db.QueryRowContext(ctx, `SELECT id FROM documents WHERE doi = ? LIMIT 1;`, doi)
	var id int64
	if err := row.Scan(&id); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return 0, false, nil
		}
		return 0, false, err
	}
	return id, true, nil
}

// LookupDocumentIDByTitle returns a document ID for an exact title match (case-insensitive).
func LookupDocumentIDByTitle(ctx context.Context, db *sql.DB, title string) (int64, bool, error) {
	if db == nil {
		return 0, false, errors.New("database is nil")
	}
	title = strings.TrimSpace(title)
	if title == "" {
		return 0, false, nil
	}
	if ctx == nil {
		ctx = context.Background()
	}
	row := db.QueryRowContext(ctx, `SELECT id FROM documents WHERE title IS NOT NULL AND lower(title) = lower(?) LIMIT 1;`, title)
	var id int64
	if err := row.Scan(&id); err != nil {
		if errors.Is(err, sql.ErrNoRows) {
			return 0, false, nil
		}
		return 0, false, err
	}
	return id, true, nil
}
