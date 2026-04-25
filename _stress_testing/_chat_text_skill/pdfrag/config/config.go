package config

import (
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v3"
)

// Config defines the application configuration shape.
type Config struct {
	Database   DatabaseConfig   `yaml:"database"`
	Indexing   IndexingConfig   `yaml:"indexing"`
	Embeddings EmbeddingsConfig `yaml:"embeddings"`
	Search     SearchConfig     `yaml:"search"`
	LLM        LLMConfig        `yaml:"llm"`
	Ollama     OllamaConfig     `yaml:"ollama"`
	Output     OutputConfig     `yaml:"output"`
	Logging    LoggingConfig    `yaml:"logging"`
}

type DatabaseConfig struct {
	Path string `yaml:"path"`
}

type IndexingConfig struct {
	Concurrency int `yaml:"concurrency"`
}

type EmbeddingsConfig struct {
	Model      string `yaml:"model"`
	BatchSize  int    `yaml:"batch_size"`
	OllamaHost string `yaml:"ollama_host"`
}

type SearchConfig struct {
	TopK          int     `yaml:"top_k"`
	MinSimilarity float64 `yaml:"min_similarity"`
}

type LLMConfig struct {
	Provider           string  `yaml:"provider"`
	Model              string  `yaml:"model"`
	Temperature        float64 `yaml:"temperature"`
	MaxTokens          int     `yaml:"max_tokens"`
	OllamaHost         string  `yaml:"ollama_host"`
	OpenRouterAPIKey   string  `yaml:"openrouter_api_key"`
	OpenRouterBaseURL  string  `yaml:"openrouter_base_url"`
	OpenRouterAppTitle string  `yaml:"openrouter_app_title"`
	OpenRouterAppURL   string  `yaml:"openrouter_app_url"`
}

type OllamaConfig struct {
	AutoStart bool `yaml:"auto_start"`
	Warm      bool `yaml:"warm"`
}

type OutputConfig struct {
	Format  string `yaml:"format"`
	Quiet   bool   `yaml:"quiet"`
	Verbose bool   `yaml:"verbose"`
}

type LoggingConfig struct {
	Level string `yaml:"level"`
	File  string `yaml:"file"`
}

// DefaultConfigPath returns the default config path honoring PDFRAG_CONFIG.
func DefaultConfigPath() string {
	if override := os.Getenv("PDFRAG_CONFIG"); override != "" {
		return expandHome(override)
	}
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return ".pdfrag/config.yaml"
	}
	return filepath.Join(home, ".pdfrag", "config.yaml")
}

// DefaultConfig provides baseline values for all config sections.
func DefaultConfig() Config {
	return Config{
		Database: DatabaseConfig{
			Path: "./data/pdfrag.db",
		},
		Indexing: IndexingConfig{
			Concurrency: 4,
		},
		Embeddings: EmbeddingsConfig{
			Model:      "nomic-embed-text",
			BatchSize:  32,
			OllamaHost: "http://localhost:11434",
		},
		Search: SearchConfig{
			TopK:          10,
			MinSimilarity: 0.3,
		},
		LLM: LLMConfig{
			Provider:          "ollama",
			Model:             "llama3.1",
			Temperature:       0.2,
			MaxTokens:         0,
			OllamaHost:        "http://localhost:11434",
			OpenRouterBaseURL: "https://openrouter.ai/api/v1",
		},
		Ollama: OllamaConfig{
			AutoStart: true,
			Warm:      true,
		},
		Output: OutputConfig{
			Format: "rich",
		},
		Logging: LoggingConfig{
			Level: "info",
			File:  "./logs/pdfrag.log",
		},
	}
}

// Load reads configuration from the provided path, applying defaults and env overrides.
func Load(path string) (Config, error) {
	cfg, err := LoadFile(path)
	if err != nil {
		return cfg, err
	}
	applyEnvOverrides(&cfg)
	return cfg, nil
}

// LoadFile reads configuration without applying environment overrides.
func LoadFile(path string) (Config, error) {
	cfg := DefaultConfig()
	if path != "" {
		expanded := expandHome(path)
		contents, err := os.ReadFile(expanded)
		if err != nil {
			if !os.IsNotExist(err) {
				return cfg, err
			}
		} else if err := yaml.Unmarshal(contents, &cfg); err != nil {
			return cfg, err
		}
	}
	return cfg, nil
}

// Save writes configuration to disk, creating the parent directory if needed.
func Save(path string, cfg Config) error {
	if path == "" {
		path = DefaultConfigPath()
	}
	expanded := expandHome(path)
	if err := os.MkdirAll(filepath.Dir(expanded), 0o755); err != nil {
		return err
	}
	data, err := yaml.Marshal(cfg)
	if err != nil {
		return err
	}
	return os.WriteFile(expanded, data, 0o644)
}

func applyEnvOverrides(cfg *Config) {
	if override := os.Getenv("PDFRAG_DB"); override != "" {
		cfg.Database.Path = override
	}
	if override := os.Getenv("OLLAMA_HOST"); override != "" {
		cfg.Embeddings.OllamaHost = override
		cfg.LLM.OllamaHost = override
	}
	if override := os.Getenv("OPENROUTER_API_KEY"); override != "" {
		cfg.LLM.OpenRouterAPIKey = override
	}
}

func expandHome(path string) string {
	if path == "" || path[0] != '~' {
		return path
	}
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return path
	}
	if path == "~" {
		return home
	}
	if strings.HasPrefix(path, "~/") {
		return filepath.Join(home, path[2:])
	}
	return filepath.Join(home, path[1:])
}
