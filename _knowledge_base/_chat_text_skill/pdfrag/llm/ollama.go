package llm

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

const (
	DefaultHost              = "http://localhost:11434"
	DefaultModel             = "llama3.1"
	DefaultTemperature       = 0.2
	DefaultOpenRouterBaseURL = "https://openrouter.ai/api/v1"
)

var ErrEmptyMessages = errors.New("chat messages are empty")
var ErrMissingAPIKey = errors.New("missing api key")

const (
	ProviderOllama     = "ollama"
	ProviderOpenRouter = "openrouter"
)

// Message represents a chat message for Ollama.
type Message struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}

// Options configures the Ollama chat client.
type Options struct {
	Provider           string
	Host               string
	Model              string
	Temperature        float64
	MaxTokens          int
	HTTPClient         *http.Client
	OpenRouterAPIKey   string
	OpenRouterBaseURL  string
	OpenRouterAppTitle string
	OpenRouterAppURL   string
}

// Client calls the Ollama chat API.
type Client struct {
	provider    string
	host        string
	model       string
	temperature float64
	maxTokens   int
	httpClient  *http.Client
	apiKey      string
	appTitle    string
	appURL      string
}

// NewClient builds a chat client with defaults applied.
func NewClient(opts Options) *Client {
	provider := strings.TrimSpace(strings.ToLower(opts.Provider))
	if provider == "" {
		provider = ProviderOllama
	}
	host := strings.TrimRight(opts.Host, "/")
	if host == "" {
		if provider == ProviderOpenRouter {
			host = DefaultOpenRouterBaseURL
		} else {
			host = DefaultHost
		}
	}
	model := opts.Model
	if model == "" {
		model = DefaultModel
	}
	temperature := opts.Temperature
	if isUnsetTemperature(temperature) {
		temperature = DefaultTemperature
	}
	httpClient := opts.HTTPClient
	if httpClient == nil {
		httpClient = &http.Client{Timeout: 90 * time.Second}
	}
	return &Client{
		provider:    provider,
		host:        host,
		model:       model,
		temperature: temperature,
		maxTokens:   opts.MaxTokens,
		httpClient:  httpClient,
		apiKey:      strings.TrimSpace(opts.OpenRouterAPIKey),
		appTitle:    strings.TrimSpace(opts.OpenRouterAppTitle),
		appURL:      strings.TrimSpace(opts.OpenRouterAppURL),
	}
}

// Chat sends a chat completion request and returns the assistant content.
func (c *Client) Chat(ctx context.Context, messages []Message) (string, error) {
	if len(messages) == 0 {
		return "", ErrEmptyMessages
	}
	if c.provider == ProviderOpenRouter {
		return c.chatOpenRouter(ctx, messages)
	}
	return c.chatOllama(ctx, messages)
}

type chatRequest struct {
	Model    string       `json:"model"`
	Messages []Message    `json:"messages"`
	Stream   bool         `json:"stream"`
	Options  *chatOptions `json:"options,omitempty"`
}

type chatOptions struct {
	Temperature float64 `json:"temperature"`
}

type chatResponse struct {
	Message Message `json:"message"`
}

func (c *Client) chatOllama(ctx context.Context, messages []Message) (string, error) {
	payload := chatRequest{
		Model:    c.model,
		Messages: messages,
		Stream:   false,
	}
	if !isUnsetTemperature(c.temperature) {
		payload.Options = &chatOptions{Temperature: c.temperature}
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return "", fmt.Errorf("marshal chat request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.host+"/api/chat", bytes.NewReader(body))
	if err != nil {
		return "", fmt.Errorf("build chat request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("send chat request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		msg, _ := io.ReadAll(io.LimitReader(resp.Body, 2048))
		return "", fmt.Errorf("ollama chat failed: status %d: %s", resp.StatusCode, strings.TrimSpace(string(msg)))
	}

	var response chatResponse
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
		return "", fmt.Errorf("decode chat response: %w", err)
	}
	content := strings.TrimSpace(response.Message.Content)
	if content == "" {
		return "", errors.New("ollama chat returned empty content")
	}
	return content, nil
}

func (c *Client) chatOpenRouter(ctx context.Context, messages []Message) (string, error) {
	if c.apiKey == "" {
		return "", ErrMissingAPIKey
	}
	if strings.TrimSpace(c.model) == "" {
		return "", errors.New("openrouter model is required")
	}
	payload := openRouterChatRequest{
		Model:    c.model,
		Messages: messages,
		Stream:   false,
	}
	if !isUnsetTemperature(c.temperature) {
		payload.Temperature = &c.temperature
	}
	if c.maxTokens > 0 {
		payload.MaxTokens = &c.maxTokens
	}
	body, err := json.Marshal(payload)
	if err != nil {
		return "", fmt.Errorf("marshal chat request: %w", err)
	}

	req, err := http.NewRequestWithContext(ctx, http.MethodPost, c.host+"/chat/completions", bytes.NewReader(body))
	if err != nil {
		return "", fmt.Errorf("build chat request: %w", err)
	}
	req.Header.Set("Content-Type", "application/json")
	req.Header.Set("Authorization", "Bearer "+c.apiKey)
	if c.appURL != "" {
		req.Header.Set("HTTP-Referer", c.appURL)
	}
	if c.appTitle != "" {
		req.Header.Set("X-Title", c.appTitle)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return "", fmt.Errorf("send chat request: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode < http.StatusOK || resp.StatusCode >= http.StatusMultipleChoices {
		msg, _ := io.ReadAll(io.LimitReader(resp.Body, 2048))
		return "", fmt.Errorf("openrouter chat failed: status %d: %s", resp.StatusCode, strings.TrimSpace(string(msg)))
	}

	var response openRouterChatResponse
	if err := json.NewDecoder(resp.Body).Decode(&response); err != nil {
		return "", fmt.Errorf("decode chat response: %w", err)
	}
	if len(response.Choices) == 0 {
		return "", errors.New("openrouter chat returned no choices")
	}
	content := strings.TrimSpace(response.Choices[0].Message.Content)
	if content == "" {
		return "", errors.New("openrouter chat returned empty content")
	}
	return content, nil
}

type openRouterChatRequest struct {
	Model       string    `json:"model"`
	Messages    []Message `json:"messages"`
	Stream      bool      `json:"stream"`
	Temperature *float64  `json:"temperature,omitempty"`
	MaxTokens   *int      `json:"max_tokens,omitempty"`
}

type openRouterChatResponse struct {
	Choices []struct {
		Message Message `json:"message"`
	} `json:"choices"`
}

func isUnsetTemperature(value float64) bool {
	return value < 0
}
