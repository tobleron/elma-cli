package cmd

import (
	"errors"
	"fmt"
	"io"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
	"text/tabwriter"

	"pdfrag/storage"

	"github.com/spf13/cobra"
)

func newRelatedCmd() *cobra.Command {
	var (
		format        string
		topK          int
		minSimilarity float64
		openResult    bool
		openIndex     int
	)

	cmd := &cobra.Command{
		Use:          "related <pdf-file>",
		Short:        "Find documents related to a given PDF",
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
			resolvedTopK, resolvedMin := resolveSearchOptions(topK, minSimilarity)

			db, err := storage.InitDuckDB(cmd.Context(), appConfig.Database.Path)
			if err != nil {
				return err
			}
			defer func() {
				_ = db.Close()
			}()

			filename := filepath.Base(args[0])
			results, err := storage.FindRelatedDocuments(cmd.Context(), db, filename, storage.RelatedSearchOptions{
				TopK:          resolvedTopK,
				MinSimilarity: resolvedMin,
			})
			if err != nil {
				if errors.Is(err, storage.ErrDocumentNotFound) {
					return fmt.Errorf("document %q not found", filename)
				}
				return err
			}

			if format == "json" {
				if err := writeJSON(cmd.OutOrStdout(), buildRelatedOutput(results, resolvedTopK, resolvedMin)); err != nil {
					return err
				}
			} else {
				if len(results) == 0 {
					if _, err := fmt.Fprintln(cmd.OutOrStdout(), "No related documents found."); err != nil {
						return err
					}
				} else if err := writeRelatedTable(cmd.OutOrStdout(), results); err != nil {
					return err
				}
			}
			if openResult {
				if len(results) == 0 {
					return errors.New("no related documents to open")
				}
				return openRelatedResult(args[0], results, openIndex)
			}
			return nil
		},
	}

	cmd.Flags().StringVar(&format, "format", "table", "output format: table or json")
	cmd.Flags().IntVar(&topK, "top-k", 0, "maximum number of results to return")
	cmd.Flags().Float64Var(&minSimilarity, "min-similarity", -1, "minimum similarity threshold (0-1)")
	cmd.Flags().BoolVar(&openResult, "open", false, "open a related document after listing")
	cmd.Flags().IntVar(&openIndex, "open-index", 1, "1-based index of result to open")

	return cmd
}

type relatedResultOutput struct {
	Rank       int     `json:"rank"`
	Filename   string  `json:"filename"`
	Title      *string `json:"title,omitempty"`
	Authors    *string `json:"authors,omitempty"`
	Similarity float64 `json:"similarity"`
	Summary    string  `json:"summary,omitempty"`
}

type relatedOutput struct {
	Results []relatedResultOutput `json:"results"`
	Meta    relatedOutputMeta     `json:"meta"`
}

type relatedOutputMeta struct {
	TopK          int     `json:"top_k"`
	MinSimilarity float64 `json:"min_similarity"`
	Count         int     `json:"count"`
}

func buildRelatedOutput(results []storage.RelatedDocument, topK int, minSimilarity float64) relatedOutput {
	output := relatedOutput{
		Meta: relatedOutputMeta{
			TopK:          topK,
			MinSimilarity: minSimilarity,
			Count:         len(results),
		},
	}
	output.Results = buildRelatedResults(results)
	return output
}

func buildRelatedResults(results []storage.RelatedDocument) []relatedResultOutput {
	output := make([]relatedResultOutput, 0, len(results))
	for i, result := range results {
		summary := summarizeContent(result.Summary, 200)
		output = append(output, relatedResultOutput{
			Rank:       i + 1,
			Filename:   result.Filename,
			Title:      result.Title,
			Authors:    result.Authors,
			Similarity: result.Similarity,
			Summary:    summary,
		})
	}
	return output
}

func writeRelatedTable(w io.Writer, results []storage.RelatedDocument) error {
	writer := tabwriter.NewWriter(w, 0, 4, 2, ' ', 0)
	if _, err := fmt.Fprintln(writer, "Rank\tFilename\tTitle\tAuthors\tSimilarity\tSummary"); err != nil {
		return err
	}
	for i, result := range results {
		summary := summarizeContent(result.Summary, 120)
		if _, err := fmt.Fprintf(writer, "%d\t%s\t%s\t%s\t%.3f\t%s\n",
			i+1,
			result.Filename,
			formatOptionalString(result.Title),
			formatOptionalString(result.Authors),
			result.Similarity,
			summary,
		); err != nil {
			return err
		}
	}
	return writer.Flush()
}

func openRelatedResult(seedPath string, results []storage.RelatedDocument, index int) error {
	if len(results) == 0 {
		return errors.New("no related documents to open")
	}
	if index <= 0 || index > len(results) {
		return fmt.Errorf("open-index %d out of range (1-%d)", index, len(results))
	}
	result := results[index-1]
	path, err := resolveRelatedPath(seedPath, result)
	if err != nil {
		return err
	}
	return openFile(path)
}

func resolveRelatedPath(seedPath string, result storage.RelatedDocument) (string, error) {
	seedDir := filepath.Dir(seedPath)
	if seedDir != "." && seedDir != "" {
		candidate := filepath.Join(seedDir, result.Filename)
		if fileExists(candidate) {
			return candidate, nil
		}
	}
	if result.MarkdownPath != "" && fileExists(result.MarkdownPath) {
		return result.MarkdownPath, nil
	}
	return "", fmt.Errorf("unable to locate file for %q", result.Filename)
}

func fileExists(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return !info.IsDir()
}

func openFile(path string) error {
	var cmd *exec.Cmd
	switch runtime.GOOS {
	case "darwin":
		cmd = exec.Command("open", path)
	case "windows":
		cmd = exec.Command("cmd", "/c", "start", "", path)
	default:
		cmd = exec.Command("xdg-open", path)
	}
	return cmd.Run()
}
