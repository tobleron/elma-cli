package cmd

import (
	"bufio"
	"context"
	"fmt"
	"io"
	"os"
	"os/exec"
	"regexp"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/charmbracelet/bubbles/cursor"
	"github.com/charmbracelet/bubbles/textarea"
	"github.com/charmbracelet/bubbles/viewport"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/google/shlex"
	"github.com/muesli/reflow/wrap"
	"github.com/spf13/cobra"

	"pdfrag/llm"
)

const (
	dashMaxLines                  = 400
	dashHelpLine                  = "Enter command (e.g., query \"...\"), ↑/↓ scroll, End bottom, Ctrl+C quit"
	dashLeftTitle                 = "ollama serve"
	dashRightTitle                = "pdfrag commands"
	dashInputHeight               = 3
	warmDefaultEmbeddingsEstimate = 60 * time.Second
	warmDefaultChatEstimate       = 120 * time.Second
	warmMinEmbeddingsTimeout      = 60 * time.Second
	warmMinChatTimeout            = 120 * time.Second
	warmMaxTimeout                = 5 * time.Minute
	warmWaitTimeout               = 30 * time.Second
)

var (
	titleActiveStyle   = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("230")).Background(lipgloss.Color("63")).Padding(0, 1)
	titleInactiveStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("244")).Background(lipgloss.Color("236")).Padding(0, 1)
	statusStyle        = lipgloss.NewStyle().Foreground(lipgloss.Color("244"))
	commandStyle       = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("252"))
	warmStyle          = lipgloss.NewStyle().Foreground(lipgloss.Color("244"))
	metaStyle          = lipgloss.NewStyle().Foreground(lipgloss.Color("244"))
	sourceStyle        = lipgloss.NewStyle().Foreground(lipgloss.Color("244"))
	errorStyle         = lipgloss.NewStyle().Bold(true).Foreground(lipgloss.Color("203"))
	inputBaseStyle     = lipgloss.NewStyle()
)

var (
	inputPromptStyle      = lipgloss.NewStyle().Foreground(lipgloss.Color("244"))
	inputTextStyle        = lipgloss.NewStyle().Foreground(lipgloss.Color("252"))
	inputPlaceholderStyle = lipgloss.NewStyle().Foreground(lipgloss.Color("240"))
)

func newDashCmd() *cobra.Command {
	cmd := &cobra.Command{
		Use:          "dash",
		Short:        "Open a TUI dashboard with Ollama and pdfrag panes",
		SilenceUsage: true,
		RunE: func(cmd *cobra.Command, args []string) error {
			model, err := newDashModel()
			if err != nil {
				return err
			}
			program := tea.NewProgram(model, tea.WithAltScreen())
			_, err = program.Run()
			return err
		},
	}

	return cmd
}

type dashModel struct {
	width  int
	height int

	leftLines  []string
	rightLines []string

	leftViewport  viewport.Model
	rightViewport viewport.Model
	activeRight   bool
	showLeftOnly  bool
	showRightOnly bool
	pauseLeftLogs bool
	leftFollow    bool
	rightFollow   bool
	dirtyLeft     bool
	dirtyRight    bool

	leftCh  chan string
	rightCh chan string
	warmCh  chan tea.Msg

	input textarea.Model

	workingDir string
	execPath   string

	ollamaCmd *exec.Cmd
	ollamaMu  sync.Mutex

	rightRunning bool

	warmingChat bool
	warmStatus  string

	ollamaStatus string
}

type leftLineMsg string
type rightLineMsg string
type rightResultMsg struct {
	lines    []string
	err      error
	duration time.Duration
}
type ollamaStatusMsg string
type warmupStartMsg struct {
	stage    string
	estimate time.Duration
	timeout  time.Duration
}
type warmupStageDoneMsg struct {
	stage    string
	duration time.Duration
	err      error
}
type warmupDoneMsg struct {
	duration time.Duration
}
type warmupSkipMsg struct {
	reason string
}

