# 209: Diagnostic Errors with Source Spans via `miette`

## Status
`pending`

## Crate
`miette` — Diagnostic errors with source spans and pretty output.

## Rationale
Elma's current error output is flat strings. `miette` enables error reports with source-code location highlighting, labeled spans, and help suggestions — much richer than `anyhow` alone. It integrates with `anyhow` via `miette::Report` and works well in terminal UIs via `miette::IntoDiagnostic`.

## Implementation Boundary
- Add `miette = "7.0"` (or current) to `Cargo.toml`.
- Create `src/diagnostics.rs` with a `Diagnostic` enum for Elma-specific errors:

  ```rust
  use miette::{LabeledSpan, Severity};
  use thiserror::Error;

  #[derive(Error, Diagnostic, Debug)]
  pub enum ElmaDiagnostic {
      #[error("invalid skill file: {name}")]
      #[help("skill files must have a .md extension and contain frontmatter")]
      InvalidSkillFile {
          name: String,
          #[label]
          span: SourceSpan,
      },
      #[error("config parse error")]
      #[diagnostic]
      ConfigError(#[from] toml::de::Error),
  }
  ```

- Use `IntoDiagnostic` on existing `anyhow::bail!` calls in config parsing, skill loading, and `json_error_handler.rs`.
- Keep the existing error surface (return types unchanged) — `miette` enhances the display without changing the API.
- Prefer `ElmaDiagnostic` for internal validation errors; keep `anyhow` for external/user-facing "something went wrong" cases.
- Do NOT replace all `anyhow` usage — only add `miette` where source context is useful.

## Verification
- `cargo build` passes.
- At least one error path (e.g., skill file parsing or config loading) renders with `miette` spans when it fails.
- Error display degrades gracefully if terminal doesn't support ANSI (falls back to plain text).