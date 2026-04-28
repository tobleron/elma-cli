package chunking

import (
	"regexp"
	"strings"
	"unicode"
)

const (
	DefaultTargetTokens  = 600
	DefaultMaxTokens     = 1000
	DefaultOverlapTokens = 100
)

// Options controls chunking behavior.
type Options struct {
	TargetTokens  int
	MaxTokens     int
	OverlapTokens int
}

// Chunk represents a semantic chunk and its metadata.
type Chunk struct {
	DocumentID   int64
	PageNumber   int
	SectionTitle string
	Content      string
	TokenCount   int
}

type section struct {
	title      string
	pageNumber int
	content    string
}

type unit struct {
	text         string
	tokens       int
	paragraphEnd bool
}

var (
	headerPattern = regexp.MustCompile(`^\s{0,3}#{1,6}\s+(.+?)\s*$`)
	pagePattern   = regexp.MustCompile(`(?i)^Page\s+(\d+)\b`)
)

// DefaultOptions returns the default chunking options.
func DefaultOptions() Options {
	return Options{
		TargetTokens:  DefaultTargetTokens,
		MaxTokens:     DefaultMaxTokens,
		OverlapTokens: DefaultOverlapTokens,
	}
}

// ChunkMarkdown splits markdown into semantic chunks with metadata.
func ChunkMarkdown(documentID int64, markdown string, opts Options) []Chunk {
	normalized := normalizeOptions(opts)
	sections := splitMarkdownSections(markdown)
	if len(sections) == 0 {
		trimmed := strings.TrimSpace(markdown)
		if trimmed == "" {
			return nil
		}
		sections = []section{{content: trimmed}}
	}

	chunks := make([]Chunk, 0, len(sections)*2)
	for _, sec := range sections {
		units := splitSectionIntoUnits(sec.content, normalized.MaxTokens)
		if len(units) == 0 {
			continue
		}
		sectionChunks := chunkUnits(documentID, sec, units, normalized)
		chunks = append(chunks, sectionChunks...)
	}
	return chunks
}

func normalizeOptions(opts Options) Options {
	if opts.MaxTokens <= 0 {
		opts.MaxTokens = DefaultMaxTokens
	}
	if opts.TargetTokens <= 0 {
		opts.TargetTokens = DefaultTargetTokens
	}
	if opts.TargetTokens > opts.MaxTokens {
		opts.TargetTokens = opts.MaxTokens
	}
	if opts.OverlapTokens <= 0 {
		opts.OverlapTokens = DefaultOverlapTokens
	}
	if opts.OverlapTokens >= opts.MaxTokens {
		if opts.MaxTokens > 1 {
			opts.OverlapTokens = opts.MaxTokens - 1
		} else {
			opts.OverlapTokens = 0
		}
	}
	return opts
}

func splitMarkdownSections(markdown string) []section {
	text := strings.ReplaceAll(markdown, "\r\n", "\n")
	text = strings.ReplaceAll(text, "\r", "\n")
	lines := strings.Split(text, "\n")

	sections := []section{}
	current := section{}
	buffer := make([]string, 0, 32)

	flush := func() {
		content := strings.TrimSpace(strings.Join(buffer, "\n"))
		if content == "" {
			buffer = buffer[:0]
			return
		}
		current.content = content
		sections = append(sections, current)
		buffer = buffer[:0]
	}

	for _, line := range lines {
		if title, ok := parseHeader(line); ok {
			flush()
			current.title = title
			if pageNum, ok := parsePageNumber(title); ok {
				current.pageNumber = pageNum
			}
			continue
		}
		buffer = append(buffer, line)
	}
	flush()

	return sections
}

func parseHeader(line string) (string, bool) {
	matches := headerPattern.FindStringSubmatch(line)
	if len(matches) < 2 {
		return "", false
	}
	return strings.TrimSpace(matches[1]), true
}

func parsePageNumber(title string) (int, bool) {
	matches := pagePattern.FindStringSubmatch(title)
	if len(matches) < 2 {
		return 0, false
	}
	pageNumber := 0
	for _, r := range matches[1] {
		if r < '0' || r > '9' {
			return 0, false
		}
		pageNumber = pageNumber*10 + int(r-'0')
	}
	if pageNumber <= 0 {
		return 0, false
	}
	return pageNumber, true
}

func splitSectionIntoUnits(content string, maxTokens int) []unit {
	paragraphs := splitParagraphs(content)
	units := make([]unit, 0, len(paragraphs))
	for _, paragraph := range paragraphs {
		appendParagraphUnits(paragraph, maxTokens, &units)
	}
	return units
}

func splitParagraphs(content string) []string {
	text := strings.ReplaceAll(content, "\r\n", "\n")
	text = strings.ReplaceAll(text, "\r", "\n")
	lines := strings.Split(text, "\n")

	paragraphs := make([]string, 0, len(lines)/4+1)
	current := make([]string, 0, 8)

	flush := func() {
		if len(current) == 0 {
			return
		}
		paragraphs = append(paragraphs, strings.Join(current, " "))
		current = current[:0]
	}

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		if trimmed == "" {
			flush()
			continue
		}
		if len(current) == 0 {
			current = append(current, trimmed)
			continue
		}
		last := current[len(current)-1]
		if strings.HasSuffix(last, "-") && startsWithLower(trimmed) {
			current[len(current)-1] = strings.TrimSuffix(last, "-") + trimmed
			continue
		}
		current = append(current, trimmed)
	}
	flush()
	return paragraphs
}

func startsWithLower(value string) bool {
	for _, r := range value {
		if unicode.IsSpace(r) {
			continue
		}
		return unicode.IsLower(r)
	}
	return false
}

