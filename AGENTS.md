This file provides universal guidance for agents working in this repository.

## UI Design Philosophy

### Minimalistic TUI
Elma's terminal UI follows a structured layout system inspired by Claude Code's design:
- **Section-based output**: Thin borders (`━`, `─`), not heavy boxes
- **Information-dense**: Every character serves a purpose
- **No chrome**: No decorative borders or visual noise
- **Full content**: Never truncate output — complete markdown is always rendered
- **Graceful fallback**: When not in a terminal (piped/scripted), output degrades to plain text
- **Markdown rendering**: Headers, bold, italic, lists, code blocks, blockquotes, horizontal rules — all rendered in-terminal

### Catppuccin Mocha Color Palette
The **only** color theme used throughout Elma is [Catppuccin Mocha](https://github.com/catppuccin/catppuccin):

| Color | Hex | Semantic Usage |
|-------|-----|---------------|
| **Mauve** `#cba6f7` | Elma prefix, primary accent, prompts |
| **Teal** `#94e2d5` | Tool execution, informational messages |
| **Red** `#f38ba8` | Errors, failures, destructive blocks |
| **Yellow** `#f9e2af` | Warnings, caution commands, shell commands |
| **Green** `#a6e3a1` | Success, confirmations, fast operations |
| **Text** `#cdd6f4` | Primary text, normal output |
| **Overlay0** `#6c7086` | Dim text, timestamps, metadata |

**Design choice:** Catppuccin Mocha over Tokyo Night and Rose Pine for warm, inviting tones (not cold/clinical), soft contrast for long sessions, and a modern aesthetic popular in dev tooling. Each color has specific meaning — mauve means "Elma is speaking", teal means "something is happening", red means "something broke", yellow means "pay attention", green means "all good".

### Module Organization
UI modules are focused and independent:

| Module | Purpose | Dependencies |
|--------|---------|-------------|
| `ui_layout.rs` | Structured layout: borders, sections, status line | crossterm, ui_colors |
| `ui_markdown.rs` | Full markdown rendering: headers, lists, code blocks | ui_colors, ui_syntax |
| `ui_colors.rs` | ANSI color functions (Tokyo Night) | None |
| `ui_tui.rs` | Ratatui TUI wrapper + Tokyo Night palette | ratatui, crossterm |
| `ui_progress.rs` | Indicatif spinners and progress bars | indicatif |
| `ui_interact.rs` | Inquire selection menus and confirmations | inquire |
| `ui_syntax.rs` | Syntect syntax highlighting for code blocks | syntect |
| `ui_spinner.rs` | Braille spinner (std::thread fallback) | None |
| `ui_effort.rs` | Wall-clock effort indicator | None |
| `ui_context_bar.rs` | Token usage progress bar | None |
| `ui_trace.rs` | Trace output formatting | None |

**Design rule:** Each UI module has a single responsibility and zero coupling to other UI modules. Color consistency is maintained through shared Rose Pine constants in `ui_tui.rs`.

### Ratatui Integration
Ratatui is used **minimally** — only for structured layout when rendering complex turn responses. The simple ANSI-based output (`eprintln!`) remains the primary path for most output. Ratatui is an enhancement, not a replacement.

### Additional UI Libraries
- **indicatif**: Thread-safe spinners and progress bars for tool execution and multi-step operations
- **inquire**: Interactive selection menus with vim-mode (j/k navigation) and Rose Pine themed prompts
- **syntect**: Language-aware syntax highlighting for code blocks (Rust, Python, JSON, etc.) with ANSI output

## Elma CLI Philosophy

Elma is a local-first autonomous CLI agent designed to deliver the highest reliability and practical usefulness possible on constrained local models.

Core philosophy:
- Reliability before speed.
- Adaptive reasoning before deterministic rule playback.
- Small-model-friendly decomposition before asking one model call to do too much.
- Truth-grounded answers before polished but weakly supported answers.
- Offline-first behavior by default, with network use only when truly necessary.
- Prompt principles over long brittle examples.
- Classification signals are soft guidance, not hard law.

Elma should feel premium, careful, and capable even on low-end hardware. The system must maximize quality per token, quality per unit of reasoning, and quality per unit of context window.

## What The User Wants

When in doubt, optimize for these repo-specific preferences:
- Keep Elma optimized for small local LLMs first.
- Do not reduce intel-unit coverage for performance without explicit approval.
- If a model is too weak for a step, prefer adding a narrow intermediary intel unit over bloating a prompt or adding rigid heuristics.
- Preserve autonomy, but make it honest and bounded.
- Prefer stable canonical system prompts that are code-authoritative and not casually rewritten to “make tests pass.”
- Do not turn Elma into a keyword matcher.
- Maintain semantic continuity from user intent to final answer.
- Keep the system principle-first, not example-driven.
- Favor incremental reliability closure over ambitious new capability work.

## Mandatory Context Order

Before substantial work:
1. Read [`_tasks/TASKS.md`](_tasks/TASKS.md).
2. Read the active master task if present, usually [`_tasks/active/058_Incremental_Stability_Master_Plan.md`](_tasks/active/058_Incremental_Stability_Master_Plan.md).
3. Check [`_dev-tasks/`](_dev-tasks/) for current structural guidance.
4. Use root-relative paths in reasoning and edits.

If the work touches an existing active task, update that task instead of creating duplicate planning.

## Task Protocol

Follow this workflow unless the user explicitly asks otherwise:
1. Pickup: move a pending task into `_tasks/active/` if starting it formally.
2. Implement surgically.
3. Verify with `cargo build`.
4. Verify behavior with the relevant tests and probes.
5. Report results while the task is still active.
6. Archive only after approval.

Troubleshooting rule:
- If a real bug or regression is discovered, create or continue a `T###` troubleshooting task immediately.

Task creation rules:
- Main tasks use the next available numeric prefix across `_tasks/active/`, `_tasks/pending/`, `_tasks/completed/`, and `_tasks/postponed/`.
- Troubleshooting tasks use the same numeric sequence with a `T` prefix.
- Tasks must be self-documenting enough that renaming to `_DONE` is sufficient when complete.

## Commit And Git Rules

- Never commit unless the user explicitly asks to save, checkpoint, commit, merge, or push.
- Never rewrite history unless the user explicitly asks.
- Never discard intended local changes just to obtain a clean diff.
- If the tree is dirty, work with it carefully.

## Non-Negotiable Architecture Rules

### 1. No Word-Based Routing

Never implement routing, classification, or behavior selection through hardcoded word triggers.

Wrong:
```rust
if input.contains("hello") { route = "CHAT"; }
```

Right:
- use model confidence
- use entropy / margin
- use evidence availability
- use bounded fallback principles

If you are checking user text for words in order to force a route, you are likely violating Elma’s philosophy.

### 2. Reliability Over Speed

Do not skip reasoning stages, ladder stages, or intel units just to make the system faster unless the user explicitly approves that tradeoff.

If a fast path exists:
- it must be low risk
- it must preserve truthfulness
- it must not silently remove needed reasoning

### 3. Use Decomposition To Help Small Models

If a small model is struggling:
- first try to tighten the narrative/context
- then reuse an existing intel unit if it already fits
- then add a new focused intermediary intel unit if truly needed

Do not respond to small-model weakness by:
- stuffing more examples into prompts
- overfitting deterministic rules
- forcing giant prompts
- merging many cognitive jobs into one prompt

Preferred pattern:
- one intel unit
- one role
- one narrow decision or transformation

### 4. Preserve Semantic Continuity

Meaning must survive the whole pipeline:
1. user message
2. intent helper / rephrase
3. speech act / routing / mode
4. complexity / formula / workflow plan
5. program steps
6. final answer

If the user asks for X, and the final answer solves Y, that is a semantic continuity failure even if the code technically ran.

Agents should inspect continuity whenever behavior feels “weird”:
- compare the raw prompt
- compare the intent annotation
- compare the chosen route/formula
- compare the executed steps
- compare the final answer

### 5. Grounded Answers Only

Repo-specific claims must be supported by actual workspace evidence.

If evidence is missing:
- gather it
- or say clearly that evidence is insufficient

Do not:
- hallucinate file names
- soften exact paths into vague labels
- claim edits/tests/verification without artifacts

### 6. Local-First, Offline-First

Elma is primarily for local use on local endpoints.

Default stance:
- prefer local workspace evidence
- prefer local tools
- prefer local runtime facts
- prefer no internet

Web access is a secondary capability, not the default operating assumption.

## Prompt Design Rules

### Principle-First Prompts

System prompts must stay principle-first.

Required structure:
1. governing principle
2. minimal boundary clarification only if necessary
3. compact output contract

Avoid:
- long deterministic rule dumps
- long positive/negative example lists
- prompt scripts that replace judgment

Rule of thumb:
- if a prompt is mostly examples or exceptions, rewrite it

### Canonical Prompt Stability

Managed prompts should be treated as canonical constants.

Do not casually mutate prompts to pass a temporary test.

If prompt changes are necessary:
- they should be deliberate
- they should align with philosophy
- they should be reflected in the code-authoritative canonical source
- grammar, parser expectations, and prompt contracts must agree

### Intel Unit Output Standard

All choice-style intel units must follow the standard compact JSON contract described in [`docs/INTEL_UNIT_STANDARD.md`](docs/INTEL_UNIT_STANDARD.md):

```json
{"choice":"<NUMBER>","label":"<LABEL>","reason":"<ULTRA_CONCISE_JUSTIFICATION>","entropy":0.42}
```

Key rules:
- choice rules describe intention, not consequence
- no heuristics section
- no large example section
- keep output compact for latency and stability

Not every intel unit must use the same classifier schema.
Structured units may return other stable canonical JSON schemas when their job requires it.

## Narrative Context Rules

Whenever an intel unit is asked to decide, prefer a purpose-built narrative over a raw dump.

Good narrative context explains:
- what the user wants
- what stage Elma is in
- what evidence is available
- what decision is needed
- what boundary matters

Raw JSON blobs are acceptable only when they remain small, clear, and are the best fit for the unit.

Before building context compaction systems, make sure the uncompressed decision narratives are already correct.

## Runtime Behavior Priorities

When choosing the next engineering step, use this priority order:
1. eliminate falsehoods
2. eliminate crashes / parse failures / stalls
3. eliminate path and evidence corruption
4. eliminate retry loops and stale recovery behavior
5. improve human-style robustness
6. improve context efficiency
7. expand autonomy
8. expand cross-model adaptability

If stress tests are green but casual human prompts still fail in the real CLI, the system is not yet reliable enough.

## Real Verification Requirements

Do not trust orchestrator-only or model-only checks when the real CLI path is the thing that matters.

Verification ladder:
1. `cargo build`
2. targeted `cargo test`
3. relevant probes or scenarios
4. real `cargo run` validation when behavior is user-facing

Use the real CLI as the authority for:
- startup correctness
- prompt routing behavior
- end-to-end final answers
- stress-testing outcomes

### Required Commands

Build:
```bash
cargo build
```

Tests:
```bash
cargo test
```

Behavior probes:
```bash
./probe_parsing.sh
./reliability_probe.sh
./run_intention_scenarios.sh
./smoke_llamacpp.sh
```

Formatting:
```bash
cargo fmt
```

Architecture analysis:
```bash
cd _dev-system/analyzer && cargo run
```

## Configuration And Runtime Safety

- Model and system configs live in `config/` as TOML files.
- Treat profile/config health as a reliability surface, not just data files.
- Prefer atomic config writes and defensive startup validation.
- Do not let malformed transient profile state silently break normal CLI usage.

## Startup Context Expectations

Elma should begin sessions with enough concise runtime context to reason well:
- workspace context
- workspace brief
- current working directory / repo context
- OS/platform basics
- shell name
- core tool availability
- active model/base URL/runtime facts when useful

This context should be concise and useful, not bloated.

## Snapshot And Safety Expectations

If Elma mutates files, recovery should be possible.

Structured edit flows should be snapshot-aware.
Long term, shell-based file mutation paths should also be rollback-safe.

Do not design workflows that make recovery harder than necessary.

## De-Bloating Guidance

`src/main.rs` has historically been oversized. Continue extracting logic into cohesive domain modules.

High-risk concentration areas should be treated carefully, especially:
- `src/intel_units.rs`
- `src/json_error_handler.rs`
- `src/program_policy.rs`
- `src/defaults_evidence.rs`
- `src/types_core.rs`

Do not perform broad refactors unless they directly serve the active reliability goal.

## What To Prefer When Stuck

Prefer this order:
1. better evidence
2. better narrative
3. narrower intel decomposition
4. smaller safer fallback program
5. better verification

Avoid this order:
1. more heuristics
2. more keywords
3. more examples
4. hotter temperatures
5. vague retries

## Success Standard

A change is aligned with Elma only if it improves at least one of:
- truthfulness
- reliability
- bounded autonomy
- small-model effectiveness
- context efficiency

without materially harming the others.
