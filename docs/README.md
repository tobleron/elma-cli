# Elma CLI

**A local-first autonomous CLI agent delivering reliable, truth-grounded assistance on constrained local models.**

Elma is a Rust CLI agent that runs fully offline. It reads your workspace, reasons about your requests, calls tools (shell, file read, search, document parsing), and produces grounded answers — all through a clean terminal interface.

## Design Philosophy

Elma is built around six core principles:

| Principle | Meaning |
|-----------|---------|
| **Reliability before speed** | Correctness and crash-safety come first. Every tool output is flushed to disk immediately. Every final answer is checked against source evidence. |
| **Local-first, offline-first** | No internet required. Works with llama.cpp, Ollama, and other local endpoints. |
| **Small-model-friendly decomposition** | Requests are decomposed into narrow, single-responsibility intel units so 3B-parameter models can reason effectively. |
| **Truth-grounded answers** | The evidence ledger tracks every tool output. An enforcement gate rejects final answers containing ungrounded claims. |
| **Principle-first prompts** | System prompts describe reasoning principles, not rigid rules. No hardcoded keyword matching for routing. |
| **Adaptive reasoning** | Elma adapts its execution strategy (skill formula, complexity ladder, tool selection) based on the user's intent and workspace state. |

## Quick Start

```bash
# Build from source
cargo build

# Run with a local model (llama.cpp endpoint)
cargo run -- --model llama-3.2-3b --base-url http://localhost:8080

# Clean old sessions
cargo run -- session-gc --older-than-days 7 --dry-run
cargo run -- session-gc --older-than-days 30 --confirm
```

## How Elma Works

A user request flows through these stages:

1. **Intent annotation** — a lightweight model call rephrases the user's request into a clear objective.
2. **Route selection** — a heuristic determines whether the request is conversational (`CHAT`) or task-oriented (`WORKFLOW`).
3. **Skill & formula matching** — based on workspace state and intent, Elma selects a skill (e.g., DocumentReader, RepoExplorer) and a formula (execution pattern).
4. **Tool-calling pipeline** — the model receives a system prompt with workspace context, skill playbook, and available tools. It invokes tools directly: `shell` for commands, `read` for files, `search` for pattern matching, `respond` for final answers.
5. **Evidence grounding** — every tool output is recorded in the evidence ledger. The enforcement gate validates final answers against collected evidence.
6. **Session persistence** — tool outputs, transcripts, and state are flushed to disk incrementally for crash safety.

## Core Capabilities

| System | Purpose |
|--------|---------|
| **Tool-calling pipeline** | Model plans and executes directly via tools (shell, read, search, respond) |
| **LLM provider layer** | Native Rust abstraction for OpenAI, Anthropic, OpenAI-compatible (llama.cpp, Ollama), Azure, Groq |
| **Dynamic tool registry** | Tools are discovered on demand via capability hints; only core tools are always loaded |
| **Skill system** | Bounded formula selection: General, DocumentReader, RepoExplorer, TaskSteward, FileScout |
| **Document intelligence** | Multi-format extraction: PDF, EPUB, HTML, MOBI, DjVu, Markdown, TXT |
| **Evidence ledger** | Tracks every tool output, maps claims to evidence, enforces grounded answers |
| **Session management** | Full lifecycle: SQLite store, JSON index, garbage collector, incremental transcript flush |
| **Safety systems** | Safe mode toggle, permission gate, shell command preflight, protected path detection |
| **Hybrid search** | TF-IDF + token-overlap retrieval for memory and tool search |

## Project Structure

```
elma-cli/
├── src/                    # Rust source (160+ modules)
│   ├── main.rs             # Entry point, module registry
│   ├── app_chat_loop.rs    # Main interactive loop
│   ├── tool_loop.rs        # Tool-calling execution loop (1518 lines)
│   ├── tool_calling.rs     # Per-tool dispatch (shell, read, search, respond)
│   ├── orchestration_core.rs # System prompt construction (747 lines)
│   ├── llm_provider.rs     # Provider abstraction (OpenAI, Anthropic, etc.) (860 lines)
│   ├── skills.rs           # Skill registry and formula selector (466 lines)
│   ├── formulas/           # Formula patterns and scoring (patterns.rs, scores.rs)
│   ├── evidence_ledger.rs  # Evidence tracking and claim enforcement (809 lines)
│   ├── session_*.rs        # Session management modules
│   ├── safe_mode.rs        # Permission toggle (ask/on/off)
│   ├── permission_gate.rs  # Destructive command confirmation
│   ├── shell_preflight.rs  # Command risk classification (964 lines)
│   ├── document_adapter.rs # Multi-format document extraction (1752 lines)
│   ├── tool_registry.rs    # Dynamic tool registry
│   ├── hybrid_search.rs    # FTS + similarity search
│   ├── stop_policy.rs      # Tool loop termination (1099 lines)
│   ├── claude_ui/          # Terminal UI components
│   └── intel_units/        # Intel unit implementations
├── config/                 # TOML configuration
│   ├── runtime.toml        # Shared llama.cpp request caps, timeouts, probe defaults
│   ├── defaults/           # Intel unit prompt templates (~70 profiles)
│   ├── grammars/           # GBNF grammar files for constrained JSON output
│   └── <model>/            # Per-model profile overrides
├── sessions/               # Session storage directory (auto-created)
├── _tasks/                 # Task management
│   ├── TASKS.md            # Canonical task list
│   ├── active/             # In-progress tasks
│   ├── pending/            # Next approved work
│   ├── completed/          # Finished tasks
│   └── postponed/          # Deferred or superseded tasks
└── docs/                   # Documentation (you are here)
```

