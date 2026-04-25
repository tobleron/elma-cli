package cmd

import (
	"context"
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strconv"
	"strings"
	"time"

	"pdfrag/config"
	"pdfrag/storage"

	"github.com/chzyer/readline"
	"gopkg.in/yaml.v3"
)

type interactiveOptions struct {
	Format        string
	Quiet         bool
	Verbose       bool
	TopK          int
	MinSimilarity float64
	FormatSet     bool
	QuietSet      bool
	VerboseSet    bool
	TopKSet       bool
	MinSet        bool
}

type sessionEntry struct {
	Question string
	Answer   string
	Results  []storage.SearchResult
	Time     time.Time
}

type interactiveSession struct {
	ctx        context.Context
	out        io.Writer
	errOut     io.Writer
	opts       interactiveOptions
	history    []sessionEntry
	interrupts int
	lastSignal time.Time
}

func runInteractive(ctx context.Context, in io.Reader, out io.Writer, errOut io.Writer, opts interactiveOptions) error {
	historyFile := defaultHistoryPath()
	stdin := wrapReadCloser(in)
	rl, err := readline.NewEx(&readline.Config{
		Prompt:          "pdfrag> ",
		HistoryFile:     historyFile,
		InterruptPrompt: "",
		EOFPrompt:       "",
		Stdin:           stdin,
		Stdout:          out,
		Stderr:          errOut,
	})
	if err != nil {
		return err
	}
	defer rl.Close()

	session := &interactiveSession{
		ctx:    ctx,
		out:    out,
		errOut: errOut,
		opts:   opts,
	}

	fmt.Fprintln(out, "Interactive mode. Type /help for commands.")

	for {
		line, err := rl.Readline()
		if err != nil {
			if errors.Is(err, readline.ErrInterrupt) {
				if session.handleInterrupt() {
					return nil
				}
				continue
			}
			if errors.Is(err, io.EOF) {
				return nil
			}
			return err
		}
		session.resetInterrupt()
		trimmed := strings.TrimSpace(line)
		if trimmed == "" {
			continue
		}
		if strings.HasPrefix(trimmed, "/") {
			exit, cmdErr := session.handleCommand(trimmed)
			if cmdErr != nil {
				fmt.Fprintln(errOut, cmdErr)
			}
			if exit {
				return nil
			}
			continue
		}
		if err := session.handleQuery(trimmed); err != nil {
			fmt.Fprintln(errOut, err)
		}
	}
}

func (s *interactiveSession) handleInterrupt() bool {
	if time.Since(s.lastSignal) > 5*time.Second {
		s.interrupts = 0
	}
	s.interrupts++
	s.lastSignal = time.Now()
	if s.interrupts >= 2 {
		fmt.Fprintln(s.out, "Exiting.")
		return true
	}
	fmt.Fprintln(s.out, "Press Ctrl+C again to exit.")
	return false
}

func (s *interactiveSession) resetInterrupt() {
	s.interrupts = 0
	s.lastSignal = time.Time{}
}

func (s *interactiveSession) handleCommand(line string) (bool, error) {
	command := strings.TrimSpace(strings.TrimPrefix(line, "/"))
	if command == "" {
		return false, nil
	}
	fields := strings.Fields(command)
	name := strings.ToLower(fields[0])
	args := strings.TrimSpace(strings.TrimPrefix(command, fields[0]))

	switch name {
	case "help":
		printInteractiveHelp(s.out)
		return false, nil
	case "exit", "quit":
		fmt.Fprintln(s.out, "Exiting.")
		return true, nil
	case "clear":
		s.history = nil
		fmt.Fprintln(s.out, "History cleared.")
		return false, nil
	case "config":
		return false, printConfig(s.out)
	case "set":
		return false, s.handleSet(args)
	case "save":
		return false, s.handleSave(args)
	default:
		return false, fmt.Errorf("unknown command: /%s", name)
	}
}

