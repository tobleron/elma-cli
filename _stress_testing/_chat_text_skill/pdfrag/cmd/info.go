package cmd

import (
	"errors"
	"fmt"
	"io"
	"strings"
	"text/tabwriter"

	"pdfrag/storage"

	"github.com/spf13/cobra"
)

func newInfoCmd() *cobra.Command {
	var format string

	cmd := &cobra.Command{
		Use:          "info <filename>",
		Short:        "Show document details",
		Args:         cobra.ExactArgs(1),
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

			detail, err := storage.GetDocumentDetail(cmd.Context(), db, args[0])
			if err != nil {
				if errors.Is(err, storage.ErrDocumentNotFound) {
					return fmt.Errorf("document %q not found", args[0])
				}
				return err
			}

			if format == "json" {
				return writeJSON(cmd.OutOrStdout(), detail)
			}
			return writeInfoTable(cmd.OutOrStdout(), detail)
		},
	}

	cmd.Flags().StringVar(&format, "format", "table", "output format: table or json")

	return cmd
}

func writeInfoTable(w io.Writer, detail storage.DocumentDetail) error {
	writer := tabwriter.NewWriter(w, 0, 4, 2, ' ', 0)
	if err := writeInfoRow(writer, "Filename", detail.Filename); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Title", formatOptionalString(detail.Title)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Authors", formatOptionalString(detail.Authors)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Publication Date", formatDate(detail.PublicationDate)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "DOI", formatOptionalString(detail.DOI)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Markdown Path", detail.MarkdownPath); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Pages", fmt.Sprintf("%d", detail.PageCount)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "File Size", formatBytes(detail.FileSizeBytes)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Indexed At", formatTimestamp(detail.IndexedAt)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Chunks", fmt.Sprintf("%d", detail.ChunkCount)); err != nil {
		return err
	}
	if err := writeInfoRow(writer, "Embeddings", fmt.Sprintf("%d", detail.EmbeddingCount)); err != nil {
		return err
	}
	return writer.Flush()
}

func writeInfoRow(w io.Writer, label, value string) error {
	_, err := fmt.Fprintf(w, "%s\t%s\n", label, value)
	return err
}
