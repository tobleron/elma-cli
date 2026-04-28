package embeddings

import (
	"bytes"
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"
)

type capturedRequest struct {
	Model  string `json:"model"`
	Prompt string `json:"prompt"`
}

func TestEmbedNormalizesAndUsesDefaultModel(t *testing.T) {
	var got capturedRequest
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path != "/api/embeddings" {
			w.WriteHeader(http.StatusNotFound)
			return
		}
		if err := json.NewDecoder(r.Body).Decode(&got); err != nil {
			w.WriteHeader(http.StatusBadRequest)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"embedding":[3,4]}`))
	}))
	defer server.Close()

	client := NewClient(Options{Host: server.URL})
	vec, err := client.Embed(context.Background(), "hello world")
	if err != nil {
		t.Fatalf("Embed returned error: %v", err)
	}
	if got.Model != DefaultModel {
		t.Fatalf("expected model %q, got %q", DefaultModel, got.Model)
	}
	if got.Prompt != "hello world" {
		t.Fatalf("expected prompt to be forwarded, got %q", got.Prompt)
	}
	if len(vec) != 2 {
		t.Fatalf("expected 2 values, got %d", len(vec))
	}
	if !approxEqual(vec[0], 0.6, 1e-4) || !approxEqual(vec[1], 0.8, 1e-4) {
		t.Fatalf("expected normalized vector [0.6 0.8], got %v", vec)
	}
}

func TestEmbedRetries(t *testing.T) {
	attempts := 0
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		attempts++
		if attempts < 3 {
			w.WriteHeader(http.StatusServiceUnavailable)
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"embedding":[1,0]}`))
	}))
	defer server.Close()

	client := NewClient(Options{
		Host:     server.URL,
		Backoffs: []time.Duration{1 * time.Millisecond, 1 * time.Millisecond},
	})
	vec, err := client.Embed(context.Background(), "retry")
	if err != nil {
		t.Fatalf("Embed returned error after retries: %v", err)
	}
	if attempts != 3 {
		t.Fatalf("expected 3 attempts, got %d", attempts)
	}
	if len(vec) != 2 {
		t.Fatalf("expected embedding length 2, got %d", len(vec))
	}
}

func TestEmbedEmptyInput(t *testing.T) {
	client := NewClient(Options{Host: "http://example"})
	if _, err := client.Embed(context.Background(), "  "); err != ErrEmptyInput {
		t.Fatalf("expected ErrEmptyInput, got %v", err)
	}
}

func approxEqual(value float32, target float64, tolerance float64) bool {
	diff := float64(value) - target
	if diff < 0 {
		diff = -diff
	}
	return diff <= tolerance
}

func TestEmbedTextsBatching(t *testing.T) {
	var prompts []string
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		var req capturedRequest
		_ = json.NewDecoder(r.Body).Decode(&req)
		prompts = append(prompts, req.Prompt)
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"embedding":[1,0]}`))
	}))
	defer server.Close()

	client := NewClient(Options{Host: server.URL})
	texts := []string{"one", "two", "three"}
	vecs, err := client.EmbedTexts(context.Background(), texts, 2)
	if err != nil {
		t.Fatalf("EmbedTexts returned error: %v", err)
	}
	if len(vecs) != len(texts) {
		t.Fatalf("expected %d embeddings, got %d", len(texts), len(vecs))
	}
	if len(prompts) != len(texts) {
		t.Fatalf("expected %d requests, got %d", len(texts), len(prompts))
	}
	if !bytes.Equal([]byte(prompts[0]), []byte("one")) {
		t.Fatalf("unexpected prompt order: %v", prompts)
	}
}