func (s *interactiveSession) handleQuery(question string) error {
	queryOpts, outputOpts, err := s.resolveOptions()
	if err != nil {
		return err
	}
	answer, results, err := performQuery(s.ctx, question, queryOpts)
	if err != nil {
		return err
	}
	s.history = append(s.history, sessionEntry{
		Question: question,
		Answer:   answer,
		Results:  results,
		Time:     time.Now(),
	})
	return writeQueryOutput(s.out, question, answer, results, outputOpts)
}

func (s *interactiveSession) resolveOptions() (searchOptions, QueryOutputOptions, error) {
	resolvedFormat, err := resolveOutputFormat(s.opts.Format)
	if err != nil {
		return searchOptions{}, QueryOutputOptions{}, err
	}
	resolvedQuiet := s.opts.Quiet
	if !s.opts.QuietSet {
		resolvedQuiet = appConfig.Output.Quiet
	}
	resolvedVerbose := s.opts.Verbose
	if !s.opts.VerboseSet {
		resolvedVerbose = appConfig.Output.Verbose
	}
	if resolvedQuiet {
		resolvedVerbose = false
	}
	resolvedTopK, resolvedMin := resolveSearchOptions(s.opts.TopK, s.opts.MinSimilarity)
	if s.opts.TopKSet {
		resolvedTopK, _ = resolveSearchOptions(s.opts.TopK, -1)
	}
	if s.opts.MinSet {
		_, resolvedMin = resolveSearchOptions(0, s.opts.MinSimilarity)
	}
	queryOpts := searchOptions{
		TopK:          resolvedTopK,
		MinSimilarity: resolvedMin,
	}
	outputOpts := QueryOutputOptions{
		Format:        resolvedFormat,
		Quiet:         resolvedQuiet,
		Verbose:       resolvedVerbose,
		ColorEnabled:  supportsColor(s.out),
		TopK:          resolvedTopK,
		MinSimilarity: resolvedMin,
	}
	return queryOpts, outputOpts, nil
}

func (s *interactiveSession) handleSet(arg string) error {
	arg = strings.TrimSpace(arg)
	if arg == "" {
		return fmt.Errorf("usage: /set <key>=<value>")
	}
	key, value, err := parseSetArgument(arg)
	if err != nil {
		return err
	}
	if err := applyConfigUpdate(&appConfig, key, value); err != nil {
		return err
	}
	fmt.Fprintf(s.out, "Updated %s.\n", key)
	return nil
}

func (s *interactiveSession) handleSave(arg string) error {
	path := strings.TrimSpace(arg)
	if path == "" {
		path = "pdfrag-session.md"
	}
	if len(s.history) == 0 {
		return fmt.Errorf("no history to save")
	}
	if err := saveHistory(path, s.history); err != nil {
		return err
	}
	fmt.Fprintf(s.out, "Saved history to %s.\n", path)
	return nil
}

func printInteractiveHelp(w io.Writer) {
	fmt.Fprintln(w, "Commands:")
	fmt.Fprintln(w, "  /help              Show this help message")
	fmt.Fprintln(w, "  /config            Show current configuration")
	fmt.Fprintln(w, "  /set <key>=<value> Update a configuration value")
	fmt.Fprintln(w, "  /clear             Clear conversation history")
	fmt.Fprintln(w, "  /save [file]       Save conversation history (default pdfrag-session.md)")
	fmt.Fprintln(w, "  /exit              Exit interactive mode")
}

func printConfig(w io.Writer) error {
	data, err := yaml.Marshal(appConfig)
	if err != nil {
		return err
	}
	_, err = fmt.Fprintf(w, "%s\n", string(data))
	return err
}

func saveHistory(path string, history []sessionEntry) error {
	file, err := os.Create(path)
	if err != nil {
		return err
	}
	defer file.Close()

	fmt.Fprintf(file, "# pdfrag session - %s\n\n", time.Now().Format(time.RFC3339))
	for i, entry := range history {
		fmt.Fprintf(file, "## Q%d (%s)\n\n", i+1, entry.Time.Format(time.RFC3339))
		fmt.Fprintf(file, "**Question:** %s\n\n", entry.Question)
		fmt.Fprintf(file, "**Answer:** %s\n\n", entry.Answer)
		if len(entry.Results) > 0 {
			fmt.Fprintln(file, "**Sources:**")
			for _, result := range buildResultsOutput(entry.Results, QueryOutputOptions{}) {
				label := formatResultLabel(result)
				fmt.Fprintf(file, "- %s\n", label)
			}
			fmt.Fprintln(file)
		}
	}
	return nil
}

