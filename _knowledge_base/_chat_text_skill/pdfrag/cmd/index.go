package cmd

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"os"
	"os/signal"
	"path/filepath"
	"syscall"
	"time"

	"pdfrag/discover"
	"pdfrag/embeddings"
	"pdfrag/indexing"
	"pdfrag/logging"
	"pdfrag/storage"

	"github.com/spf13/cobra"
	"go.uber.org/zap"
)

func newIndexCmd() *cobra.Command {
	var progressEvery int
	var progressThreshold int
	var listFiles bool
	var checkpointPath string
	var resume bool

	cmd := &cobra.Command{
		Use:          "index <dir>",
		Short:        "Index PDFs and Markdown files in a directory",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			ctx, stop := signal.NotifyContext(cmd.Context(), os.Interrupt, syscall.SIGTERM)
			defer stop()

			db, err := storage.InitDuckDB(ctx, appConfig.Database.Path)
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
			documents, stats, err := discover.FindDocuments(ctx, args[0], discover.Options{
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

			rootDir, err := filepath.Abs(args[0])
			if err != nil {
				return err
			}

			existing, err := fetchExistingFilenames(ctx, db)
			if err != nil {
				return err
			}
			documentPaths := make([]string, len(documents))
			for i, doc := range documents {
				documentPaths[i] = doc.Path
			}
			checkpoint, err := prepareIndexCheckpoint(checkpointPath, resume, rootDir, appConfig.Database.Path, documentPaths)
			if err != nil {
				return err
			}
			if checkpoint != nil {
				if err := saveIndexCheckpoint(checkpointPath, checkpoint); err != nil {
					return err
				}
			}

			if err := ensureOllama(ctx, appConfig.Embeddings.OllamaHost, cmd.ErrOrStderr()); err != nil {
				return err
			}
			if appConfig.Ollama.Warm {
				if err := warmOllamaEmbeddings(ctx, appConfig.Embeddings.OllamaHost, appConfig.Embeddings.Model); err != nil {
					logging.L().Warn("ollama embeddings warmup failed", zap.Error(err))
				}
			}
			embedClient := embeddings.NewClient(embeddings.Options{
				Host:  appConfig.Embeddings.OllamaHost,
				Model: appConfig.Embeddings.Model,
			})
			seen := make(map[string]struct{})
			processed := 0
			skipped := 0
			totalChunks := 0
			totalDocs := len(documents)
			if checkpoint != nil {
				processed = checkpoint.ProcessedCount()
				totalChunks = checkpoint.TotalChunks
				for filename := range checkpoint.Processed {
					seen[filename] = struct{}{}
				}
				if processed > 0 {
					fmt.Fprintf(cmd.OutOrStdout(), "Resuming from checkpoint: %d document(s) already indexed (%d chunks).\n", processed, totalChunks)
				}
				if processed >= totalDocs && totalDocs > 0 {
					fmt.Fprintln(cmd.OutOrStdout(), "All documents already indexed from checkpoint.")
					if err := clearIndexCheckpoint(checkpointPath); err != nil {
						logging.L().Warn("failed to clear checkpoint", zap.Error(err))
					}
					return nil
				}
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Processing %d document(s)...\n", totalDocs)

			type indexResult struct {
				doc    discover.Document
				chunks int
				err    error
				skip   bool
			}

			workerCount := appConfig.Indexing.Concurrency
			if workerCount <= 0 {
				workerCount = 4
			}
			embedLimiter := make(chan struct{}, 1)
			jobs := make(chan discover.Document)

			queue := make([]discover.Document, 0, len(documents))
			for _, doc := range documents {
				if err := ctx.Err(); err != nil {
					if checkpoint != nil {
						if saveErr := saveIndexCheckpoint(checkpointPath, checkpoint); saveErr != nil {
							logging.L().Warn("failed to save checkpoint", zap.Error(saveErr))
						}
					}
					fmt.Fprintln(cmd.ErrOrStderr(), "Indexing interrupted; checkpoint saved.")
					return err
				}
				filename := filepath.Base(doc.Path)
				if checkpoint != nil {
					if _, ok := checkpoint.Processed[filename]; ok {
						skipped++
						continue
					}
				}
				if _, ok := existing[filename]; ok {
					skipped++
					logging.L().Info("skipping duplicate document", zap.String("filename", filename))
					continue
				}
				if _, ok := seen[filename]; ok {
					skipped++
					logging.L().Info("skipping duplicate document in run", zap.String("filename", filename))
					continue
				}
				seen[filename] = struct{}{}
				queue = append(queue, doc)
			}
			queued := len(queue)
			if queued == 0 {
				if checkpoint != nil {
					if err := clearIndexCheckpoint(checkpointPath); err != nil {
						logging.L().Warn("failed to clear checkpoint", zap.Error(err))
					}
				}
				fmt.Fprintln(cmd.OutOrStdout(), "All documents already indexed.")
				return nil
			}

			results := make(chan indexResult, workerCount)

			worker := func() {
				for doc := range jobs {
					if err := ctx.Err(); err != nil {
						results <- indexResult{doc: doc, err: err}
						continue
					}
					var result indexing.IndexResult
					var err error
					switch doc.Kind {
					case discover.DocumentMarkdown:
						result, err = indexing.IndexMarkdown(ctx, db, embedClient, doc.Path, indexing.IndexOptions{
							EmbeddingBatchSize: appConfig.Embeddings.BatchSize,
							EmbedLimiter:       embedLimiter,
						})
					default:
						result, err = indexing.IndexPDF(ctx, db, embedClient, doc.Path, indexing.IndexOptions{
							EmbeddingBatchSize: appConfig.Embeddings.BatchSize,
							EmbedLimiter:       embedLimiter,
						})
					}
					results <- indexResult{doc: doc, chunks: result.ChunkCount, err: err}
				}
			}

			for i := 0; i < workerCount; i++ {
				go worker()
			}

			go func() {
				for _, doc := range queue {
					jobs <- doc
				}
				close(jobs)
			}()

			for i := 0; i < queued; i++ {
				res := <-results
				if res.err != nil {
					if checkpoint != nil {
						if saveErr := saveIndexCheckpoint(checkpointPath, checkpoint); saveErr != nil {
							logging.L().Warn("failed to save checkpoint", zap.Error(saveErr))
						}
					}
					if errors.Is(res.err, context.Canceled) || errors.Is(res.err, context.DeadlineExceeded) {
						fmt.Fprintln(cmd.ErrOrStderr(), "Indexing interrupted; checkpoint saved.")
					}
					return res.err
				}
				filename := filepath.Base(res.doc.Path)
				if checkpoint != nil {
					checkpoint.MarkProcessed(filename, res.doc.Path, res.chunks)
					if err := saveIndexCheckpoint(checkpointPath, checkpoint); err != nil {
						return err
					}
				}
				processed++
				totalChunks += res.chunks
				fmt.Fprintf(cmd.OutOrStdout(), "Indexed %d/%d documents (%d chunks, %d skipped)\n", processed, totalDocs, totalChunks, skipped)
			}
			if checkpoint != nil {
				if err := clearIndexCheckpoint(checkpointPath); err != nil {
					logging.L().Warn("failed to clear checkpoint", zap.Error(err))
				}
			}
			return nil
		},
	}

	cmd.Flags().IntVar(&progressEvery, "progress-every", 200, "emit progress update every N entries")
	cmd.Flags().IntVar(&progressThreshold, "progress-threshold", 500, "minimum entries before showing progress")
	cmd.Flags().BoolVar(&listFiles, "list", false, "list discovered documents")
	cmd.Flags().StringVar(&checkpointPath, "checkpoint", defaultIndexCheckpointPath(), "checkpoint file path")
	cmd.Flags().BoolVar(&resume, "resume", true, "resume from existing checkpoint when available")

	return cmd
}

func fetchExistingFilenames(ctx context.Context, db *sql.DB) (map[string]struct{}, error) {
	rows, err := db.QueryContext(ctx, "SELECT filename FROM documents")
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	existing := make(map[string]struct{})
	for rows.Next() {
		var filename string
		if err := rows.Scan(&filename); err != nil {
			return nil, err
		}
		existing[filename] = struct{}{}
	}
	if err := rows.Err(); err != nil {
		return nil, err
	}
	return existing, nil
}
