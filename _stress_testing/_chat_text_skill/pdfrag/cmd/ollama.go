package cmd

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"os/exec"
	"strings"
	"sync"
	"time"

	"pdfrag/embeddings"
	"pdfrag/llm"
	"pdfrag/logging"

	"go.uber.org/zap"
)

var (
	ollamaStartMu        sync.Mutex
	ollamaStartAttempted bool
	ollamaStartErr       error
)

func ensureOllama(ctx context.Context, host string, out io.Writer) error {
	return ensureOllamaWithStart(ctx, host, out, startOllama)
}

func ensureOllamaWithStart(ctx context.Context, host string, out io.Writer, allowStart bool) error {
	parsed, normalized, err := parseOllamaHost(host)
	if err != nil {
		return err
	}
	if ollamaReachable(ctx, normalized) {
		return nil
	}
	logging.L().Warn("ollama not reachable", zap.String("host", normalized))
	if out != nil {
		fmt.Fprintf(out, "Ollama not reachable at %s\n", normalized)
	}
	if !allowStart {
		msg := fmt.Sprintf("ollama not reachable at %q (auto-start disabled)", normalized)
		logging.L().Warn("ollama unavailable", zap.String("host", normalized))
		if out != nil {
			fmt.Fprintln(out, msg)
		}
		return errors.New(msg)
	}
	if !isLocalHost(parsed) {
		msg := fmt.Sprintf("ollama not reachable at non-local host %q", normalized)
		logging.L().Warn("ollama unavailable", zap.String("host", normalized))
		if out != nil {
			fmt.Fprintln(out, msg)
		}
		return errors.New(msg)
	}
	if err := startOllamaProcess(out); err != nil {
		return err
	}
	if err := waitForOllama(ctx, normalized, 20*time.Second); err != nil {
		return err
	}
	return nil
}

func warmOllamaEmbeddings(ctx context.Context, host, model string) error {
	client := embeddings.NewClient(embeddings.Options{
		Host:  host,
		Model: model,
	})
	_, err := client.Embed(ctx, "warmup")
	return err
}

func warmOllamaChat(ctx context.Context, host, model string, temperature float64) error {
	client := llm.NewClient(llm.Options{
		Host:        host,
		Model:       model,
		Temperature: temperature,
	})
	_, err := client.Chat(ctx, []llm.Message{
		{Role: "user", Content: "warmup"},
	})
	return err
}

func parseOllamaHost(raw string) (*url.URL, string, error) {
	trimmed := strings.TrimSpace(raw)
	if trimmed == "" {
		trimmed = "http://localhost:11434"
	}
	if !strings.Contains(trimmed, "://") {
		trimmed = "http://" + trimmed
	}
	parsed, err := url.Parse(trimmed)
	if err != nil {
		return nil, "", fmt.Errorf("parse ollama host: %w", err)
	}
	if parsed.Host == "" && parsed.Path != "" {
		parsed.Host = parsed.Path
		parsed.Path = ""
	}
	if parsed.Scheme == "" {
		parsed.Scheme = "http"
	}
	parsed.Path = strings.TrimRight(parsed.Path, "/")
	return parsed, strings.TrimRight(parsed.String(), "/"), nil
}

func isLocalHost(u *url.URL) bool {
	host := strings.ToLower(u.Hostname())
	return host == "localhost" || host == "127.0.0.1" || host == "::1"
}

func ollamaReachable(ctx context.Context, baseURL string) bool {
	client := &http.Client{Timeout: 1 * time.Second}
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, baseURL+"/api/version", nil)
	if err != nil {
		return false
	}
	resp, err := client.Do(req)
	if err != nil {
		return false
	}
	_ = resp.Body.Close()
	return true
}

func startOllamaProcess(out io.Writer) error {
	ollamaStartMu.Lock()
	defer ollamaStartMu.Unlock()
	if ollamaStartAttempted {
		return ollamaStartErr
	}
	ollamaStartAttempted = true
	path, err := exec.LookPath("ollama")
	if err != nil {
		ollamaStartErr = fmt.Errorf("ollama not found in PATH: %w", err)
		return ollamaStartErr
	}
	if out != nil {
		fmt.Fprintln(out, "Starting Ollama...")
	}
	cmd := exec.Command(path, "serve")
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard
	if err := cmd.Start(); err != nil {
		ollamaStartErr = fmt.Errorf("start ollama: %w", err)
		return ollamaStartErr
	}
	go func() {
		if err := cmd.Wait(); err != nil {
			logging.L().Warn("ollama exited", zap.Error(err))
		}
	}()
	ollamaStartErr = nil
	return nil
}

func waitForOllama(ctx context.Context, baseURL string, timeout time.Duration) error {
	if timeout <= 0 {
		timeout = 10 * time.Second
	}
	deadline := time.Now().Add(timeout)
	for {
		if ctx != nil && ctx.Err() != nil {
			return ctx.Err()
		}
		if ollamaReachable(ctx, baseURL) {
			return nil
		}
		if time.Now().After(deadline) {
			return errors.New("ollama did not become ready before timeout")
		}
		time.Sleep(250 * time.Millisecond)
	}
}

func pullOllamaModel(ctx context.Context, out io.Writer, model string) error {
	parsed, normalized, err := parseOllamaHost(appConfig.Embeddings.OllamaHost)
	if err != nil {
		return err
	}
	if !isLocalHost(parsed) {
		return fmt.Errorf("ollama pull requires a local host; current host is %q", normalized)
	}
	path, err := exec.LookPath("ollama")
	if err != nil {
		return fmt.Errorf("ollama not found in PATH: %w", err)
	}
	if out != nil {
		fmt.Fprintf(out, "Pulling model %s...\n", model)
	}
	cmd := exec.CommandContext(ctx, path, "pull", model)
	cmd.Stdout = out
	cmd.Stderr = out
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("ollama pull %s: %w", model, err)
	}
	return nil
}
