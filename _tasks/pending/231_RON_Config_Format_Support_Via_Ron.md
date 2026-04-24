# 231: RON Config Format Support via `ron`

## Status
`pending`

## Crate
`ron` — Rusty Object Notation for readable configs.

## Rationale
RON is a human-readable, Rust-like config format (compatible with Serde). Useful as an alternative to TOML for user-facing config files that need nested structures and comments — RON supports trailing commas, multi-line strings, and struct shorthands that TOML doesn't. Good for skill config files or session archives that benefit from Rust-like syntax.

## Implementation Boundary
- Add `ron = "0.8"` to `Cargo.toml`.
- Create `src/config_ron.rs` with RON helpers:

  ```rust
  pub fn parse_ron<T: serde::de::DeserializeOwned>(content: &str) -> anyhow::Result<T> {
      ron::from_str(content).context("RON parse error")
  }

  pub fn to_ron<T: serde::Serialize>(value: &T) -> anyhow::Result<String> {
      ron::to_string(value).context("RON serialize error")
  }
  ```

- Add `.ron` extension as an optional config format alongside `.toml` and `.json`.
- Support `config/elma.ron` as an alternative to `elma.toml`.
- Do NOT replace TOML as the primary config format — add RON as an optional alternative.
- Do NOT add `ron_edit` — that is a follow-on if RON editing is needed.

## Verification
- `cargo build` passes.
- `ron` round-trips correctly through `parse_ron` and `to_ron`.
- `.ron` config files load correctly when present alongside `.toml`.