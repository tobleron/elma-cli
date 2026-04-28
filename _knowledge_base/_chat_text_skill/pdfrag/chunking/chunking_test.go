package chunking

import (
	"fmt"
	"strings"
	"testing"
)

func TestChunkMarkdown_Metadata(t *testing.T) {
	markdown := strings.Join([]string{
		"# Doc",
		"",
		"## Page 1",
		"",
		"### Intro",
		"This is sentence one. This is sentence two.",
		"",
		"## Page 2",
		"Another paragraph.",
	}, "\n")

	opts := Options{TargetTokens: 5, MaxTokens: 20, OverlapTokens: 2}
	chunks := ChunkMarkdown(42, markdown, opts)
	if len(chunks) == 0 {
		t.Fatal("expected chunks")
	}

	foundIntro := false
	foundPage2 := false
	for _, chunk := range chunks {
		if chunk.DocumentID != 42 {
			t.Fatalf("unexpected document id: %d", chunk.DocumentID)
		}
		switch chunk.SectionTitle {
		case "Intro":
			foundIntro = true
			if chunk.PageNumber != 1 {
				t.Fatalf("expected page 1 for Intro, got %d", chunk.PageNumber)
			}
		case "Page 2":
			foundPage2 = true
			if chunk.PageNumber != 2 {
				t.Fatalf("expected page 2 for Page 2, got %d", chunk.PageNumber)
			}
		}
	}

	if !foundIntro {
		t.Fatal("expected Intro section chunks")
	}
	if !foundPage2 {
		t.Fatal("expected Page 2 section chunks")
	}
}

func TestChunkMarkdown_Overlap(t *testing.T) {
	paragraphs := buildParagraphs(6, 5)
	markdown := strings.Join([]string{
		"# Doc",
		"",
		"## Page 1",
		"",
		paragraphs,
	}, "\n")

	opts := Options{TargetTokens: 10, MaxTokens: 12, OverlapTokens: 2}
	chunks := ChunkMarkdown(1, markdown, opts)
	if len(chunks) < 3 {
		t.Fatalf("expected at least 3 chunks, got %d", len(chunks))
	}

	for i := 1; i < len(chunks); i++ {
		prevTokens := strings.Fields(chunks[i-1].Content)
		currTokens := strings.Fields(chunks[i].Content)
		if len(prevTokens) < opts.OverlapTokens || len(currTokens) < opts.OverlapTokens {
			t.Fatalf("insufficient tokens for overlap check")
		}
		prevOverlap := prevTokens[len(prevTokens)-opts.OverlapTokens:]
		currOverlap := currTokens[:opts.OverlapTokens]
		if strings.Join(prevOverlap, " ") != strings.Join(currOverlap, " ") {
			t.Fatalf("expected overlap between chunks %d and %d", i-1, i)
		}
	}

	for _, chunk := range chunks {
		if chunk.TokenCount > opts.MaxTokens {
			t.Fatalf("chunk exceeds max tokens: %d", chunk.TokenCount)
		}
	}
}

func buildParagraphs(paragraphCount, tokensPerParagraph int) string {
	var builder strings.Builder
	index := 1
	for p := 0; p < paragraphCount; p++ {
		if p > 0 {
			builder.WriteString("\n\n")
		}
		for t := 0; t < tokensPerParagraph; t++ {
			if t > 0 {
				builder.WriteString(" ")
			}
			builder.WriteString(fmt.Sprintf("w%d", index))
			index++
		}
	}
	return builder.String()
}
