# 232: TOML Editing with Formatting Preservation via `toml_edit`

## Status
`pending`

## Crate
`toml_edit` — Edit TOML while preserving formatting and comments.

## Rationale
Elma writes its own config files (`elma.toml`, skill configs). Using raw `toml` to serialize from scratch loses user comments and custom formatting. `toml_edit` is the underlying engine of the `toml` crate's serializer and allows surgical edits: update a single key without rewriting the whole file, preserving comments and structure. Essential for Elma to safely write config updates without destroying user annotations.

## Implementation Boundary
- Add `toml_edit = "0.22"` to `Cargo.toml`.
- Audit config write paths (e.g., saving user preferences, skill config updates, session metadata).
- Create `src/config_edit.rs`:

  ```rust
  use toml_edit::{DocumentMut, Item};

  pub fn update_toml_key(path: &str, key: &str, value: &str) -> anyhow::Result<String> {
      let mut doc: DocumentMut = path.read_to_string()?.parse()?;
      doc["profiles"]["default"][key] = value.into();
      Ok(doc.to_string())
  }
  ```

- Replace at least one raw `toml::to_string` config write with `toml_edit` for preservation.
- Keep `toml::to_string` for initial file creation where no preservation is needed.
- Do NOT use `toml_edit` for reading — keep `toml::from_str` for that.

## Verification
- `cargo build` passes.
- Updating a key in a TOML file preserves comments and formatting of other keys.
- Existing config write behavior unchanged for new files.