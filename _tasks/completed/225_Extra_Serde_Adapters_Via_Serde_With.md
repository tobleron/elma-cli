# 225: Extra Serde Adapters via `serde_with`

## Status
`pending`

## Crate
`serde_with` — Extra Serde adapters for common formats.

## Rationale
Elma's config and skill frontmatter use diverse types (timestamps, byte sizes, durations, base64, hex strings). `serde_with` provides `#[serde_as]` macros that handle conversions like `Option<DateTime<Utc>>`, `u64 as bytes`, `Vec<u8> as hex` without hand-rolling custom serializers. Reduces boilerplate significantly.

## Implementation Boundary
- Add `serde_with = "3.0"` to `Cargo.toml`.
- Audit Serde serialization/deserialization in `src/types_core.rs`, config parsing, and skill frontmatter.
- Apply `#[serde_with]` to structs that handle timestamps, byte counts, or base64 data:

  ```rust
  use serde_with::{serde_as, DisplayFromStr, DurationSeconds};
  use std::time::Duration;

  #[serde_with]
  #[derive(Serialize, Deserialize)]
  pub struct SkillMetadata {
      #[serde_as(as = "Option<DisplayFromStr>")]
      pub timeout_sec: Option<u64>,
      #[serde_as(as = "DurationSeconds<u64>")]
      pub ideal_duration: Option<Duration>,
      #[serde_as(as = "Vec<DisplayFromStr>")]
      pub tags: Vec<String>,
  }
  ```

- Replace at least two custom `serialize_with`/`deserialize_with` implementations with `serde_with` equivalents.
- Keep existing Serde behavior — `serde_with` is additive.

## Verification
- `cargo build` passes.
- Existing config/skill deserialization unchanged.
- New adapters compile and round-trip correctly.