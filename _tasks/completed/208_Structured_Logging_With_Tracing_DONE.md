# 208: Structured Logging with `tracing`

## Status
`pending`

## Crate
`tracing` + `tracing-subscriber` — Structured logging and instrumentation.

## Rationale
Elma currently uses `println!`/`eprintln!` and ad-hoc logging for diagnostics. `tracing` provides structured, span-based instrumentation with levels, targets, and field data — and `tracing-subscriber` lets you configure output (pretty console, JSON, file). This enables production-grade observability without bloating the transcript.

## Implementation Boundary
- Add `tracing = "0.1"` and `tracing-subscriber = "0.2"` (with `fmt`, `env-filter` features) to `Cargo.toml`.
- Create `src/logging.rs` that initializes the subscriber in `main.rs`/`lib.rs`:

  ```rust
  use tracing_subscriber::{fmt, prelude::*, EnvFilter};

  pub fn init_logging(verbose: bool) {
      let filter = EnvFilter::try_from_default_env()
          .unwrap_or_else(|_| EnvFilter::new(if verbose { "elma_cli=debug" } else { "elma_cli=info" }));
      tracing_subscriber::registry()
          .with(fmt::layer().with_ansi(true))
          .with(filter)
          .init();
  }
  ```

- Replace at least 3 `println!`/`eprintln!` diagnostic calls in `src/main.rs` or `src/lib.rs` with `tracing::info!`, `tracing::debug!`, `tracing::error!`.
- Instrument at least one function with `#[instrument]` (e.g., the main message-processing loop).
- Keep the logging silent by default; verbose mode via `--verbose` or `RUST_LOG`.
- Do NOT log user messages or LLM responses — keep transcript as the only user-facing output channel.
- Do NOT add `tracing-appender` yet; that is a follow-on task.

## Verification
- `cargo build` passes.
- `elma --verbose` emits structured log lines to stderr.
- Default run emits no log output.
- Existing CLI output unchanged.