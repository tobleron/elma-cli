package cmd

import (
	"github.com/spf13/cobra"
)

func newInteractiveCmd() *cobra.Command {
	var format string
	var quiet bool
	var verbose bool
	var topK int
	var minSimilarity float64

	cmd := &cobra.Command{
		Use:          "interactive",
		Short:        "Start an interactive session",
		Args:         cobra.NoArgs,
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			opts := interactiveOptions{
				Format:        format,
				Quiet:         quiet,
				Verbose:       verbose,
				TopK:          topK,
				MinSimilarity: minSimilarity,
				FormatSet:     cmd.Flags().Changed("format"),
				QuietSet:      cmd.Flags().Changed("quiet"),
				VerboseSet:    cmd.Flags().Changed("verbose"),
				TopKSet:       cmd.Flags().Changed("top-k"),
				MinSet:        cmd.Flags().Changed("min-similarity"),
			}
			return runInteractive(cmd.Context(), cmd.InOrStdin(), cmd.OutOrStdout(), cmd.ErrOrStderr(), opts)
		},
	}

	cmd.Flags().StringVar(&format, "format", "", "output format (rich, json, markdown)")
	cmd.Flags().BoolVar(&quiet, "quiet", false, "print answer only")
	cmd.Flags().BoolVar(&verbose, "verbose", false, "include extra metadata in output")
	cmd.Flags().IntVar(&topK, "top-k", 0, "maximum number of results to return")
	cmd.Flags().Float64Var(&minSimilarity, "min-similarity", -1, "minimum similarity threshold (0-1)")

	return cmd
}
