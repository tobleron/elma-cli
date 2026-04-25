package cmd

import (
	"context"
	"fmt"
	"regexp"
	"strings"

	"pdfrag/llm"
	"pdfrag/storage"
)

const llmSystemPrompt = `You are a helpful assistant that answers questions using only the provided sources.
If the sources do not contain enough information, say you don't know.
Cite sources in brackets using the format [filename p.X] after each sentence that references a source.`

func generateLLMAnswer(ctx context.Context, client *llm.Client, question string, results []storage.SearchResult) (string, error) {
	if client == nil {
		return "", fmt.Errorf("llm client is nil")
	}
	sources := buildLLMSources(results)
	if sources == "" {
		return "", fmt.Errorf("no sources available")
	}
	content := fmt.Sprintf("Question: %s\n\nSources:\n%s", strings.TrimSpace(question), sources)
	messages := []llm.Message{
		{Role: "system", Content: llmSystemPrompt},
		{Role: "user", Content: content},
	}
	return client.Chat(ctx, messages)
}

func ensureAnswerCitations(answer string, results []storage.SearchResult) string {
	trimmed := strings.TrimSpace(answer)
	if trimmed == "" || len(results) == 0 {
		return answer
	}
	if citationPattern.MatchString(trimmed) {
		return answer
	}
	citations := collectCitationTags(results, 3)
	if len(citations) == 0 {
		return answer
	}
	return fmt.Sprintf("%s\nSources: %s", trimmed, strings.Join(citations, " "))
}

var citationPattern = regexp.MustCompile(`\\[[^\\]]*p\\.\\d+[^\\]]*\\]`)

func collectCitationTags(results []storage.SearchResult, limit int) []string {
	seen := make(map[string]struct{})
	tags := make([]string, 0, limit)
	for _, result := range results {
		tag := formatCitationTag(result)
		if tag == "" {
			continue
		}
		if _, ok := seen[tag]; ok {
			continue
		}
		seen[tag] = struct{}{}
		tags = append(tags, tag)
		if limit > 0 && len(tags) >= limit {
			break
		}
	}
	return tags
}

func formatCitationTag(result storage.SearchResult) string {
	label := strings.TrimSpace(result.Filename)
	if label == "" {
		label = "unknown"
	}
	if result.PageNumber > 0 {
		return fmt.Sprintf("[%s p.%d]", label, result.PageNumber)
	}
	return fmt.Sprintf("[%s]", label)
}

func buildLLMSources(results []storage.SearchResult) string {
	var builder strings.Builder
	for i, result := range results {
		label := formatCitation(result)
		content := summarizeContent(result.Content, 1200)
		if content == "" {
			continue
		}
		if builder.Len() > 0 {
			builder.WriteString("\n\n")
		}
		fmt.Fprintf(&builder, "%d. %s", i+1, label)
		if result.SectionTitle != "" {
			fmt.Fprintf(&builder, " — %s", result.SectionTitle)
		}
		fmt.Fprintf(&builder, "\n%s", content)
	}
	return strings.TrimSpace(builder.String())
}

func formatCitation(result storage.SearchResult) string {
	label := strings.TrimSpace(result.Filename)
	if label == "" {
		label = "unknown"
	}
	if result.PageNumber > 0 {
		return fmt.Sprintf("%s p.%d", label, result.PageNumber)
	}
	return label
}
