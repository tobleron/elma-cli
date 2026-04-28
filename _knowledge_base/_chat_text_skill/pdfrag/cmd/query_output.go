package cmd

import (
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"

	"pdfrag/storage"
)

type QueryOutputOptions struct {
	Format        string
	Quiet         bool
	Verbose       bool
	ColorEnabled  bool
	TopK          int
	MinSimilarity float64
}

type queryResultOutput struct {
	Rank         int     `json:"rank"`
	Filename     string  `json:"filename"`
	PageNumber   int     `json:"page_number,omitempty"`
	SectionTitle string  `json:"section_title,omitempty"`
	Similarity   float64 `json:"similarity"`
	Content      string  `json:"content"`
	DocumentID   int64   `json:"document_id,omitempty"`
	ChunkID      int64   `json:"chunk_id,omitempty"`
}

type queryOutput struct {
	Question string              `json:"question,omitempty"`
	Answer   string              `json:"answer,omitempty"`
	Results  []queryResultOutput `json:"results,omitempty"`
	Meta     *queryOutputMeta    `json:"meta,omitempty"`
}

type queryOutputMeta struct {
	TopK          int     `json:"top_k"`
	MinSimilarity float64 `json:"min_similarity"`
	Count         int     `json:"count"`
}

func resolveOutputFormat(flagValue string) (string, error) {
	value := strings.TrimSpace(flagValue)
	if value == "" {
		value = strings.TrimSpace(appConfig.Output.Format)
	}
	if value == "" {
		value = "rich"
	}
	value = strings.ToLower(value)
	switch value {
	case "rich", "json", "markdown", "md":
		if value == "md" {
			return "markdown", nil
		}
		return value, nil
	default:
		return "", fmt.Errorf("unknown format %q (use rich, json, markdown)", value)
	}
}

func supportsColor(w io.Writer) bool {
	if os.Getenv("NO_COLOR") != "" {
		return false
	}
	file, ok := w.(*os.File)
	if !ok {
		return false
	}
	info, err := file.Stat()
	if err != nil {
		return false
	}
	return info.Mode()&os.ModeCharDevice != 0
}

func buildExtractiveAnswer(results []storage.SearchResult) string {
	if len(results) == 0 {
		return ""
	}
	return summarizeContent(results[0].Content, 420)
}

func writeQueryOutput(w io.Writer, question, answer string, results []storage.SearchResult, opts QueryOutputOptions) error {
	if w == nil {
		return errors.New("output writer is nil")
	}
	format := strings.ToLower(strings.TrimSpace(opts.Format))
	if format == "" {
		format = "rich"
	}
	if opts.TopK > 0 && len(results) > opts.TopK {
		results = results[:opts.TopK]
	}
	switch format {
	case "json":
		return writeJSONOutput(w, question, answer, results, opts)
	case "markdown":
		return writeMarkdownOutput(w, question, answer, results, opts)
	default:
		return writeRichOutput(w, question, answer, results, opts)
	}
}

func writeJSONOutput(w io.Writer, question, answer string, results []storage.SearchResult, opts QueryOutputOptions) error {
	output := queryOutput{
		Answer: answer,
	}
	if !opts.Quiet {
		output.Question = question
		output.Results = buildResultsOutput(results, opts)
		if opts.Verbose {
			output.Meta = &queryOutputMeta{
				TopK:          opts.TopK,
				MinSimilarity: opts.MinSimilarity,
				Count:         len(results),
			}
		}
	}
	enc := json.NewEncoder(w)
	enc.SetIndent("", "  ")
	return enc.Encode(output)
}

