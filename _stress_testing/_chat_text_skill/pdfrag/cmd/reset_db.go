package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/spf13/cobra"
)

func newResetDBCmd() *cobra.Command {
	var keepMarkdown bool
	var keepCheckpoint bool

	cmd := &cobra.Command{
		Use:          "reset-db",
		Short:        "Delete the database and indexing artifacts",
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			if err := removeDBFile(cmd, appConfig.Database.Path); err != nil {
				return err
			}
			if !keepMarkdown {
				if err := removePath(cmd, "./output/markdown", true); err != nil {
					return err
				}
			}
			if !keepCheckpoint {
				if err := removePath(cmd, "./output/index.checkpoint.json", false); err != nil {
					return err
				}
			}
			fmt.Fprintln(cmd.OutOrStdout(), "Database reset complete.")
			return nil
		},
	}

	cmd.Flags().BoolVar(&keepMarkdown, "keep-markdown", false, "keep generated markdown output")
	cmd.Flags().BoolVar(&keepCheckpoint, "keep-checkpoint", false, "keep indexing checkpoint file")

	return cmd
}

func removeDBFile(cmd *cobra.Command, path string) error {
	if strings.TrimSpace(path) == "" {
		return fmt.Errorf("database path is empty")
	}
	cleaned := filepath.Clean(path)
	if cleaned == "." || cleaned == string(filepath.Separator) {
		return fmt.Errorf("refusing to delete database at %q", cleaned)
	}
	info, err := os.Stat(cleaned)
	if err != nil {
		if os.IsNotExist(err) {
			fmt.Fprintf(cmd.OutOrStdout(), "Database file not found: %s\n", cleaned)
			return nil
		}
		return err
	}
	if info.IsDir() {
		return fmt.Errorf("database path is a directory: %s", cleaned)
	}
	if err := os.Remove(cleaned); err != nil {
		return err
	}
	fmt.Fprintf(cmd.OutOrStdout(), "Deleted database: %s\n", cleaned)
	return nil
}

func removePath(cmd *cobra.Command, path string, isDir bool) error {
	cleaned := filepath.Clean(path)
	if cleaned == "." || cleaned == string(filepath.Separator) {
		return fmt.Errorf("refusing to delete path %q", cleaned)
	}
	if isDir {
		if err := os.RemoveAll(cleaned); err != nil {
			if os.IsNotExist(err) {
				return nil
			}
			return err
		}
		fmt.Fprintf(cmd.OutOrStdout(), "Deleted directory: %s\n", cleaned)
		return nil
	}
	if err := os.Remove(cleaned); err != nil {
		if os.IsNotExist(err) {
			return nil
		}
		return err
	}
	fmt.Fprintf(cmd.OutOrStdout(), "Deleted file: %s\n", cleaned)
	return nil
}