func newDashModel() (dashModel, error) {
	wd, err := os.Getwd()
	if err != nil {
		return dashModel{}, err
	}
	execPath, err := os.Executable()
	if err != nil {
		return dashModel{}, err
	}
	input := textarea.New()
	input.Prompt = ""
	input.Placeholder = "Ask a question (no need to type query)"
	input.ShowLineNumbers = false
	input.CharLimit = 0
	input.SetHeight(dashInputHeight)
	input.FocusedStyle = textarea.Style{
		Base:        inputBaseStyle,
		Prompt:      inputPromptStyle,
		Text:        inputTextStyle,
		Placeholder: inputPlaceholderStyle,
		CursorLine:  lipgloss.NewStyle(),
	}
	input.BlurredStyle = textarea.Style{
		Base:        inputBaseStyle,
		Prompt:      inputPromptStyle,
		Text:        inputTextStyle,
		Placeholder: inputPlaceholderStyle,
		CursorLine:  lipgloss.NewStyle(),
	}
	input.KeyMap.InsertNewline.SetEnabled(false)
	_ = input.Cursor.SetMode(cursor.CursorStatic)
	_ = input.Focus()

	return dashModel{
		leftLines:     []string{},
		rightLines:    []string{"Type a pdfrag command (without './pdfrag')."},
		leftCh:        make(chan string, 128),
		rightCh:       make(chan string, 128),
		warmCh:        make(chan tea.Msg, 16),
		input:         input,
		workingDir:    wd,
		execPath:      execPath,
		activeRight:   true,
		showRightOnly: true,
		leftFollow:    true,
		rightFollow:   true,
		dirtyLeft:     true,
		dirtyRight:    true,
		ollamaStatus:  "checking",
	}, nil
}

func (m dashModel) Init() tea.Cmd {
	startCmd := maybeStartOllama(&m)
	warmCmd := startWarmupCmd(m.warmCh)
	return tea.Batch(startCmd, warmCmd, listenRight(m.rightCh), listenWarm(m.warmCh))
}

func listenLeft(ch <-chan string) tea.Cmd {
	return func() tea.Msg {
		return leftLineMsg(<-ch)
	}
}

func listenRight(ch <-chan string) tea.Cmd {
	return func() tea.Msg {
		return rightLineMsg(<-ch)
	}
}

func listenWarm(ch <-chan tea.Msg) tea.Cmd {
	return func() tea.Msg {
		return <-ch
	}
}

