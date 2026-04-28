package cmd

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"path/filepath"
	"time"

	"pdfrag/discover"
	"pdfrag/embeddings"
	"pdfrag/indexing"
	"pdfrag/logging"
	"pdfrag/storage"

	"github.com/spf13/cobra"
	"go.uber.org/zap"
)

func newReindexCmd() *cobra.Command {
	var progressEvery int
	var progressThreshold int
	var listFiles bool

	cmd := &cobra.Command{
		Use:          "reindex <dir>",
		Short:        "Reindex changed PDFs and Markdown files in a directory",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			db, err := storage.InitDuckDB(cmd.Context(), appConfig.Database.Path)
			if err != nil {
				return err
			}
			defer func() {
				if closeErr := db.Close(); closeErr != nil {
					logging.L().Warn("failed to close database",
						zap.Error(closeErr),
					)
				}
			}()

			progress := discover.NewProgressPrinter(cmd.ErrOrStderr(), progressThreshold, 250*time.Millisecond)
			documents, stats, err := discover.FindDocuments(cmd.Context(), args[0], discover.Options{
				Progress:      progress,
				ProgressEvery: progressEvery,
			})
			if err != nil {
				return err
			}
			logging.L().Info("document discovery completed",
				zap.Int("visited", stats.Visited),
				zap.Int("pdfs", stats.PDFs),
				zap.Int("markdown", stats.Markdown),
				zap.Int("skipped", stats.Skipped),
				zap.Int("errors", stats.Errors),
				zap.Duration("elapsed", stats.Elapsed),
			)
			if len(documents) == 0 {
				fmt.Fprintln(cmd.OutOrStdout(), "No documents found.")
				return nil
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Found %d document(s) (%d PDFs, %d Markdown).\n", len(documents), stats.PDFs, stats.Markdown)
			if listFiles {
				for _, doc := range documents {
					fmt.Fprintln(cmd.OutOrStdout(), doc.Path)
				}
			}

			existing, err := fetchExistingSignatures(cmd.Context(), db)
			if err != nil {
				return err
			}
			if err := ensureOllama(cmd.Context(), appConfig.Embeddings.OllamaHost, cmd.ErrOrStderr()); err != nil {
				return err
			}
			if appConfig.Ollama.Warm {
				if err := warmOllamaEmbeddings(cmd.Context(), appConfig.Embeddings.OllamaHost, appConfig.Embeddings.Model); err != nil {
					logging.L().Warn("ollama embeddings warmup failed", zap.Error(err))
				}
			}
			embedClient := embeddings.NewClient(embeddings.Options{
				Host:  appConfig.Embeddings.OllamaHost,
				Model: appConfig.Embeddings.Model,
			})

			handled := 0
			reindexed := 0
			added := 0
			skipped := 0
			totalChunks := 0
			totalDocs := len(documents)
			fmt.Fprintf(cmd.OutOrStdout(), "Processing %d document(s)...\n", totalDocs)
			for _, doc := range documents {
				filename := filepath.Base(doc.Path)
				signature, err := indexing.ComputeFileSignature(doc.Path)
				if err != nil {
					return err
				}
				entry, ok := existing[filename]
				if ok && signaturesMatch(entry, signature) {
					skipped++
					handled++
					fmt.Fprintf(cmd.OutOrStdout(), "Processed %d/%d documents (%d chunks, %d reindexed, %d new, %d skipped)\n", handled, totalDocs, totalChunks, reindexed, added, skipped)
					continue
				}

				if ok {
					reindexed++
					if _, err := storage.DeleteDocument(cmd.Context(), db, filename); err != nil && !errors.Is(err, storage.ErrDocumentNotFound) {
						return err
					}
				} else {
					added++
				}

				var result indexing.IndexResult
				switch doc.Kind {
				case discover.DocumentMarkdown:
					result, err = indexing.IndexMarkdownWithSignature(cmd.Context(), db, embedClient, doc.Path, signature, indexing.IndexOptions{
						EmbeddingBatchSize: appConfig.Embeddings.BatchSize,
					})
				default:
					result, err = indexing.IndexPDFWithSignature(cmd.Context(), db, embedClient, doc.Path, signature, indexing.IndexOptions{
						EmbeddingBatchSize: appConfig.Embeddings.BatchSize,
					})
				}
				if err != nil {
					return err
				}
				totalChunks += result.ChunkCount
				handled++
				fmt.Fprintf(cmd.OutOrStdout(), "Processed %d/%d documents (%d chunks, %d reindexed, %d new, %d skipped)\n", handled, totalDocs, totalChunks, reindexed, added, skipped)
			}
			return nil
		},
	}

	cmd.Flags().IntVar(&progressEvery, "progress-every", 200, "emit progress update every N entries")
	cmd.Flags().IntVar(&progressThreshold, "progress-threshold", 500, "minimum entries before showing progress")
	cmd.Flags().BoolVar(&listFiles, "list", false, "list discovered documents")

	return cmd
}

type storedSignature struct {
	ID      int64
	ModTime *time.Time
	Hash    string
}

func fetchExistingSignatures(ctx context.Context, db *sql.DB) (map[string]storedSignature, error) {
	if ctx == nil {
		ctx = context.Background()
	}
	rows, err := db.QueryContext(ctx, "SELECT id, filename, file_mtime, file_hash FROM documents")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	existing := make(map[string]storedSignature)
	for rows.Next() {
		var (
			id       int64
			filename string
			mtime    sql.NullTime
			hash     sql.NullString
		)
		if err := rows.Scan(&id, &filename, &mtime, &hash); err != nil {
			return nil, err
		}
		var mtimePtr *time.Time
		if mtime.Valid {
			copy := mtime.Time
			mtimePtr = &copy
		}
		existing[filename] = storedSignature{
			ID:      id,
			ModTime: mtimePtr,
			Hash:    hash.String,
		}
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	return existing, nil
}

func signaturesMatch(stored storedSignature, current indexing.FileSignature) bool {
	if stored.ModTime == nil || stored.Hash == "" {
		return false
	}
	if stored.Hash != current.Hash {
		return false
	}
	return stored.ModTime.Equal(current.ModTime)
}
