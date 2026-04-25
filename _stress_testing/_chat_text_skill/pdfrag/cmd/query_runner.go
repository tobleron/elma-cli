package cmd

import (
	"context"
	"fmt"
	"strings"

	"pdfrag/embeddings"
	"pdfrag/llm"
	"pdfrag/logging"
	"pdfrag/storage"

	"go.uber.org/zap"
)

type searchOptions struct {
	TopK          int
	MinSimilarity float64
}

func resolveSearchOptions(topK int, minSimilarity float64) (int, float64) {
	resolvedTopK := topK
	if resolvedTopK <= 0 {
		resolvedTopK = appConfig.Search.TopK
	}
	if resolvedTopK <= 0 {
		resolvedTopK = 10
	}
	resolvedMin := minSimilarity
	if resolvedMin < 0 {
		resolvedMin = appConfig.Search.MinSimilarity
	}
	if resolvedMin < 0 {
		resolvedMin = 0
	}
	return resolvedTopK, resolvedMin
}

func performQuery(ctx context.Context, question string, opts searchOptions) (string, []storage.SearchResult, error) {
	trimmed := strings.TrimSpace(question)
	if trimmed == "" {
		return "", nil, fmt.Errorf("query cannot be empty")
	}

	db, err := storage.InitDuckDB(ctx, appConfig.Database.Path)
	if err != nil {
		return "", nil, err
	}
	defer func() {
		if closeErr := db.Close(); closeErr != nil {
			logging.L().Warn("failed to close database",
				zap.Error(closeErr),
			)
		}
	}()

	if err := ensureOllama(ctx, appConfig.Embeddings.OllamaHost, nil); err != nil {
		return "", nil, err
	}
	if appConfig.Ollama.Warm {
		if err := warmOllamaEmbeddings(ctx, appConfig.Embeddings.OllamaHost, appConfig.Embeddings.Model); err != nil {
			logging.L().Warn("ollama embeddings warmup failed", zap.Error(err))
		}
	}
	llmProvider := strings.TrimSpace(strings.ToLower(appConfig.LLM.Provider))
	if llmProvider == "" {
		llmProvider = llm.ProviderOllama
	}
	if llmProvider == llm.ProviderOllama {
		if appConfig.LLM.OllamaHost != "" && appConfig.LLM.OllamaHost != appConfig.Embeddings.OllamaHost {
			if err := ensureOllama(ctx, appConfig.LLM.OllamaHost, nil); err != nil {
				return "", nil, err
			}
		}
		if appConfig.Ollama.Warm && appConfig.LLM.OllamaHost != "" {
			if err := warmOllamaChat(ctx, appConfig.LLM.OllamaHost, appConfig.LLM.Model, appConfig.LLM.Temperature); err != nil {
				logging.L().Warn("ollama chat warmup failed", zap.Error(err))
			}
		}
	}

	embedClient := embeddings.NewClient(embeddings.Options{
		Host:  appConfig.Embeddings.OllamaHost,
		Model: appConfig.Embeddings.Model,
	})
	queryEmbedding, err := embedClient.Embed(ctx, trimmed)
	if err != nil {
		return "", nil, fmt.Errorf("embed query: %w", err)
	}

	results, err := storage.SearchEmbeddings(ctx, db, queryEmbedding, storage.SearchOptions{
		TopK:           opts.TopK,
		MinSimilarity:  opts.MinSimilarity,
		MaxPerDocument: 3,
	})
	if err != nil {
		return "", nil, err
	}

	answer := ""
	llmFailed := false
	if len(results) > 0 {
		llmHost := appConfig.LLM.OllamaHost
		if llmProvider == llm.ProviderOpenRouter {
			llmHost = appConfig.LLM.OpenRouterBaseURL
		}
		llmClient := llm.NewClient(llm.Options{
			Provider:           llmProvider,
			Host:               llmHost,
			Model:              appConfig.LLM.Model,
			Temperature:        appConfig.LLM.Temperature,
			MaxTokens:          appConfig.LLM.MaxTokens,
			OpenRouterAPIKey:   appConfig.LLM.OpenRouterAPIKey,
			OpenRouterBaseURL:  appConfig.LLM.OpenRouterBaseURL,
			OpenRouterAppTitle: appConfig.LLM.OpenRouterAppTitle,
			OpenRouterAppURL:   appConfig.LLM.OpenRouterAppURL,
		})
		answer, err = generateLLMAnswer(ctx, llmClient, trimmed, results)
		if err != nil {
			llmFailed = true
			logging.L().Warn("llm answer failed",
				zap.Error(err),
			)
			answer = ""
		}
	}
	if answer == "" {
		answer = buildExtractiveAnswer(results)
	}
	if len(results) == 0 && answer == "" {
		answer = "No results above similarity threshold."
	}
	if answer != "" {
		answer = ensureAnswerCitations(answer, results)
		if llmFailed {
			logging.L().Info("llm unavailable, returned extractive answer",
				zap.Bool("llm_failed", llmFailed),
			)
		}
	}

	return answer, results, nil
}
