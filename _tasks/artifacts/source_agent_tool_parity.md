# Source-Agent Tool Parity Gap Matrix

**Generated:** 2026-05-01
**Scope:** Agents under `_knowledge_base/_source_code_agents/`
**Purpose:** Identify every tool class supported by peer agents, map to elma-cli equivalent or document the gap.

## Classification Key

| Status | Meaning |
|--------|---------|
| DONE | Elma has a working equivalent (rust-native or shell-backed) |
| PENDING | Gap identified, a pending task exists |
| MISSING | No implementation, no pending task — needs scoping |
| DEFERRED_NETWORK | Requires network — offline-disabled by default |
| NOT_APPLICABLE | Not relevant to elma-cli's architecture or scope |

## Implementation Mode Key

| Mode | Meaning |
|------|---------|
| `rust_native` | Pure Rust implementation, no external deps at runtime |
| `rust_wrapper` | Rust wrapper around system binary/Library |
| `shell_fallback` | Rust-native preferred, shell fallback available |
| `network_optional` | Works offline but optional network mode exists |
| `external_extension` | MCP or plugin-based, loaded at runtime |

---

## Tool Family Matrix

### File System Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Read/View | `read` | DONE | rust_native | Claude Code, Codex CLI, Crush, Roo Code, Qwen Code, Hermes Agent, OpenCode, OpenCrabs | — |
| Write | `write` | DONE | rust_native | All agents | — |
| Edit (exact find-replace) | `edit` | DONE | rust_native | Claude Code, Crush, Roo Code, Qwen Code, Hermes Agent, OpenCode, OpenCrabs | — |
| Multi-edit / Patch | `patch` | DONE (needs 394) | rust_native | Crush (multiedit), Roo Code (apply_diff), Qwen Code (edit correction), Hermes Agent (patch) | 394 |
| Glob | `glob` | DONE | rust_native | Claude Code, Codex CLI, Crush, Qwen Code, OpenCode, OpenCrabs | — |
| LS / List directory | `ls` | DONE | rust_native | Codex CLI, Crush, Roo Code, Qwen Code, OpenCode, OpenCrabs | — |
| File metadata (stat) | MISSING | MISSING | — | Crush (filetracker), Claude Code (implicit via read) | 393 |
| Copy/Move/Delete | MISSING | MISSING | — | Crush (via bash), Roo Code (via bash), Claude Code (via bash) | 425 |
| Mkdir / Touch | MISSING | MISSING | — | All agents (via bash or file tools) | 425 |
| Download URL to file | MISSING | MISSING | — | Crush (download), Roo Code (via bash) | 419 |
| File context tracking | MISSING | MISSING | — | Crush (filetracker), Claude Code (implicit) | 395 |
| Workspace ignore policy | MISSING | MISSING | — | Roo Code (.clinerules), Qwen Code (.qwenignore), Claude Code (CLAUDE.md) | 396 |

### Search & Navigation Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Text pattern search (regex) | `search` | DONE | shell_fallback | Claude Code, Codex CLI, Crush, Roo Code, Qwen Code, Hermes Agent, OpenCode, OpenCrabs | — |
| Symbol-aware repo map | MISSING | MISSING | — | Aider (repomap), Qwen Code (LSP), Claude Code (LSPTool) | 397 |
| Tag cache / ctags | MISSING | MISSING | — | Indirect via LSP in Claude Code, Qwen Code | 397 |
| LSP diagnostics | MISSING | PENDING | — | Claude Code (LSPTool), Codex CLI, Crush (diagnostics), Qwen Code (LSP), OpenCode (diagnostics) | 398 |
| LSP references / intelligence | MISSING | PENDING | — | Crush (references), Claude Code (LSPTool), Codex CLI (LSP), Qwen Code (LSP) | 398 |
| Tool search / discovery | `tool_search` | DONE | rust_native | Claude Code (ToolSearchTool), Codex CLI (tool_suggest) | — |
| Search web | MISSING | DEFERRED_NETWORK | network_optional | Claude Code, Crush, Qwen Code, Hermes Agent, OpenCrabs (3 variants) | 426 |

