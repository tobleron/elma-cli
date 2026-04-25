package cmd

import (
	"database/sql"
	"fmt"
)

func rollback(tx *sql.Tx, err error) error {
	if rbErr := tx.Rollback(); rbErr != nil && rbErr != sql.ErrTxDone {
		return fmt.Errorf("%w: %v", err, rbErr)
	}
	return err
}
