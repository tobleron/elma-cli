# 215: Terminal Dimension and Interaction Helpers via `console`

## Status
`pending`

## Crate
`console` — Terminal styling, dimensions, and interaction helpers.

## Rationale
`console` provides `console::Term` for reading terminal dimensions, cursor position, and key events — plus width-aware string truncation and ANSI strip utilities. Elma's Ratatui renderer already handles most of this, but `console` fills gaps: measuring terminal width for non-TUI output (e.g., `--help` text wrapping, progress bar terminal width), stripping ANSI codes for log files, and detecting terminal capabilities.

## Implementation Boundary
- Add `console = "0.18"` to `Cargo.toml`.
- Identify non-TUI output paths that hardcode terminal assumptions:
  - `println!` output in `--help` or config error messages
  - Any manual ASCII art or separator rendering
  - Error messages with hardcoded line widths
- Replace hardcoded widths with `console::Term::stdout().size().0` (columns).
- Use `console::strip_ansi_codes()` on strings before writing to log files (complements Task 208).
- Use `console::user_attended()` to detect if stdout is a TTY vs. piped — useful for conditionally enabling color output.
- Do NOT replace Ratatui's own terminal dimension handling.
- Do NOT use `console::Term` for interactive TUI input — Ratatui owns that contract.

## Verification
- `cargo build` passes.
- `--help` output respects actual terminal width.
- `console::user_attended()` returns correct value for TTY vs. piped execution.