func (m dashModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.KeyMsg:
		switch msg.String() {
		case "ctrl+c":
			m.stopOllama()
			return m, tea.Quit
		case "end":
			if m.activeRight {
				m.rightFollow = true
				m.rightViewport.GotoBottom()
			} else {
				m.leftFollow = true
				m.leftViewport.GotoBottom()
			}
		case "enter":
			line := strings.TrimSpace(m.input.Value())
			m.input.SetValue("")
			if line == "" {
				return m, nil
			}
			if strings.EqualFold(line, "exit") || strings.EqualFold(line, "quit") {
				m.stopOllama()
				return m, tea.Quit
			}
			line = normalizeDashInput(line)
			if m.rightRunning {
				m.appendRightLine("A command is already running. Please wait.")
				return m, nil
			}
			m.rightFollow = true
			m.appendRightLine("> " + line)
			m.rightRunning = true
			return m, m.runPdfragCommand(line)
		case "up":
			if m.activeRight {
				m.rightFollow = false
				m.rightViewport.LineUp(1)
			} else {
				m.leftFollow = false
				m.leftViewport.LineUp(1)
			}
		case "down":
			if m.activeRight {
				m.rightViewport.LineDown(1)
				if m.rightViewport.AtBottom() {
					m.rightFollow = true
				}
			} else {
				m.leftViewport.LineDown(1)
				if m.leftViewport.AtBottom() {
					m.leftFollow = true
				}
			}
		case "pgup":
			if m.activeRight {
				m.rightFollow = false
				m.rightViewport.LineUp(m.rightViewport.Height)
			} else {
				m.leftFollow = false
				m.leftViewport.LineUp(m.leftViewport.Height)
			}
		case "pgdown":
			if m.activeRight {
				m.rightViewport.LineDown(m.rightViewport.Height)
				if m.rightViewport.AtBottom() {
					m.rightFollow = true
				}
			} else {
				m.leftViewport.LineDown(m.leftViewport.Height)
				if m.leftViewport.AtBottom() {
					m.leftFollow = true
				}
			}
		}
	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.updateViewports()
		m.dirtyLeft = true
		m.dirtyRight = true
	case leftLineMsg:
		if !m.pauseLeftLogs {
			m.appendLeftLine(string(msg))
		}
		return m, listenLeft(m.leftCh)
	case rightLineMsg:
		m.appendRightLine(string(msg))
		return m, listenRight(m.rightCh)
	case rightResultMsg:
		if len(msg.lines) > 0 {
			for _, line := range msg.lines {
				m.appendRightLine(line)
			}
		}
		if msg.err != nil {
			m.appendRightLine(fmt.Sprintf("command failed: %v", msg.err))
		}
		if msg.duration > 0 {
			m.appendRightLine(fmt.Sprintf("Command finished in %s.", formatElapsed(msg.duration)))
		}
		m.rightRunning = false
	case ollamaStatusMsg:
		m.ollamaStatus = string(msg)
	case warmupStartMsg:
		m.warmingChat = true
		estimate := msg.estimate
		if estimate <= 0 {
			estimate = msg.timeout
		}
		m.warmStatus = fmt.Sprintf("Warm-up: %s (avg %s, timeout %s)", msg.stage, formatElapsed(estimate), formatElapsed(msg.timeout))
		m.appendRightLine(fmt.Sprintf("Warming up Ollama %s (avg %s, timeout %s)...", msg.stage, formatElapsed(estimate), formatElapsed(msg.timeout)))
		return m, listenWarm(m.warmCh)
	case warmupStageDoneMsg:
		if msg.err != nil {
			m.appendRightLine(fmt.Sprintf("Warm-up %s failed after %s: %v", msg.stage, formatElapsed(msg.duration), msg.err))
		} else {
			m.appendRightLine(fmt.Sprintf("Warm-up %s complete in %s.", msg.stage, formatElapsed(msg.duration)))
		}
		return m, listenWarm(m.warmCh)
	case warmupSkipMsg:
		m.appendRightLine(fmt.Sprintf("Warm-up skipped: %s", msg.reason))
		m.warmingChat = false
		m.warmStatus = ""
		return m, listenWarm(m.warmCh)
	case warmupDoneMsg:
		if msg.duration > 0 {
			m.appendRightLine(fmt.Sprintf("Warm-up complete in %s.", formatElapsed(msg.duration)))
		} else {
			m.appendRightLine("Warm-up complete.")
		}
		m.warmingChat = false
		m.warmStatus = ""
		return m, listenWarm(m.warmCh)
	}

	var cmd tea.Cmd
	m.input, cmd = m.input.Update(msg)
	return m, cmd
}

