# 219: Better Serde Config Errors via `serde_path_to_error`

## Status
`pending`

## Crate
`serde_path_to_error` — Better Serde error paths for config files.

## Rationale
When `serde` fails to parse `elma.toml` or a skill config, the error is a generic "missing field" with no path context. `serde_path_to_error` annotates the error with the full field path (e.g., `profiles.default.temperature` instead of just `temperature`), making config debugging dramatically faster. Trivial to integrate with existing `serde` deserialization.

## Implementation Boundary
- Add `serde_path_to_error = "0.2"` to `Cargo.toml`.
- Audit all `serde` deserialization of config files (e.g., `toml::from_str`, `serde_json::from_str` in config loading).
- Wrap `serde` calls:

  ```rust
  use serde_path_to_error::de;

  fn load_config<T: de::DeserializeOwned>(content: &str) -> anyhow::Result<T> {
      let mut de = de::Deserializer::new(content)?;
      T::deserialize(&mut de).map_err(|e| {
          anyhow!("config error at {}: {}", e.path().display(), e)
      })
  }
  ```

- Apply to at least `config/elma.toml` loading and skill frontmatter parsing.
- Keep the existing error return type (`anyhow::Result`).
- Do NOT change config schema or add new fields.

## Verification
- `cargo build` passes.
- On a malformed config field, error message shows the full path (e.g., `profiles.default.model`).
- Existing config loading still works.