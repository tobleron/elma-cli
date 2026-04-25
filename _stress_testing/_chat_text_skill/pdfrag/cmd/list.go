package cmd

import (
	"fmt"
	"io"
	"strings"
	"text/tabwriter"

	"pdfrag/storage"

	"github.com/spf13/cobra"
)

func newListCmd() *cobra.Command {
	var format string

	cmd := &cobra.Command{
		Use:          "list",
		Short:        "List indexed documents",
		Args:         cobra.NoArgs,
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			format = strings.ToLower(strings.TrimSpace(format))
			if format == "" {
				format = "table"
			}
			if format != "table" && format != "json" {
				return fmt.Errorf("unknown format %q (use table or json)", format)
			}

			db, err := storage.InitDuckDB(cmd.Context(), appConfig.Database.Path)
			if err != nil {
				return err
			}
			defer func() {
				_ = db.Close()
			}()

			docs, err := storage.ListDocuments(cmd.Context(), db)
			if err != nil {
				return err
			}
			if format == "json" {
				return writeJSON(cmd.OutOrStdout(), docs)
			}
			if len(docs) == 0 {
				_, err := fmt.Fprintln(cmd.OutOrStdout(), "No documents found.")
				return err
			}
			return writeDocumentTable(cmd.OutOrStdout(), docs)
		},
	}

	cmd.Flags().StringVar(&format, "format", "table", "output format: table or json")

	return cmd
}

func writeDocumentTable(w io.Writer, docs []storage.DocumentSummary) error {
	writer := tabwriter.NewWriter(w, 0, 4, 2, ' ', 0)
	if _, err := fmt.Fprintln(writer, "ID\tFilename\tTitle\tPages\tSize\tIndexed At"); err != nil {
		return err
	}
	for _, doc := range docs {
		if _, err := fmt.Fprintf(writer, "%d\t%s\t%s\t%d\t%s\t%s\n",
			doc.ID,
			doc.Filename,
			formatOptionalString(doc.Title),
			doc.PageCount,
			formatBytes(doc.FileSizeBytes),
			formatTimestamp(doc.IndexedAt),
		); err != nil {
			return err
		}
	}
	return writer.Flush()
}