func (m dashModel) View() string {
	if m.width == 0 || m.height == 0 {
		return "Loading..."
	}

	m.updateViewports()
	if m.dirtyLeft {
		m.leftViewport.SetContent(renderWrapped(m.leftLines, m.leftViewport.Width, nil))
		if m.leftFollow {
			m.leftViewport.GotoBottom()
		}
		m.dirtyLeft = false
	}
	if m.dirtyRight {
		m.rightViewport.SetContent(renderWrapped(m.rightLines, m.rightViewport.Width, styleForRightLine))
		if m.rightFollow {
			m.rightViewport.GotoBottom()
		}
		m.dirtyRight = false
	}

	status := m.ollamaStatus
	if status == "" {
		status = "unknown"
	}
	runStatus := ""
	if m.rightRunning {
		runStatus = "Running"
	}
	warmStatus := m.warmStatus
	rightTitle := titleStyle(m.activeRight).Render(dashRightTitle)
	model := strings.TrimSpace(appConfig.LLM.Model)
	if model == "" {
		model = "unknown"
	}
	provider := strings.TrimSpace(appConfig.LLM.Provider)
	if provider == "" {
		provider = llm.ProviderOllama
	}
	helpText := fmt.Sprintf("Ollama: %s | Provider: %s | Model: %s", status, provider, model)
	if runStatus != "" {
		helpText = fmt.Sprintf("%s | %s", helpText, runStatus)
	}
	if warmStatus != "" {
		helpText = fmt.Sprintf("%s | %s", helpText, warmStatus)
	}
	helpText = fmt.Sprintf("%s | %s", helpText, dashHelpLine)
	helpLine := statusStyle.Render(helpText)

	rightPaneHeight := m.rightViewport.Height + dashInputHeight + 2
	rightPane := lipgloss.NewStyle().Width(m.rightViewport.Width).Height(rightPaneHeight).Render(
		rightTitle + "\n" + m.rightViewport.View() + "\n" + m.input.View() + "\n" + helpLine,
	)

	return rightPane
}

func (m *dashModel) updateViewports() {
	if m.width <= 0 || m.height <= 0 {
		return
	}
	leftWidth := m.width / 2
	rightWidth := m.width - leftWidth
	if m.showLeftOnly {
		leftWidth = m.width
		rightWidth = m.width
	}
	if m.showRightOnly {
		leftWidth = m.width
		rightWidth = m.width
	}
	leftHeight := m.height - 1
	if leftHeight < 1 {
		leftHeight = 1
	}
	rightHeight := m.height - (dashInputHeight + 2)
	if rightHeight < 1 {
		rightHeight = 1
	}
	m.leftViewport.Width = leftWidth
	m.leftViewport.Height = leftHeight
	m.rightViewport.Width = rightWidth
	m.rightViewport.Height = rightHeight
	if rightWidth > 0 {
		inputWidth := rightWidth - 2
		if inputWidth < 10 {
			inputWidth = rightWidth
		}
		if inputWidth < 1 {
			inputWidth = 1
		}
		m.input.SetWidth(inputWidth)
		m.input.SetHeight(dashInputHeight)
	}
}

func renderWrapped(lines []string, width int, styleFn func(string) (lipgloss.Style, bool)) string {
	if width <= 0 {
		return strings.Join(lines, "\n")
	}
	var b strings.Builder
	for i, line := range lines {
		if i > 0 {
			b.WriteByte('\n')
		}
		wrapped := wrap.String(line, width)
		if styleFn != nil {
			if style, ok := styleFn(line); ok {
				b.WriteString(style.Render(wrapped))
				continue
			}
		}
		b.WriteString(wrapped)
	}
	return b.String()
}

func titleStyle(active bool) lipgloss.Style {
	if active {
		return titleActiveStyle
	}
	return titleInactiveStyle
}

func tailLines(lines []string, max int) []string {
	if max <= 0 || len(lines) <= max {
		return lines
	}
	return lines[len(lines)-max:]
}

func (m *dashModel) appendLeftLine(line string) {
	m.leftLines = appendLine(m.leftLines, line)
	m.dirtyLeft = true
}

func (m *dashModel) appendRightLine(line string) {
	m.rightLines = appendLine(m.rightLines, line)
	m.dirtyRight = true
}

func appendLine(lines []string, line string) []string {
	line = strings.TrimRight(line, "\r\n")
	lines = append(lines, line)
	if len(lines) > dashMaxLines {
		lines = lines[len(lines)-dashMaxLines:]
	}
	return lines
}

