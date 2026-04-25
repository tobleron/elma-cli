package cmd

import (
	"fmt"
	"strings"

	"github.com/spf13/cobra"
)

func newQueryCmd() *cobra.Command {
	var topK int
	var minSimilarity float64
	var format string
	var quiet bool
	var verbose bool

	cmd := &cobra.Command{
		Use:          "query <question>",
		Short:        "Query indexed PDFs",
		Args:         cobra.ExactArgs(1),
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			question := strings.TrimSpace(args[0])
			if question == "" {
				return fmt.Errorf("query cannot be empty")
			}
			resolvedFormat, err := resolveOutputFormat(format)
			if err != nil {
				return err
			}
			resolvedQuiet := quiet
			if !cmd.Flags().Changed("quiet") {
				resolvedQuiet = appConfig.Output.Quiet
			}
			resolvedVerbose := verbose
			if !cmd.Flags().Changed("verbose") {
				resolvedVerbose = appConfig.Output.Verbose
			}
			if resolvedQuiet {
				resolvedVerbose = false
			}
			resolvedTopK, resolvedMin := resolveSearchOptions(topK, minSimilarity)

			answer, results, err := performQuery(cmd.Context(), question, searchOptions{
				TopK:          resolvedTopK,
				MinSimilarity: resolvedMin,
			})
			if err != nil {
				return err
			}
			return writeQueryOutput(cmd.OutOrStdout(), question, answer, results, QueryOutputOptions{
				Format:        resolvedFormat,
				Quiet:         resolvedQuiet,
				Verbose:       resolvedVerbose,
				ColorEnabled:  supportsColor(cmd.OutOrStdout()),
				TopK:          resolvedTopK,
				MinSimilarity: resolvedMin,
			})
		},
	}

	cmd.Flags().IntVar(&topK, "top-k", 0, "maximum number of results to return")
	cmd.Flags().Float64Var(&minSimilarity, "min-similarity", -1, "minimum similarity threshold (0-1)")
	cmd.Flags().StringVar(&format, "format", "", "output format (rich, json, markdown)")
	cmd.Flags().BoolVar(&quiet, "quiet", false, "print answer only")
	cmd.Flags().BoolVar(&verbose, "verbose", false, "include extra metadata in output")

	return cmd
}