### File Mutation Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Edit (find-replace) | `edit` | DONE | rust_native | All code agents | — |
| Write (create/overwrite) | `write` | DONE | rust_native | All agents | — |
| Atomic multi-file patch | `patch` | DONE | rust_native | Crush (multiedit), Roo Code (apply_diff), Hermes Agent (patch) | 394 |
| Diff-aware edit | MISSING | PENDING | — | Roo Code (apply_diff), Aider (udiff) | 394 |
| Trash / safe delete | MISSING | PENDING | — | Roo Code (via custom), OpenCode (via bash) | 217 (trash crate exists) |
| Notebook (.ipynb) edit | MISSING | MISSING | — | Claude Code (NotebookEditTool), OpenCrabs (notebook_edit) | — |

### Execution Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Shell command | `shell` | DONE | rust_native | All agents | — |
| Background jobs | MISSING | PENDING | — | Claude Code (Task tools), Crush (job_output/kill), Roo Code (read_command_output), Qwen Code (is_background), Hermes Agent (process) | 418 |
| Code interpreter | MISSING | MISSING | — | Hermes Agent (execute_code), OpenCrabs (execute_code), Open Interpreter (exec()), AgenticSeek | 420 |
| Background job management | MISSING | PENDING | — | Claude Code (Task system), Codex CLI (jobs), Crush (job_output/job_kill) | 418 |
| Clean room shell | MISSING | PENDING | — | Claude Code (sandbox), Codex CLI (bwrap/seatbelt), Qwen Code (Docker) | 417 |

### Network & Web Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| URL fetch | `fetch` | DONE | network_optional | Claude Code, Codex CLI, Crush (2 variants), Qwen Code, Hermes Agent, OpenCode, OpenCrabs (http_request) | — |
| Web search | MISSING | DEFERRED_NETWORK | network_optional | Claude Code, Codex CLI, Crush (DuckDuckGo), Qwen Code (3 providers), Hermes Agent (Firecrawl), OpenCrabs (3 variants), AgenticSeek (SearXNG) | 426 |
| Browser automation | MISSING | DEFERRED_NETWORK | external_extension | Claude Code (WebBrowserTool), OpenHands (Playwright), Hermes Agent (CDP), AgenticSeek (Selenium), Open Interpreter (Chrome), OpenCrabs (CDP) | 404 |
| MCP client | MISSING | PENDING | external_extension | Claude Code, Codex CLI, Crush, Goose (native), Roo Code, OpenHands, Qwen Code, Hermes Agent, OpenCode, OpenCrabs, LocalAGI | 405 |
| Download file | MISSING | MISSING | network_optional | Crush (download), Hermes Agent, Roo Code (via shell) | 419 |

### Agent & Orchestration Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Subagent delegation | MISSING | MISSING | — | Claude Code (AgentTool), Codex CLI (agent), Crush (coordinator), Goose, Roo Code (new_task), Qwen Code (task), Hermes Agent (delegate_task), OpenCode (agent), OpenCrabs (spawn_agent) | 410 |
| Agent-to-Agent messaging | MISSING | MISSING | — | Claude Code (SendMessageTool), OpenCrabs (A2A) | 410 |
| Task/Plan creation | MISSING | MISSING | — | OpenCrabs (plan), Roo Code (plan mode), Claude Code (plan mode), Qwen Code (plan mode) | 389 |
| Slash commands | MISSING | PENDING | — | Claude Code, Roo Code (run_slash_command), Qwen Code, Hermes Agent, OpenCode, OpenCrabs | 423 |
| Skills / Recipes | MISSING | MISSING | — | All major agents have a skill system | 407 |

### Memory & Persistence Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Todo / Task list | `update_todo_list` | DONE | rust_native | Claude Code (Task tools), Crush (todos), Roo Code (update_todo_list), Qwen Code (todo_write), Hermes Agent (todo) | — |
| Session persistence | SQLite/JSONL | DONE | rust_native | All agents have session state | — |
| Project memory | MISSING | PENDING | — | Claude Code (CLAUDE.md), Goose (memories), Qwen Code (save_memory), Hermes Agent (memory), OpenCode (OpenCode.md), OpenCrabs (memory_search) | 411 |
| RAG / vector search | MISSING | PENDING | — | LocalAGI (chromem), OpenCrabs (hybrid FTS5+GGUF) | 411 |
| Session search | MISSING | PENDING | — | Hermes Agent (session_search), OpenCrabs (session_search) | 411 |
| Evidence ledger | `evidence` | DONE | rust_native | Claude Code (implicit via artifact tracking) | — |
| Large result persistence | auto-persist | DONE | rust_native | Claude Code, Crush (large file to temp) | — |

