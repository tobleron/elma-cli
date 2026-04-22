This file provides universal guidance for agents working in this repository.

## UI Design Philosophy

### Current UI Direction
Elma's interactive terminal UI must closely mimic Claude Code's terminal interface as observed in `_stress_testing/_claude_code_src`.

The active planning source for this work is:
- `_tasks/pending/166_Claude_Code_Terminal_Parity_Master_Plan.md`
- `_tasks/pending/T179_Terminal_UI_Hang_Triage_And_Recovery_Gate.md`
- `_tasks/pending/167_Claude_Code_Source_Audit_And_Golden_Terminal_Harness.md` through `_tasks/pending/178_End_To_End_Claude_Parity_Stress_Gate.md`

Older UI instructions that conflict with Claude Code parity are stale. In particular:
- Do not preserve the old five-frame Elma layout.
- Do not preserve the old persistent header strip, activity rail, boxed composer, or context progress bar.
- Do not preserve Gruvbox-only, Tokyo Night, Catppuccin, or Rose Pine theme assumptions.
- Do not preserve a crossterm-only restriction if Ratatui or another Rust terminal crate gets closer to Claude Code behavior.

### Claude Code Parity Target
The default interactive UI should be sparse, message-first, and Claude-like:

```text
> user request

∴ Thinking

● assistant response with rendered markdown

● shell (cargo test)
  running...
✓ shell completed

> prompt
```

Required user-facing patterns:
- User rows use a `>` prompt convention.
- Assistant rows use `●` and rich terminal markdown.
- Thinking rows use the Claude-style `∴ Thinking` convention, collapsed by default and expanded in transcript/verbose modes.
- Tool rows use Claude-like loader/progress/result states, not Elma-specific activity rails.
- Slash commands open a fuzzy picker when the user types `/`.
- File mentions open a quick-open picker when the user types `@`.
- Bash mode is entered with `!` where supported.
- The task/todo list appears and checkmarks automatically during multi-step work.
- Context compaction displays a Claude-like compact summary and compact boundary.
- Ctrl-O expands transcript/history details.
- Ctrl-T toggles tasks.
- Double Esc clears the prompt.
- Double Ctrl-C or Ctrl-D exits cleanly.

### Theme
The default theme is a high-contrast monochrome base with Pink as the primary accent and Cyan as the complementary accent:

| Token | Purpose |
|-------|---------|
| Black | terminal background baseline |
| White | primary text |
| Greys | metadata, separators, disabled text, inactive hints |
| Pink | primary accent, prompt affordances, active selection, attention states |
| Cyan | complementary accent, tools, file mentions, progress and informational contrast |

Theme implementation must be tokenized. Future themes should be able to replace Pink with another primary color, such as Orange, without rewriting renderers.

Do not hard-code active UI colors outside the canonical theme module. Active interactive UI code must not reintroduce Gruvbox/Tokyo Night/Catppuccin/Rose Pine palettes.

### Implementation Posture
Be aggressive about replacing old UI code. Binary size is not a concern for this UI track.

Allowed and encouraged when they improve parity:
- `ratatui` for retained frame rendering.
- `crossterm` for raw mode, events, alternate screen, and terminal control.
- `tui-textarea` or an equivalent editor layer for prompt editing.
- `nucleo-matcher` for fuzzy command/file pickers.
- `portable-pty`, `rexpect`, `vt100`, `strip-ansi-escapes`, and `insta` for real terminal snapshot tests.
- `pulldown-cmark` plus `syntect` for markdown and syntax highlighting if the current markdown renderer is not enough.
- `arboard` for clipboard-aware prompt affordances.
- `notify-rust` for optional OS notifications after in-terminal notifications work.

### Hang And Terminal Safety
The UI currently has reported hang/bad-behavior risk. Treat terminal responsiveness as P0.

Never let the active interactive path:
- mix direct `println!`/`eprintln!` output with a TUI-owned terminal;
- block on raw stdin prompts while the TUI is active;
- leave raw mode or alternate screen uncleared after exit or panic;
- hide permission prompts behind stale UI state;
- stop redrawing during model streams, tool execution, picker navigation, resize, Ctrl-C, or Ctrl-D;
- keep old Elma chrome because it is easier than replacing it.

Interactive output should flow through one authoritative renderer/event queue. Noninteractive/script output may keep simple output paths, but those paths must be explicit and tested.

### Preserved Elma Behavior
Claude Code parity governs the visible terminal interface. It does not replace Elma's local-first, small-model-first architecture.

Keep context management, compaction, token budgeting, and llama.cpp-friendly reliability work when it helps constrained local 3B models. Do not delete useful context-management tasks or modules just because Claude Code has different internals. Adapt their visible behavior to the Claude-like UI instead.

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
- Make the interactive terminal look and behave like Claude Code as closely as practical in Rust.
- Treat ugly, hanging, stale, or non-Claude UI behavior as a P0 product defect.
- Prefer removing or quarantining conflicting legacy UI paths over preserving old Elma chrome.
- Use the black/white/grey plus Pink/Cyan tokenized theme for the default interactive UI.
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
2. Read the active master task identified by `_tasks/TASKS.md`, currently usually [`_tasks/active/095_Incremental_Upgrade_Master_Plan.md`](_tasks/active/095_Incremental_Upgrade_Master_Plan.md) if present.
3. Check [`_dev-tasks/`](_dev-tasks/) for current structural guidance.
4. For interactive UI work, read [`_tasks/pending/166_Claude_Code_Terminal_Parity_Master_Plan.md`](_tasks/pending/166_Claude_Code_Terminal_Parity_Master_Plan.md) and [`_tasks/pending/T179_Terminal_UI_Hang_Triage_And_Recovery_Gate.md`](_tasks/pending/T179_Terminal_UI_Hang_Triage_And_Recovery_Gate.md).
5. Use root-relative paths in reasoning and edits.

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

UI parity probes:
```bash
./ui_parity_probe.sh --all
```

If `./ui_parity_probe.sh` has not been implemented yet, UI-facing work must still include real CLI or pseudo-terminal validation and must document that the parity harness is not available yet.

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
