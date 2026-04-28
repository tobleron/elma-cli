package cmd

import (
	"fmt"
	"os"

	"pdfrag/config"
	"pdfrag/logging"

	"github.com/spf13/cobra"
	"go.uber.org/zap"
)

var (
	version = "dev"
	commit  = "none"
	date    = "unknown"
)

var showVersion bool
var configPath string
var appConfig config.Config
var loggerInitialized bool
var logSync func() error
var startOllama = true
var noStartOllama bool

func buildVersionString() string {
	return fmt.Sprintf("pdfrag %s (%s) %s", version, commit, date)
}

// NewRootCmd builds the base command when invoked from main.
func NewRootCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:          "pdfrag",
		Short:        "PDF knowledge base query system",
		Long:         "pdfrag indexes PDFs and enables semantic search with citations.",
		SilenceUsage: true,
		PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
			cfg, err := config.Load(configPath)
			if err != nil {
				return err
			}
			appConfig = cfg
			if !loggerInitialized {
				logger, syncFn, err := logging.Init(logging.Options{
					FilePath:    appConfig.Logging.File,
					Level:       appConfig.Logging.Level,
					Service:     "pdfrag",
					Environment: logging.CurrentEnvironment(),
				})
				if err != nil {
					return err
				}
				loggerInitialized = true
				logSync = syncFn
				logger.Info("logging initialized",
					zap.String("event", "logging_initialized"),
					zap.String("log_file", appConfig.Logging.File),
					zap.String("level", appConfig.Logging.Level),
				)
				logger.Info("config loaded",
					zap.String("event", "config_loaded"),
					zap.String("config_path", configPath),
				)
			}
			startOllama = appConfig.Ollama.AutoStart && !noStartOllama
			return nil
		},
		RunE: func(cmd *cobra.Command, args []string) error {
			if showVersion {
				fmt.Fprintln(cmd.OutOrStdout(), buildVersionString())
				return nil
			}
			return cmd.Help()
		},
	}

	cmd.PersistentFlags().BoolVar(&showVersion, "version", false, "show version information")
	cmd.PersistentFlags().StringVar(&configPath, "config", config.DefaultConfigPath(), "config file path")
	cmd.PersistentFlags().BoolVar(&noStartOllama, "no-start-ollama", false, "disable auto-starting Ollama")

	cmd.AddCommand(
		newSetupCmd(),
		newResetDBCmd(),
		newIndexCmd(),
		newReindexCmd(),
		newQueryCmd(),
		newRelatedCmd(),
		newInteractiveCmd(),
		newListCmd(),
		newInfoCmd(),
		newDeleteCmd(),
		newStatsCmd(),
		newDashCmd(),
		newExportCmd(),
		newImportCmd(),
	)

	return cmd
}

// Execute runs the CLI.
func Execute() {
	if err := NewRootCmd().Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		if logSync != nil {
			_ = logSync()
		}
		os.Exit(1)
	}
	if logSync != nil {
		_ = logSync()
	}
}