### Developer Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| LSP diagnostics | MISSING | PENDING | external_extension | Claude Code (LSPTool), Codex CLI, Crush (diagnostics), Qwen Code, OpenCode | 398 |
| LSP references | MISSING | PENDING | external_extension | Crush (references), Claude Code (LSPTool), Codex CLI | 398 |
| Auto lint/test | MISSING | PENDING | — | Aider (auto-lint), Claude Code (post-edit lint) | 399 |
| Git inspection | MISSING | PENDING | — | Claude Code (via bash), Codex CLI (git), Aider (auto-commit) | 421 |
| Git commit/push | MISSING | PENDING | — | Claude Code, Codex CLI (git), Aider (auto-commit) | 421 |
| Verification planner | MISSING | PENDING | — | Roo Code (attempt_completion), Claude Code (implicit via task system) | 399 |
| Log viewer | MISSING | MISSING | — | Crush (crush_logs) | — |

### Terminal & UI Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Respond to user | `respond` | DONE | rust_native | All agents | — |
| Summary / task complete | `summary` | DONE | rust_native | Claude Code (BriefTool), Roo Code (attempt_completion) | — |
| Ask user question | MISSING | PENDING | — | Claude Code (AskUserQuestionTool), Roo Code (ask_followup_question), Hermes Agent (clarify) | 428 |
| Final answer formatting | `respond` | DONE | rust_native | All agents | — |
| Plaintext default output | current TUI | PENDING | — | Claude Code (plain text), Roo Code (plain text) | 392 |
| Markdown artifact output | MISSING | PENDING | — | Claude Code (markdown render), Roo Code (markdown) | 385 |
| Compact header / footer | implemented | DONE | rust_native | Claude Code (status bar), Roo Code (compact mode) | — |

### Meta & Config Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Config management | `config` (via CLI) | DONE | rust_native | Claude Code (ConfigTool), Codex CLI, Roo Code, Qwen Code, Hermes Agent, OpenCrabs (config_manager) | — |
| Doctor / diagnostics | MISSING | PENDING | — | Roo Code (task system), Claude Code (Crush info/logs) | 401 |
| Session lifecycle | `session` commands | DONE | rust_native | Claude Code (resume/clear/exit), Hermes Agent (session management) | — |
| Permission / approval gate | `permission_gate` | DONE | rust_native | Codex CLI (request_permissions), Crush (permission checking), Roo Code (auto-approval) | — |
| Execution profile (local/restricted) | MISSING | MISSING | — | Codex CLI (sandbox), Qwen Code (Docker), Open Interpreter (safe mode) | 406 |

### Specialized Tools

| Tool Family | Elma Tool | Status | Mode | Source Agent Support | Pending Task |
|---|---|---|---|---|---|
| Image generation | MISSING | MISSING | external_extension | Claude Code (via MCP), Roo Code (generate_image), Hermes Agent (image_generate), OpenCrabs (generate_image), LocalAGI (flux/SD) | — |
| Image analysis | MISSING | MISSING | external_extension | Hermes Agent (vision_analyze), OpenCrabs (analyze_image), AgenticSeek | — |
| Voice / TTS | MISSING | NOT_APPLICABLE | — | Hermes Agent (text_to_speech), AgenticSeek (whisper) | — |
| Cron / scheduling | MISSING | MISSING | — | Claude Code (ScheduleCronTool), Goose (scheduler), Hermes Agent (cronjob), OpenCrabs (cron_manage), LocalAGI (periodic_runs) | — |
| Slack / Discord / Messaging | MISSING | NOT_APPLICABLE | — | Hermes Agent (gateway platform tools), OpenCrabs (messaging tools) | — |
| Home Assistant | MISSING | NOT_APPLICABLE | — | Hermes Agent (ha_* tools) | — |
| RL training tools | MISSING | NOT_APPLICABLE | — | Hermes Agent (rl_* tools) | — |

---

## Elma-CLI Current Tool Inventory

### Core Tools (always available, all `rust_native` or `rust_wrapper`)

