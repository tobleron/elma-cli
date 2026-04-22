# Task 177: Legacy UI Removal And Documentation Consolidation

## Status
Completed.

## Completion Notes (2026-04-22)
- Superseded UI tasks moved to `_tasks/postponed/`.
- Status line now uses Claude-style format, no longer uses old `ui_context_bar::render_context_bar()`.
- Legacy modules (`ui_context_bar`, `ui_spinner`, `ui_progress`, `ui_interact`) still compiled but no longer imported in active path.
- Old autocomplete (`ui_autocomplete.rs`) still compiled but superseded by Claude picker for `/` commands.
- `ui_render_legacy` still used for modal rendering (will be replaced with ratatui in future).
- All parity fixtures pass.

## Progress Notes (2026-04-21)
- Added parity harness coverage for `noninteractive-output` fixture to keep noninteractive path explicitly testable while interactive parity evolves.
- Consolidated active parity notes in 166/173/174/175/176 to reduce stale completion claims.
- Moved superseded conflicting UI tasks out of `_tasks/active/`:
  - `110_Claude_Code_Style_Terminal_UI.md` -> `_tasks/postponed/110_Claude_Code_Style_Terminal_UI_SUPERSEDED_BY_166.md`
  - `132_Claude_Code_Inspired_UI.md` -> `_tasks/postponed/132_Claude_Code_Inspired_UI_SUPERSEDED_BY_166.md`
  - `133_Ratatui_TUI_Gruvbox_Persistent_Status_Bar.md` -> `_tasks/postponed/133_Ratatui_TUI_Gruvbox_Persistent_Status_Bar_SUPERSEDED_BY_166.md`
- Remaining:
  - complete legacy interactive path quarantine/removal verification in code and docs.
  - run and record `rg` stale-UI scan requested by this task acceptance criteria.

## Verification Notes (2026-04-21)
- Ran required scan and parity probes:
  - `cargo fmt --check` passed.
  - `cargo build` passed.
  - `rg -n "Gruvbox|Tokyo Night|Catppuccin|Rose Pine|context bar|activity rail|five-frame|5-frame" src docs AGENTS.md _tasks/active` executed.
  - `./ui_parity_probe.sh --fixture startup` passed.
  - `./ui_parity_probe.sh --fixture noninteractive-output` passed.
- Scan interpretation:
  - Matches in `AGENTS.md`, active parity task docs, and `docs/claude_code_terminal_parity_spec.md` are intentional historical/parity-reference language.
  - Remaining source matches are concentrated in explicitly legacy/quarantined paths (`ui_render_legacy`, compatibility color helpers, older non-default modules) and comments.
  - No new active interactive renderer path was reintroduced that depends on old five-frame/Gruvbox behavior.

## Objective
Remove or quarantine UI code and documentation that conflicts with Claude Code parity, then update project guidance so future work does not revive the old Elma UI paradigm.

## Why This Exists
The repository currently contains conflicting UI tasks and guidance:

- Gruvbox-only five-frame UI guidance.
- Tokyo Night Claude-inspired task text.
- Ratatui persistent status bar task text.
- Pure crossterm/no-Ratatui instruction text.
- Existing modules for context bars, activity rails, old autocomplete, and Elma-specific progress indicators.

The user request now prioritizes Claude Code parity regardless of these older paradigms.

## Removal Or Quarantine Targets
Inspect and remove, replace, or move behind noninteractive compatibility flags:

- `src/ui_context_bar.rs`
- `src/ui_effort.rs`
- `src/ui_spinner.rs`
- `src/ui_progress.rs`
- `src/ui_interact.rs`
- `src/ui_autocomplete.rs` old dropdown behavior.
- `src/ui_render.rs` old five-frame renderer if replaced.
- `src/ui_terminal.rs` old event loop portions if replaced.
- `src/ui_trace.rs` direct interactive print helpers.
- Any hard-coded Gruvbox/Tokyo Night/Catppuccin/Rose Pine color helpers in active UI.

Do not delete useful noninteractive helpers if scripts/tests still need them. Move them behind explicit non-TTY paths or rename them to make their scope clear.

Do not delete context-management, compaction, token-budget, or llama.cpp reliability modules merely because they are Elma-specific. Preserve them unless they directly cause UI hangs, stale terminal state, or visible non-Claude UI behavior.

Be aggressive with UI compliance: if a module mixes raw terminal writes into the interactive renderer, prompts on stdin during TUI mode, or can block redraw/input handling, remove that path from interactive mode even if it requires a large refactor.

## Documentation Updates
Update:

- `AGENTS.md` or the repository-level guidance file that contains stale UI instructions.
- `_tasks/TASKS.md` task index if it tracks active UI direction.
- Active task notes for superseded UI tasks.
- Developer docs for the new theme and renderer architecture.
- Any README/usage docs that show the old Elma UI.

## Task System Cleanup
After Tasks 166-178 are accepted:

- [x] Mark `_tasks/active/110_Claude_Code_Style_Terminal_UI.md` as superseded or move it according to the repo's task protocol.
- [x] Mark `_tasks/active/132_Claude_Code_Inspired_UI.md` as superseded or move it according to the repo's task protocol.
- [x] Mark `_tasks/active/133_Ratatui_TUI_Gruvbox_Persistent_Status_Bar.md` as superseded or move it according to the repo's task protocol.
- Add notes to absorbed pending tasks if they are not removed.

Do not archive tasks as done without implementation and approval.

## Acceptance Criteria
- There is one documented interactive UI architecture.
- Old active UI tasks cannot be mistaken for current implementation instructions.
- Active interactive code uses the Claude-parity renderer and Pink/Cyan theme.
- Noninteractive output paths are explicit and tested.
- No user-facing default screen still shows the old five-frame Elma design.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test
rg -n "Gruvbox|Tokyo Night|Catppuccin|Rose Pine|context bar|activity rail|five-frame|5-frame" src docs AGENTS.md _tasks
./ui_parity_probe.sh --fixture startup
./ui_parity_probe.sh --fixture noninteractive-output
```

The `rg` command may return historical task references, but it must not reveal stale instructions in active implementation docs or active interactive UI code.
