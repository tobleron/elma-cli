# 227: Lazy Static Initialization via `once_cell`

## Status
`pending`

## Crrate
`once_cell` — Lazy global initialization.

## Rationale
`std::sync::LazyLock` (stable since Rust 1.80) covers most cases, but `once_cell::sync::Lazy` supports older toolchains and has `INIT` patterns that `LazyLock` lacks. Useful for global singletons: the Ratatui theme, the skill registry, the formula catalog, and other module-level state that should be initialized once on first access.

## Implementation Boundary
- Add `once_cell = "1.20"` to `Cargo.toml`.
- Audit module-level `std::sync::LazyLock` or `Mutex::new` initialization patterns.
- Replace where applicable:

  ```rust
  use once_cell::sync::Lazy;
  use std::sync::Mutex;

  static SKILL_REGISTRY: Lazy<Mutex<SkillRegistry>> = Lazy::new(|| Mutex::new(SkillRegistry::default()));
  static THEME: Lazy<Theme> = Lazy::new(Theme::default);
  ```

- Prefer `std::sync::LazyLock` for new code where the MSRV supports it; use `once_cell` for cases needing `INIT`-style fallback or older toolchain compatibility.
- Do NOT introduce `once_cell` into hot paths — initialization cost must be negligible.
- Do NOT use `once_cell::unsync::Lazy` for anything already wrapped in `Mutex`/`RwLock`.

## Verification
- `cargo build` passes.
- Lazy globals initialize exactly once across the lifetime of a session.
- No deadlocks or initialization races.