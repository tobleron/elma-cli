package cmd

import (
	"errors"
	"fmt"

	"pdfrag/storage"

	"github.com/spf13/cobra"
)

func newDeleteCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:          "delete <filename>",
		Short:        "Delete a document from the index",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			db, err := storage.InitDuckDB(cmd.Context(), appConfig.Database.Path)
			if err != nil {
				return err
			}
			defer func() {
				_ = db.Close()
			}()

			result, err := storage.DeleteDocument(cmd.Context(), db, args[0])
			if err != nil {
				if errors.Is(err, storage.ErrDocumentNotFound) {
					return fmt.Errorf("document %q not found", args[0])
				}
				return err
			}
			_, err = fmt.Fprintf(cmd.OutOrStdout(), "Deleted %s (chunks: %d, embeddings: %d)\n", result.Filename, result.ChunkCount, result.EmbeddingCount)
			return err
		},
	}

	return cmd
}
