# 214: Beautiful Panic Reports via `color-eyre`

## Status
`pending`

## Crate
`color-eyre` — Beautiful panic/error reports for terminal apps.

## Rationale
Elma's current panic handler produces a raw Rust panic message. `color-eyre` wraps `eyre` with color, source highlighting, and a chain of errors — producing panic reports that match the premium terminal UX standard. It integrates with `miette` (Task 209) and works with the existing `anyhow` setup.


## Implementation Boundary
- Add `color-eyre = "0.6"` to `Cargo.toml`.
- In `src/main.rs`, replace `std::panic::set_hook` with `color_eyre::install()`:

  ```rust
  use color_eyre::install;

  fn main() {
      if let Err(e) = run() {
          eprintln!("{e}");
          std::process::exit(1);
      }
  }

  fn run() -> color_eyre::Result<()> {
      install()?;
      // ... rest of app
  }
  ```

- Use `color_eyre::Result<T>` as the top-level error type for `run()`, converting inner `anyhow::Result` calls via `.into()`.
- Keep the return code as 1 on error, 0 on success.
- Ensure panic traces render with color when the terminal supports it.
- Do NOT replace per-module error types (Tasks 206/209) — `color-eyre` is the display layer.
- Do NOT change error return types for internal functions — only the top-level `main`/`run` boundary.

## Verification
- `cargo build` passes.
- Trigger a panic (e.g., `panic!("test")`) and verify output is colored and formatted with `color-eyre`.
- Normal operation (no panic) is unchanged.
- Works correctly in both dark and light terminals.