func parseSetArgument(arg string) (string, string, error) {
	if strings.Contains(arg, "=") {
		parts := strings.SplitN(arg, "=", 2)
		key := strings.TrimSpace(parts[0])
		value := strings.TrimSpace(parts[1])
		if key == "" || value == "" {
			return "", "", fmt.Errorf("usage: /set <key>=<value>")
		}
		return key, value, nil
	}
	fields := strings.Fields(arg)
	if len(fields) < 2 {
		return "", "", fmt.Errorf("usage: /set <key>=<value>")
	}
	key := fields[0]
	value := strings.TrimSpace(strings.TrimPrefix(arg, key))
	value = strings.TrimSpace(value)
	if value == "" {
		return "", "", fmt.Errorf("usage: /set <key>=<value>")
	}
	return key, value, nil
}

func applyConfigUpdate(cfg *config.Config, key, value string) error {
	key = strings.ToLower(strings.TrimSpace(key))
	switch key {
	case "database.path":
		cfg.Database.Path = value
	case "indexing.concurrency":
		parsed, err := strconv.Atoi(value)
		if err != nil {
			return fmt.Errorf("invalid integer for %s", key)
		}
		cfg.Indexing.Concurrency = parsed
	case "embeddings.model":
		cfg.Embeddings.Model = value
	case "embeddings.batch_size":
		parsed, err := strconv.Atoi(value)
		if err != nil {
			return fmt.Errorf("invalid integer for %s", key)
		}
		cfg.Embeddings.BatchSize = parsed
	case "embeddings.ollama_host":
		cfg.Embeddings.OllamaHost = value
	case "search.top_k":
		parsed, err := strconv.Atoi(value)
		if err != nil {
			return fmt.Errorf("invalid integer for %s", key)
		}
		cfg.Search.TopK = parsed
	case "search.min_similarity":
		parsed, err := strconv.ParseFloat(value, 64)
		if err != nil {
			return fmt.Errorf("invalid float for %s", key)
		}
		cfg.Search.MinSimilarity = parsed
	case "llm.model":
		cfg.LLM.Model = value
	case "llm.temperature":
		parsed, err := strconv.ParseFloat(value, 64)
		if err != nil {
			return fmt.Errorf("invalid float for %s", key)
		}
		cfg.LLM.Temperature = parsed
	case "llm.ollama_host":
		cfg.LLM.OllamaHost = value
	case "ollama.auto_start":
		parsed, err := strconv.ParseBool(value)
		if err != nil {
			return fmt.Errorf("invalid boolean for %s", key)
		}
		cfg.Ollama.AutoStart = parsed
	case "ollama.warm":
		parsed, err := strconv.ParseBool(value)
		if err != nil {
			return fmt.Errorf("invalid boolean for %s", key)
		}
		cfg.Ollama.Warm = parsed
	case "output.format":
		cfg.Output.Format = value
	case "output.quiet":
		parsed, err := strconv.ParseBool(value)
		if err != nil {
			return fmt.Errorf("invalid boolean for %s", key)
		}
		cfg.Output.Quiet = parsed
	case "output.verbose":
		parsed, err := strconv.ParseBool(value)
		if err != nil {
			return fmt.Errorf("invalid boolean for %s", key)
		}
		cfg.Output.Verbose = parsed
	default:
		return fmt.Errorf("unknown config key: %s", key)
	}
	return nil
}

func defaultHistoryPath() string {
	home, err := os.UserHomeDir()
	if err != nil || home == "" {
		return ""
	}
	path := filepath.Join(home, ".pdfrag", "history")
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return ""
	}
	return path
}

func wrapReadCloser(r io.Reader) io.ReadCloser {
	if rc, ok := r.(io.ReadCloser); ok {
		return rc
	}
	return io.NopCloser(r)
}