func maybeStartOllama(m *dashModel) tea.Cmd {
	host := appConfig.Embeddings.OllamaHost
	llmProvider := strings.TrimSpace(strings.ToLower(appConfig.LLM.Provider))
	if llmProvider == "" {
		llmProvider = llm.ProviderOllama
	}
	if llmProvider == llm.ProviderOllama && appConfig.LLM.OllamaHost != "" {
		host = appConfig.LLM.OllamaHost
	}
	if host == "" {
		host = "http://localhost:11434"
	}
	parsed, normalized, err := parseOllamaHost(host)
	if err != nil {
		m.rightCh <- fmt.Sprintf("invalid ollama host: %v", err)
		return statusCmd("invalid host")
	}
	if ollamaReachable(context.Background(), normalized) {
		m.rightCh <- "Ollama already running at " + normalized
		return statusCmd("ready")
	}
	if !appConfig.Ollama.AutoStart {
		m.rightCh <- "Ollama not reachable and auto_start=false"
		return statusCmd("disabled")
	}
	if !isLocalHost(parsed) {
		m.rightCh <- "Ollama host is non-local; not starting local server."
		return statusCmd("non-local")
	}
	path, err := exec.LookPath("ollama")
	if err != nil {
		m.rightCh <- "ollama not found in PATH"
		return statusCmd("missing")
	}
	cmd := exec.Command(path, "serve")
	cmd.Stdout = io.Discard
	cmd.Stderr = io.Discard
	if err := cmd.Start(); err != nil {
		m.rightCh <- fmt.Sprintf("ollama serve failed: %v", err)
		return statusCmd("error")
	}
	m.ollamaMu.Lock()
	m.ollamaCmd = cmd
	m.ollamaMu.Unlock()
	m.rightCh <- "Starting Ollama..."
	go func() {
		err := cmd.Wait()
		if err != nil {
			m.rightCh <- fmt.Sprintf("ollama exited: %v", err)
		} else {
			m.rightCh <- "ollama exited"
		}
	}()
	return tea.Batch(statusCmd("starting"), waitOllamaReadyCmd(normalized))
}

func waitOllamaReadyCmd(host string) tea.Cmd {
	return func() tea.Msg {
		ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
		defer cancel()
		if err := waitForOllama(ctx, host, 30*time.Second); err != nil {
			return ollamaStatusMsg("unavailable")
		}
		return ollamaStatusMsg("ready")
	}
}

func (m *dashModel) stopOllama() {
	m.ollamaMu.Lock()
	defer m.ollamaMu.Unlock()
	if m.ollamaCmd == nil || m.ollamaCmd.Process == nil {
		return
	}
	_ = m.ollamaCmd.Process.Signal(os.Interrupt)
}

func statusCmd(status string) tea.Cmd {
	return func() tea.Msg {
		return ollamaStatusMsg(status)
	}
}

func (m *dashModel) runPdfragCommand(line string) tea.Cmd {
	args, err := shlex.Split(line)
	if err != nil {
		m.rightRunning = false
		m.appendRightLine(fmt.Sprintf("parse error: %v", err))
		return nil
	}
	if len(args) == 0 {
		m.rightRunning = false
		return nil
	}
	if args[0] == "pdfrag" || args[0] == "./pdfrag" {
		args = args[1:]
	}
	if len(args) == 0 {
		m.rightRunning = false
		return nil
	}
	cmd := exec.Command(m.execPath, args...)
	cmd.Dir = m.workingDir
	return func() tea.Msg {
		start := time.Now()
		output, err := cmd.CombinedOutput()
		lines := splitOutputLines(output)
		return rightResultMsg{
			lines:    lines,
			err:      err,
			duration: time.Since(start),
		}
	}
}

func formatElapsed(d time.Duration) string {
	if d < 0 {
		d = 0
	}
	if d < time.Minute {
		return fmt.Sprintf("%ds", int(d.Seconds()))
	}
	minutes := int(d.Minutes())
	seconds := int(d.Seconds()) % 60
	return fmt.Sprintf("%dm%02ds", minutes, seconds)
}

