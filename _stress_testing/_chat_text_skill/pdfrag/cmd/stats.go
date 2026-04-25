package cmd

import (
	"fmt"
	"io"
	"strings"

	"pdfrag/storage"

	"github.com/spf13/cobra"
)

func newStatsCmd() *cobra.Command {
	var format string

	cmd := &cobra.Command{
		Use:          "stats",
		Short:        "Show database statistics",
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

			stats, err := storage.GetDatabaseStats(cmd.Context(), db)
			if err != nil {
				return err
			}

			if format == "json" {
				return writeJSON(cmd.OutOrStdout(), stats)
			}

			return writeStatsTable(cmd.OutOrStdout(), stats)
		},
	}

	cmd.Flags().StringVar(&format, "format", "table", "output format: table or json")

	return cmd
}

func writeStatsTable(w io.Writer, stats storage.DatabaseStats) error {
	_, err := fmt.Fprintf(w, "Documents: %d\nChunks: %d\nEmbeddings: %d\nTotal Size: %s\nLast Indexed: %s\n",
		stats.DocumentCount,
		stats.ChunkCount,
		stats.EmbeddingCount,
		formatBytes(stats.TotalBytes),
		formatTimestamp(stats.LastIndexedAt),
	)
	return err
}