func writeMarkdownOutput(w io.Writer, question, answer string, results []storage.SearchResult, opts QueryOutputOptions) error {
	if opts.Quiet {
		_, err := fmt.Fprintln(w, answer)
		return err
	}
	if question != "" {
		if _, err := fmt.Fprintf(w, "# Query\n\n%s\n\n", question); err != nil {
			return err
		}
	}
	if answer != "" {
		if _, err := fmt.Fprintf(w, "## Answer\n\n%s\n\n", answer); err != nil {
			return err
		}
	}
	if len(results) == 0 {
		return nil
	}
	if _, err := fmt.Fprintln(w, "## Sources"); err != nil {
		return err
	}
	for i, result := range buildResultsOutput(results, opts) {
		label := formatResultLabel(result)
		if _, err := fmt.Fprintf(w, "\n%d. **%s**\n", i+1, label); err != nil {
			return err
		}
		if result.SectionTitle != "" {
			if _, err := fmt.Fprintf(w, "   - Section: %s\n", result.SectionTitle); err != nil {
				return err
			}
		}
		if result.Content != "" {
			if _, err := fmt.Fprintf(w, "   - Snippet: %s\n", result.Content); err != nil {
				return err
			}
		}
		if opts.Verbose {
			if _, err := fmt.Fprintf(w, "   - Similarity: %.3f\n", result.Similarity); err != nil {
				return err
			}
			if result.DocumentID != 0 || result.ChunkID != 0 {
				if _, err := fmt.Fprintf(w, "   - Document ID: %d, Chunk ID: %d\n", result.DocumentID, result.ChunkID); err != nil {
					return err
				}
			}
		}
	}
	return nil
}

func writeRichOutput(w io.Writer, question, answer string, results []storage.SearchResult, opts QueryOutputOptions) error {
	if opts.Quiet {
		_, err := fmt.Fprintln(w, answer)
		return err
	}
	heading := func(text string) string {
		if !opts.ColorEnabled {
			return text
		}
		return "\x1b[1m" + text + "\x1b[0m"
	}
	if question != "" {
		if _, err := fmt.Fprintf(w, "%s %s\n\n", heading("Query:"), question); err != nil {
			return err
		}
	}
	if answer != "" {
		if _, err := fmt.Fprintf(w, "%s\n%s\n\n", heading("Answer:"), answer); err != nil {
			return err
		}
	}
	if len(results) == 0 {
		return nil
	}
	if _, err := fmt.Fprintln(w, heading("Sources:")); err != nil {
		return err
	}
	for i, result := range buildResultsOutput(results, opts) {
		label := formatResultLabel(result)
		if _, err := fmt.Fprintf(w, "%d. %s\n", i+1, label); err != nil {
			return err
		}
		if result.SectionTitle != "" {
			if _, err := fmt.Fprintf(w, "   Section: %s\n", result.SectionTitle); err != nil {
				return err
			}
		}
		if result.Content != "" {
			if _, err := fmt.Fprintf(w, "   %s\n", result.Content); err != nil {
				return err
			}
		}
		if opts.Verbose {
			if _, err := fmt.Fprintf(w, "   Similarity: %.3f\n", result.Similarity); err != nil {
				return err
			}
			if result.DocumentID != 0 || result.ChunkID != 0 {
				if _, err := fmt.Fprintf(w, "   Document ID: %d, Chunk ID: %d\n", result.DocumentID, result.ChunkID); err != nil {
					return err
				}
			}
		}
	}
	return nil
}

func buildResultsOutput(results []storage.SearchResult, opts QueryOutputOptions) []queryResultOutput {
	out := make([]queryResultOutput, 0, len(results))
	for i, result := range results {
		item := queryResultOutput{
			Rank:         i + 1,
			Filename:     result.Filename,
			PageNumber:   result.PageNumber,
			SectionTitle: result.SectionTitle,
			Similarity:   result.Similarity,
			Content:      summarizeContent(result.Content, 240),
		}
		if opts.Verbose {
			item.DocumentID = result.DocumentID
			item.ChunkID = result.ChunkID
		}
		out = append(out, item)
	}
	return out
}

func formatResultLabel(result queryResultOutput) string {
	label := result.Filename
	if result.PageNumber > 0 {
		label = fmt.Sprintf("%s (page %d)", label, result.PageNumber)
	}
	if result.Similarity > 0 {
		label = fmt.Sprintf("%s [%.3f]", label, result.Similarity)
	}
	return label
}

func summarizeContent(content string, maxChars int) string {
	trimmed := strings.TrimSpace(content)
	if trimmed == "" {
		return ""
	}
	fields := strings.Fields(trimmed)
	if len(fields) == 0 {
		return ""
	}
	collapsed := strings.Join(fields, " ")
	if maxChars <= 0 {
		return collapsed
	}
	runes := []rune(collapsed)
	if len(runes) <= maxChars {
		return collapsed
	}
	return string(runes[:maxChars]) + "..."
}