func startWarmupCmd(warmCh chan<- tea.Msg) tea.Cmd {
	if !appConfig.Ollama.Warm {
		return nil
	}
	return func() tea.Msg {
		go runWarmup(warmCh)
		return nil
	}
}

func runWarmup(warmCh chan<- tea.Msg) {
	totalStart := time.Now()
	stats, statsErr := loadWarmupStats()
	statsUpdated := false
	embedHost := appConfig.Embeddings.OllamaHost
	embedModel := appConfig.Embeddings.Model
	_, embedNormalized, err := parseOllamaHost(embedHost)
	if err != nil {
		warmCh <- warmupSkipMsg{reason: fmt.Sprintf("invalid embeddings host: %v", err)}
		warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
		return
	}

	ctxWait, cancelWait := context.WithTimeout(context.Background(), warmWaitTimeout)
	err = waitForOllama(ctxWait, embedNormalized, warmWaitTimeout)
	cancelWait()
	if err != nil {
		warmCh <- warmupSkipMsg{reason: fmt.Sprintf("Ollama unavailable (%v)", err)}
		warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
		return
	}

	embedEstimate := warmDefaultEmbeddingsEstimate
	if statsErr == nil {
		embedEstimate = stats.estimate("embeddings", warmDefaultEmbeddingsEstimate)
	}
	embedTimeout := clampDuration(embedEstimate*2, warmMinEmbeddingsTimeout, warmMaxTimeout)
	warmCh <- warmupStartMsg{stage: "embeddings", estimate: embedEstimate, timeout: embedTimeout}
	ctxEmb, cancelEmb := context.WithTimeout(context.Background(), embedTimeout)
	start := time.Now()
	err = warmOllamaEmbeddings(ctxEmb, embedHost, embedModel)
	cancelEmb()
	warmCh <- warmupStageDoneMsg{stage: "embeddings", duration: time.Since(start), err: err}
	if err != nil {
		warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
		return
	}
	stats.addSample("embeddings", time.Since(start))
	statsUpdated = true

	if appConfig.LLM.OllamaHost != "" {
		provider := strings.TrimSpace(strings.ToLower(appConfig.LLM.Provider))
		if provider == "" {
			provider = llm.ProviderOllama
		}
		if provider != llm.ProviderOllama {
			warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
			return
		}
		chatHost := appConfig.LLM.OllamaHost
		_, chatNormalized, err := parseOllamaHost(chatHost)
		if err != nil {
			warmCh <- warmupSkipMsg{reason: fmt.Sprintf("invalid chat host: %v", err)}
			warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
			return
		}
		if chatNormalized != embedNormalized {
			ctxWaitChat, cancelWaitChat := context.WithTimeout(context.Background(), warmWaitTimeout)
			err = waitForOllama(ctxWaitChat, chatNormalized, warmWaitTimeout)
			cancelWaitChat()
			if err != nil {
				warmCh <- warmupSkipMsg{reason: fmt.Sprintf("chat host unavailable (%v)", err)}
				warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
				return
			}
		}
		chatEstimate := warmDefaultChatEstimate
		if statsErr == nil {
			chatEstimate = stats.estimate("chat", warmDefaultChatEstimate)
		}
		chatTimeout := clampDuration(chatEstimate*2, warmMinChatTimeout, warmMaxTimeout)
		warmCh <- warmupStartMsg{stage: "chat", estimate: chatEstimate, timeout: chatTimeout}
		ctxChat, cancelChat := context.WithTimeout(context.Background(), chatTimeout)
		start = time.Now()
		err = warmOllamaChat(ctxChat, chatHost, appConfig.LLM.Model, appConfig.LLM.Temperature)
		cancelChat()
		warmCh <- warmupStageDoneMsg{stage: "chat", duration: time.Since(start), err: err}
		if err == nil {
			stats.addSample("chat", time.Since(start))
			statsUpdated = true
		}
	}

	if statsUpdated && statsErr == nil {
		_ = saveWarmupStats(stats)
	}
	warmCh <- warmupDoneMsg{duration: time.Since(totalStart)}
}