| Tool | Category | Implementation | Can be deferred? | Check prerequisite |
|---|---|---|---|---|
| `read` | File System | Rust-native (document_adapter) | No | None |
| `write` | File System | Rust-native | No | None |
| `edit` | File System | Rust-native | No | None |
| `patch` | File System | Rust-native | No | None |
| `glob` | File System | Rust-native | No | None |
| `ls` | File System | Rust-native | No | None |
| `search` | Search | Rust-native (wraps ripgrep) | No | rg or grep |
| `shell` | Execution | Rust-native (wraps sh) | No | sh or bash |
| `fetch` | Network | Rust-native | No | None |
| `respond` | Output | Rust-native (internal) | No | None |
| `summary` | Output | Rust-native (internal) | No | None |
| `update_todo_list` | Memory | Rust-native (internal) | No | None |
| `tool_search` | Meta | Rust-native (registry) | No | None |

### Runtime Systems (not exposed as tools but provide capabilities)

| System | Purpose |
|---|---|
| Session persistence (JSONL/SQLite) | Save/restore sessions |
| Transcript-native UI (ratatui) | Terminal rendering |
| Context compaction | Manage context window |
| Permission gates | Safety for dangerous operations |
| Command budget | Rate limiting |
| Hook system | Pre/post tool hooks |
| Evidence ledger | Structured evidence tracking |
| Skills system | Formula-based external guidance |
| Dynamic tool registry | Searchable tool discovery |

---

## Gap Summary

### Critical Gaps (missing in elma-cli, present in most source agents)

| Gap | Agent Coverage | Recommended Task |
|---|---|---|
| Background job management | 6 agents | 418 |
| MCP client | 12 agents | 405 |
| Subagent delegation | 9 agents | 410 |
| Web search | 7 agents | 426 |
| Browser automation | 6 agents | 404 |
| LSP/Diagnostics | 5 agents | 398 |
| Skills/Recipe system | 8 agents | 407 |
| Project memory | 7 agents | 411 |
| Git tools (status/diff/log) | 3 agents | 421 |
| Code interpreter | 4 agents | 420 |

### Important Gaps

| Gap | Agent Coverage | Recommended Task |
|---|---|---|
| File metadata (stat) | Implicit in all agents | 393 |
| File copy/move/delete | All agents (via bash) | 425 |
| Slash command parity | 7 agents | 423 |
| Clean room shell | 3 agents | 417 |
| Auto lint/test planner | 3 agents | 399 |
| Config manager/doctor | 5 agents | 401 |
| Clarification tool | 3 agents | 428 |
| Cron/scheduling | 5 agents | — (lower priority) |
| Notebook editing | 2 agents | — (lower priority) |

### Rust-Native Preference Mapping

| Tool | Current Mode | Should Remain? | Notes |
|---|---|---|---|
| `search` | shell_fallback | Yes | Wraps rg; fast enough, no perf benefit to native |
| `shell` | rust_native | Yes | Core execution primitive |
| `fetch` | network_optional | Yes | Properly gated |
| `patch` | rust_native | Yes | Already atomic, needs transaction/rollback (394) |
| `read` | rust_native | Yes | document_adapter handles all formats |
| `write` | rust_native | Yes | — |
| `edit` | rust_native | Yes | — |
| `glob` | rust_native | Yes | — |
| `ls` | rust_native | Yes | — |

---

## Agent-Specific Notable Tools (No Elma Equivalent, Probably Out Of Scope)

| Agent | Tool | Reason Out Of Scope |
|---|---|---|
| Hermes Agent | Voice/TTS | Not relevant for CLI agent |
| Hermes Agent | Home Assistant | Domain-specific IoT |
| Hermes Agent | RL training tools | Niche ML use case |
| Hermes Agent | Feishu/Lark/DingTalk | Platform-specific integrations |
| Hermes Agent | Discord gateway | Platform-specific |
| OpenCrabs | Messaging (Trello/Discord/Telegram/Slack) | Platform-specific integrations |
| AgenticSeek | Voice (whisper) | Not relevant for CLI agent |
| Open Interpreter | %verbose/%reset/%undo interactive cmds | Elma has transcript-native workflow |

### Agents Without Standard Tool-Calling Interface

| Agent | Notes |
|---|---|
| Aider | Uses edit formats (whole/diff/udiff), not discrete tool functions |
| Open Interpreter | Uses exec() with language parameter — minimal tool surface |
| OpenAI Python SDK | Library, not an agent |
| Kolosal (Desktop) | Desktop app with no tool-calling interface |
| Kolosal CLI | Fork of Qwen Code, covered by Qwen Code row |
