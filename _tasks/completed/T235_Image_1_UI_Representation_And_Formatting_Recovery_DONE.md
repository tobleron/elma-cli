# Task T235: Image #1 UI Representation And Formatting Recovery

## Status
Active.

## Priority
P0 troubleshooting.

## Objective
Fix the Claude-style transcript representation defects shown in Image #1:
- assistant gutter drifting onto a blank line when content begins with a fence
- overly heavy fixed-width code fence rendering
- shell suggestions looking like tool traces instead of suggested commands
- prompt-clear hints polluting transcript history
- footer overcrowding with long model labels and transcript metrics
- live thinking regressing away from the older expanded streaming format
- shell/tool execution leaking terminal debris instead of staying isolated and compact

## Scope
- typed assistant block parsing and spacing normalization
- typed notice model with ephemeral prompt hints
- responsive footer view model
- PTY-isolated shell capture with sanitized transcript output
- compact-by-default completed tool traces with expandable transcript details
- parity fixture and unit-test coverage for the screenshot-shaped regression

## Progress Notes
- Added `AssistantContent` / `AssistantBlock` parsing and normalization in `src/claude_ui/claude_markdown.rs`.
- Converted assistant transcript rows to render from normalized blocks so the `●` gutter attaches to the first visible line.
- Converted transcript telemetry to typed notices and moved prompt-clear hints into an ephemeral footer-adjacent lane.
- Replaced string-built footer status with a typed `FooterModel` and width-aware compaction.
- Restored live `Thinking` streaming directly into the expanded transcript layout while a turn is in progress.
- Switched shell execution capture to a PTY-backed subterminal path in `src/program_utils.rs`, with ANSI/control-sequence sanitization and fallback sanitization for the redirected path.
- Made completed tool traces auto-collapse again so shell activity stays compact until the user expands it.
- Added regression coverage for shell command extraction, blank-line normalization, orphan `command` cleanup, and footer compaction.
- Added transcript sanitization tests covering ANSI stripping and carriage-return rewrite behavior.

## Verification
- `cargo fmt`
- `cargo build`
- `cargo test program_utils::tests -- --nocapture`
- `cargo test claude_ -- --nocapture`
- PTY parity run executed before interruption:
  - `double_escape_clear_fixture` passed
  - `notification_fixture` passed
  - `status_line_fixture` passed
  - full `cargo test --test ui_parity -- --nocapture` reached 25 passing tests and 1 unrelated failure in `snapshot_tests::startup_snapshot` due existing snapshot drift (`tests/snapshots/ui_parity__snapshot_tests__startup.snap`)