var ansiEscape = regexp.MustCompile(`\x1b\[[0-9;]*[A-Za-z]`)

func sanitizeLine(line string) string {
	line = strings.ReplaceAll(line, "\r", "")
	line = ansiEscape.ReplaceAllString(line, "")
	line = strings.Map(func(r rune) rune {
		if r < 32 && r != '\t' {
			return -1
		}
		return r
	}, line)
	return line
}

func splitOutputLines(output []byte) []string {
	if len(output) == 0 {
		return nil
	}
	scanner := bufio.NewScanner(strings.NewReader(string(output)))
	buf := make([]byte, 0, 64*1024)
	scanner.Buffer(buf, 1024*1024)
	lines := make([]string, 0, 32)
	for scanner.Scan() {
		lines = append(lines, sanitizeLine(scanner.Text()))
	}
	return lines
}

func styleForRightLine(line string) (lipgloss.Style, bool) {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" {
		return lipgloss.Style{}, false
	}
	lower := strings.ToLower(trimmed)

	switch {
	case strings.HasPrefix(trimmed, "> "):
		return commandStyle, true
	case strings.HasPrefix(trimmed, "Answer:"),
		strings.HasPrefix(trimmed, "Query:"):
		return lipgloss.Style{}, false
	case strings.HasPrefix(trimmed, "Sources:"):
		return sourceStyle, true
	case strings.HasPrefix(trimmed, "Warm-up") || strings.HasPrefix(trimmed, "Warming up"):
		return warmStyle, true
	case strings.HasPrefix(trimmed, "Source:"),
		strings.HasPrefix(trimmed, "Sources:"),
		strings.HasPrefix(trimmed, "[") && strings.Contains(trimmed, ".pdf"):
		return sourceStyle, true
	case strings.HasPrefix(lower, "command failed"),
		strings.HasPrefix(lower, "warm-up") && strings.Contains(lower, "failed"):
		return errorStyle, true
	case strings.HasPrefix(trimmed, "Command finished"),
		strings.HasPrefix(trimmed, "Ollama"),
		strings.HasPrefix(trimmed, "Starting Ollama"),
		strings.HasPrefix(trimmed, "Type a pdfrag command"):
		return metaStyle, true
	default:
		return lipgloss.Style{}, false
	}
}

func normalizeDashInput(line string) string {
	trimmed := strings.TrimSpace(line)
	if trimmed == "" {
		return ""
	}
	if strings.HasPrefix(trimmed, "/") {
		raw := strings.TrimSpace(strings.TrimPrefix(trimmed, "/"))
		if raw == "" {
			return ""
		}
		return raw
	}
	if isDashCommandLine(trimmed) {
		return trimmed
	}
	question := unquoteIfQuoted(trimmed)
	if question == "" {
		question = trimmed
	}
	return fmt.Sprintf("query %s", strconv.Quote(question))
}

func isDashCommandLine(line string) bool {
	args, err := shlex.Split(line)
	if err != nil || len(args) == 0 {
		return false
	}
	cmd := strings.ToLower(args[0])
	if cmd == "pdfrag" || cmd == "./pdfrag" {
		if len(args) < 2 {
			return false
		}
		cmd = strings.ToLower(args[1])
	}
	switch cmd {
	case "query", "index", "reindex", "info", "list", "delete", "stats", "export", "import", "setup", "reset-db", "related", "interactive", "dash", "help", "-h", "--help":
		return true
	default:
		return false
	}
}

func unquoteIfQuoted(s string) string {
	if len(s) < 2 {
		return s
	}
	if (s[0] == '"' && s[len(s)-1] == '"') || (s[0] == '\'' && s[len(s)-1] == '\'') {
		if unquoted, err := strconv.Unquote(s); err == nil {
			return unquoted
		}
	}
	return s
}
