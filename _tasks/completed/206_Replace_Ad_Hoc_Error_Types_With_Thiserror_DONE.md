# 206: Replace Ad-Hoc Error Types with `thiserror`

## Status
`pending`

## Crate
`thiserror` — Clean custom error types for libraries and modules.

## Rationale
Elma currently mixes `anyhow::Error` (application-level) with raw `String`/`Box<dyn Error>` in several modules. `thiserror` produces proper typed error enums that implement `std::error::Error`, making error propagation, matching, and testing far more reliable. It pairs naturally with `anyhow` (use `thiserror` for module-internal types, `anyhow` for application-level "anyhow" errors).

## Implementation Boundary
- Add `thiserror = "1.0"` to `Cargo.toml` dependencies.
- Audit existing error types in `src/types_core.rs`, `src/json_error_handler.rs`, `src/program_policy.rs`, `src/defaults_evidence.rs`, and any `Result<T, String>` or `Result<T, Box<dyn Error>>` patterns.
- Convert module-level errors to `thiserror` enums. Example:

  ```rust
  use thiserror::Error;

  #[derive(Error, Debug)]
  pub enum IntelUnitError {
      #[error("classification timeout after {elapsed_ms}ms")]
      ClassificationTimeout { elapsed_ms: u64 },
      #[error("invalid choice: {0}")]
      InvalidChoice(String),
      #[error(transparent)]
      JsonParse(#[from] serde_json::Error),
  }
  ```

- Expose `From` impls so `?` propagation works transparently.
- Keep public-facing methods returning `anyhow::Result` at the orchestrator boundary.
- Do NOT replace `anyhow` usage at the top-level CLI entry or session boundary.
- Verify: `cargo build` succeeds, no `#[error]` annotations added to existing `anyhow` call sites.

## Verification
- `cargo build` passes.
- Existing error-handling paths unchanged at the surface.
- New typed errors used in at least one module (e.g., `types_core.rs` or `json_error_handler.rs`).