func appendParagraphUnits(paragraph string, maxTokens int, units *[]unit) {
	paragraph = strings.TrimSpace(paragraph)
	if paragraph == "" {
		return
	}
	paragraphTokens := countTokens(paragraph)
	if paragraphTokens <= maxTokens {
		*units = append(*units, unit{text: paragraph, tokens: paragraphTokens, paragraphEnd: true})
		return
	}

	sentences := splitSentences(paragraph)
	if len(sentences) == 0 {
		return
	}

	for i, sentence := range sentences {
		isLastSentence := i == len(sentences)-1
		appendSentenceUnits(sentence, maxTokens, isLastSentence, units)
	}
}

func appendSentenceUnits(sentence string, maxTokens int, paragraphEnd bool, units *[]unit) {
	sentence = strings.TrimSpace(sentence)
	if sentence == "" {
		return
	}
	sentenceTokens := countTokens(sentence)
	if sentenceTokens <= maxTokens {
		*units = append(*units, unit{text: sentence, tokens: sentenceTokens, paragraphEnd: paragraphEnd})
		return
	}

	parts := splitByTokens(sentence, maxTokens)
	for i, part := range parts {
		last := paragraphEnd && i == len(parts)-1
		*units = append(*units, unit{text: part, tokens: countTokens(part), paragraphEnd: last})
	}
}

func splitSentences(paragraph string) []string {
	var sentences []string
	runes := []rune(paragraph)
	start := 0
	for i, r := range runes {
		switch r {
		case '.', '!', '?':
			next := i + 1
			if next == len(runes) || unicode.IsSpace(runes[next]) {
				sentence := strings.TrimSpace(string(runes[start : i+1]))
				if sentence != "" {
					sentences = append(sentences, sentence)
				}
				start = i + 1
			}
		}
	}
	if start < len(runes) {
		trail := strings.TrimSpace(string(runes[start:]))
		if trail != "" {
			sentences = append(sentences, trail)
		}
	}
	return sentences
}

func splitByTokens(text string, maxTokens int) []string {
	words := strings.Fields(text)
	if len(words) == 0 {
		return nil
	}
	if maxTokens <= 0 || len(words) <= maxTokens {
		return []string{strings.Join(words, " ")}
	}
	parts := make([]string, 0, (len(words)/maxTokens)+1)
	for i := 0; i < len(words); i += maxTokens {
		end := i + maxTokens
		if end > len(words) {
			end = len(words)
		}
		parts = append(parts, strings.Join(words[i:end], " "))
	}
	return parts
}

func chunkUnits(documentID int64, sec section, units []unit, opts Options) []Chunk {
	chunks := make([]Chunk, 0, len(units)/3+1)
	var buffer []unit
	bufferTokens := 0
	bufferFromOverlap := false

	emit := func() {
		if len(buffer) == 0 {
			return
		}
		content := strings.TrimSpace(joinUnits(buffer))
		if content == "" {
			return
		}
		chunks = append(chunks, Chunk{
			DocumentID:   documentID,
			PageNumber:   sec.pageNumber,
			SectionTitle: sec.title,
			Content:      content,
			TokenCount:   countTokens(content),
		})
	}

	emitAndOverlap := func() {
		emit()
		buffer = overlapUnits(buffer, opts.OverlapTokens)
		bufferTokens = sumTokens(buffer)
		bufferFromOverlap = len(buffer) > 0
	}

	for _, u := range units {
		if u.tokens == 0 {
			continue
		}
		if bufferTokens+u.tokens > opts.MaxTokens && bufferTokens > 0 {
			if bufferFromOverlap {
				buffer = overlapUnits(buffer, max(0, opts.MaxTokens-u.tokens))
				bufferTokens = sumTokens(buffer)
				if bufferTokens+u.tokens > opts.MaxTokens {
					buffer = nil
					bufferTokens = 0
					bufferFromOverlap = false
				}
			} else {
				emitAndOverlap()
			}
		}
		buffer = append(buffer, u)
		bufferTokens += u.tokens
		bufferFromOverlap = false

		if bufferTokens >= opts.TargetTokens {
			emitAndOverlap()
		}
	}

	if bufferTokens > 0 && !bufferFromOverlap {
		emit()
	}
	return chunks
}

func joinUnits(units []unit) string {
	var builder strings.Builder
	for i, u := range units {
		if i > 0 {
			if units[i-1].paragraphEnd {
				builder.WriteString("\n\n")
			} else {
				builder.WriteString(" ")
			}
		}
		builder.WriteString(u.text)
	}
	return builder.String()
}

func overlapUnits(units []unit, overlapTokens int) []unit {
	if overlapTokens <= 0 || len(units) == 0 {
		return nil
	}
	remaining := overlapTokens
	result := []unit{}
	for i := len(units) - 1; i >= 0 && remaining > 0; i-- {
		u := units[i]
		if u.tokens <= remaining {
			result = append([]unit{u}, result...)
			remaining -= u.tokens
			continue
		}
		trimmed := trimToLastTokens(u.text, remaining)
		if trimmed != "" {
			result = append([]unit{{text: trimmed, tokens: remaining, paragraphEnd: u.paragraphEnd}}, result...)
		}
		remaining = 0
	}
	return result
}

func trimToLastTokens(text string, tokens int) string {
	if tokens <= 0 {
		return ""
	}
	words := strings.Fields(text)
	if len(words) == 0 {
		return ""
	}
	if tokens >= len(words) {
		return strings.Join(words, " ")
	}
	return strings.Join(words[len(words)-tokens:], " ")
}

func sumTokens(units []unit) int {
	total := 0
	for _, u := range units {
		total += u.tokens
	}
	return total
}

func countTokens(text string) int {
	return len(strings.Fields(text))
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
