package cmd

import (
	"context"
	"errors"
	"fmt"
	"io"
	"strings"

	"github.com/spf13/cobra"
)

func newSetupCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:          "setup",
		Short:        "Start Ollama and pull required models",
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			ctx := cmd.Context()
			parsed, normalized, err := parseOllamaHost(appConfig.Embeddings.OllamaHost)
			if err != nil {
				return err
			}
			if !isLocalHost(parsed) {
				return fmt.Errorf("setup requires a local Ollama host; current host is %q", normalized)
			}
			if appConfig.Embeddings.OllamaHost != "" && appConfig.LLM.OllamaHost != "" && appConfig.Embeddings.OllamaHost != appConfig.LLM.OllamaHost {
				return errors.New("setup supports a single local Ollama host; update config to use one host")
			}
			if err := ensureOllamaWithStart(ctx, appConfig.Embeddings.OllamaHost, cmd.OutOrStdout(), true); err != nil {
				return err
			}
			models := []string{appConfig.Embeddings.Model, appConfig.LLM.Model}
			if err := pullOllamaModels(ctx, cmd.OutOrStdout(), models); err != nil {
				return err
			}
			fmt.Fprintln(cmd.OutOrStdout(), "Setup complete.")
			return nil
		},
	}

	return cmd
}

func pullOllamaModels(ctx context.Context, out io.Writer, models []string) error {
	seen := make(map[string]struct{})
	for _, model := range models {
		trimmed := strings.TrimSpace(model)
		if trimmed == "" {
			continue
		}
		if _, ok := seen[trimmed]; ok {
			continue
		}
		seen[trimmed] = struct{}{}
		if err := pullOllamaModel(ctx, out, trimmed); err != nil {
			return err
		}
	}
	return nil
}
