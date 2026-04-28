package embeddings

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"math"
	"net/http"
	"strings"
	"time"

	"pdfrag/logging"

	"go.uber.org/zap"
)

const (
	DefaultModel        = "nomic-embed-text"
	DefaultHost         = "http://localhost:11434"
	DefaultBatchSize    = 32
	DefaultEmbeddingDim = 768
)

var DefaultBackoff = []time.Duration{1 * time.Second, 2 * time.Second, 4 * time.Second}

var ErrEmptyInput = errors.New("embedding input is empty")

// Options configures the Ollama embeddings client.
type Options struct {
	Host       string
	Model      string
	HTTPClient *http.Client
	Backoffs   []time.Duration
}

// Client calls the Ollama embeddings API.
type Client struct {
	host       string
	model      string
	httpClient *http.Client
	backoffs   []time.Duration
}

// NewClient builds a client with defaults applied.
func NewClient(opts Options) *Client {
	host := strings.TrimRight(opts.Host, "/")
	if host == "" {
		host = DefaultHost
	}
	model := opts.Model
	if model == "" {
		model = DefaultModel
	}
	httpClient := opts.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{Timeout: 60 * time.Second}
	}
	backoffs := opts.Backoffs
	if len(backoffs) == 0 {
		backoffs = DefaultBackoff
	}
	return &Client{
		host:       host,
		model:      model,
		httpClient: httpClient,
		backoffs:   backoffs,
	}
}

// Embed returns a normalized embedding for a single text.
func (c *Client) Embed(ctx context.Context, text string) ([]float32, error) {
	prompt := strings.TrimSpace(text)
	if prompt == "" {
		return nil, ErrEmptyInput
	}

	var lastErr error
	for attempt := 0; attempt <= len(c.backoffs); attempt++ {
		embedding, err := c.embedOnce(ctx, prompt)
		if err == nil {
			return embedding, nil
		}
		lastErr = err
		if attempt < len(c.backoffs) {
			if err := sleepWithContext(ctx, c.backoffs[attempt]); err != nil {
				return nil, err
			}
		}
	}
	return nil, lastErr
}

// EmbedTexts returns embeddings for each input text using batch processing.
func (c *Client) EmbedTexts(ctx context.Context, texts []string, batchSize int) ([][]float32, error) {
	if len(texts) == 0 {
		return nil, nil
	}
	batchSize = normalizeBatchSize(batchSize)
	results := make([][]float32, 0, len(texts))
	for start := 0; start < len(texts); start += batchSize {
		end := start + batchSize
		if end > len(texts) {
			end = len(texts)
		}
		for i := start; i < end; i++ {
			if ctx != nil {
				select {
				case <-ctx.Done():
					return nil, ctx.Err()
				default:
				}
			}
			embedding, err := c.Embed(ctx, texts[i])
			if err != nil {
				return nil, fmt.Errorf("embed text %d: %w", i, err)
			}
			results = append(results, embedding)
		}
	}
	return results, nil
}

func (c *Client) embedOnce(ctx context.Context, prompt string) ([]float32, error) {
	payload := embeddingsRequest{
		Model:  c.model,
		Prompt: prompt,
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return nil, fmt.Errorf("marshal embeddings request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.host+"/api/embeddings", bytes.NewReader(body))
	if err != nil {
		return nil, fmt.Errorf("build embeddings request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("send embeddings request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		msg, _ := io.ReadAll(io.LimitReader(resp.Body, 2048))
		return nil, fmt.Errorf("ollama embeddings failed: status %d: %s", resp.StatusCode, strings.TrimSpace(string(msg)))
	}

	var response embeddingsResponse
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
		return nil, fmt.Errorf("decode embeddings response: %w", err)
	}
	if len(response.Embedding) == 0 {
		return nil, errors.New("ollama embeddings returned empty vector")
	}
	if len(response.Embedding) != DefaultEmbeddingDim {
		logging.L().Warn("unexpected embedding dimension",
			zap.Int("dimensions", len(response.Embedding)),
			zap.String("model", c.model),
		)
	}
	return normalizeEmbedding(response.Embedding)
}

type embeddingsRequest struct {
	Model  string `json:"model"`
	Prompt string `json:"prompt"`
}

type embeddingsResponse struct {
	Embedding []float64 `json:"embedding"`
}

func normalizeEmbedding(vec []float64) ([]float32, error) {
	var sum float64
	for _, v := range vec {
		sum += v * v
	}
	if sum == 0 || math.IsNaN(sum) || math.IsInf(sum, 0) {
		return nil, errors.New("invalid embedding vector")
	}
	norm := math.Sqrt(sum)
	if norm == 0 || math.IsNaN(norm) || math.IsInf(norm, 0) {
		return nil, errors.New("invalid embedding norm")
	}
	out := make([]float32, len(vec))
	for i, v := range vec {
		out[i] = float32(v / norm)
	}
	return out, nil
}

func sleepWithContext(ctx context.Context, d time.Duration) error {
	if d <= 0 {
		return nil
	}
	if ctx == nil {
		time.Sleep(d)
		return nil
	}
	timer := time.NewTimer(d)
	defer timer.Stop()
	select {
	case <-ctx.Done():
		return ctx.Err()
	case <-timer.C:
		return nil
	}
}

func normalizeBatchSize(batchSize int) int {
	if batchSize <= 0 {
		return DefaultBatchSize
	}
	return batchSize
}