## Configuration

Elma loads configuration from TOML files with a fallback chain:

```
1. config/<model_name>/<profile>.toml     # Model-specific overrides
   ↓
2. config/defaults/<profile>.toml         # Global defaults
```

Key configuration files:
- `config/profiles.toml` — profile registry mapping names to endpoints and models
- `config/runtime.toml` — shared llama.cpp/OpenAI-compatible runtime knobs such as HTTP timeout, request timeout, response token caps, tool-loop timeout, and probe `n_probs`
- `config/defaults/` — ~70 intel unit prompt templates (orchestrator, intent_helper, speech_act, router, planner, summarizer, critic, reviewers, etc.)
- `config/grammars/` — GBNF grammars for forcing valid JSON from intel unit calls

## Documentation Index

### Current State (How Elma Works Now)

| Document | Covers |
|----------|--------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | End-to-end workflow, design decisions, system interactions |
| [ARCHITECTURAL_RULES.md](ARCHITECTURAL_RULES.md) | Non-negotiable architecture rules, narrative context, runtime priorities |
| [SOUL.md](SOUL.md) | Elma's character, identity, tone, autonomy boundary |
| [TOOL_CALLING_PIPELINE.md](TOOL_CALLING_PIPELINE.md) | How the model plans and executes via tools |
| [LLM_PROVIDER.md](LLM_PROVIDER.md) | Native Rust provider abstraction layer |
| [SKILL_SYSTEM.md](SKILL_SYSTEM.md) | Skill registry, formulas, playbook rules, context-budget awareness |
| [EVIDENCE_LEDGER.md](EVIDENCE_LEDGER.md) | Claim grounding, enforcement gate, evidence lifecycle |
| [DOCUMENT_INTELLIGENCE.md](DOCUMENT_INTELLIGENCE.md) | Multi-format extraction and document reading |
| [SESSION_MANAGEMENT.md](SESSION_MANAGEMENT.md) | Session lifecycle, storage, GC, transcripts |
| [SECURITY_AND_PERMISSIONS.md](SECURITY_AND_PERMISSIONS.md) | Safe mode, permission gate, shell preflight |
| [CONFIGURATION.md](CONFIGURATION.md) | TOML profiles, model configs, tuning system |
| [DEVELOPMENT.md](DEVELOPMENT.md) | Build, test, project structure, common issues |
| [DEVELOPMENT_GUIDELINES.md](DEVELOPMENT_GUIDELINES.md) | Commit rules, config safety, verification, de-bloating, canonical prompts |

### Future State (What's Being Designed)

| Document | Type | Status |
|----------|------|--------|
| [_proposals/](_proposals/) | Optional features under consideration | |
| &nbsp;&nbsp;[001-fetch-sandboxing](_proposals/001-fetch-sandboxing.md) | Proposal | Draft |
| &nbsp;&nbsp;[002-sub-agent-delegation](_proposals/002-sub-agent-delegation.md) | Proposal | Draft |
| &nbsp;&nbsp;[003-budget-aware-orchestration](_proposals/003-budget-aware-orchestration.md) | Proposal | Draft |
| &nbsp;&nbsp;[004-crate-reconciliation](_proposals/004-crate-reconciliation.md) | Proposal | Draft |
| &nbsp;&nbsp;[005-extraction-integration](_proposals/005-extraction-integration.md) | Proposal | Draft |
| [_directives/](_directives/) | Non-negotiable architecture mandates | |
| &nbsp;&nbsp;[001-evidence-grounded-stability](_directives/001-evidence-grounded-stability.md) | Directive | Proposed |
| &nbsp;&nbsp;[002-dispatchable-modes](_directives/002-dispatchable-modes.md) | Directive | Proposed |
| &nbsp;&nbsp;[003-semantic-continuity](_directives/003-semantic-continuity.md) | Directive | Proposed |
| &nbsp;&nbsp;[004-offline-first](_directives/004-offline-first.md) | Directive | Proposed |
| &nbsp;&nbsp;[005-transcript-visibility](_directives/005-transcript-visibility.md) | Directive | Proposed |
| &nbsp;&nbsp;[006-principle-first-prompts](_directives/006-principle-first-prompts.md) | Directive | Proposed |
| &nbsp;&nbsp;[007-dynamic-decomposition](_directives/007-dynamic-decomposition.md) | Directive | Proposed |
| &nbsp;&nbsp;[008-tokenized-theme](_directives/008-tokenized-theme.md) | Directive | Proposed |

Task management: [`_tasks/TASKS.md`](../_tasks/TASKS.md) | Master plan: [`_tasks/_masterplan.md`](../_tasks/_masterplan.md)

## Development

```bash
# Build
cargo build

# Run tests (600+ tests)
cargo test

# Run specific test
cargo test session_flush

# Format code
cargo fmt

# Architecture analysis
cd _dev-system/analyzer && cargo run

# Run behavioral probes
./probe_parsing.sh
./reliability_probe.sh
./run_intention_scenarios.sh
./smoke_llamacpp.sh
```
