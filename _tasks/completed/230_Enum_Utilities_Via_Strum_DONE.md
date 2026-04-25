# 230: Enum Utilities via `strum`

## Status
`pending`

## Crrate
`strum` — Enum iteration, string conversion, and variant metadata.

## Rationale
Elma has many enums (speech acts, formula types, stop reasons, tool types, error variants from Task 206). `strum` provides `#[derive(EnumString, Display, EnumIter, EnumCount)]` that eliminate boilerplate `match` arms for `as_str()`, `from_str()`, and iteration over variants. Particularly valuable for `strum_macros::IntoStaticStr` on enums used in `serde` serialization.

## Implementation Boundary
- Add `strum = { version = "0.25", features = ["derive", "enum-string", "enum-iter", "enum-count"] }` to `Cargo.toml`.
- Audit enums in `src/types_core.rs`, `src/intel_units.rs`, and any routing/formula enums.
- Apply relevant `strum` derives:

  ```rust
  use strum::{Display, EnumString, EnumCount, EnumIter};

  #[derive(Display, EnumString, EnumCount, EnumIter, Debug, Clone, Serialize, Deserialize)]
  #[strum(serialize_all = "snake_case")]
  pub enum FormulaType {
      Reason,
      Act,
      Document,
      Scout,
      Steward,
  }
  ```

- Replace hand-written `FromStr`, `as_str()`, and variant count logic with `strum` derives.
- Do NOT apply `strum` to enums with non-trivial variant data requiring custom logic.

## Verification
- `cargo build` passes.
- `"reason".parse::<FormulaType>()` works via `EnumString`.
- `FormulaType::COUNT == 5` via `EnumCount`.
- Iteration over variants works via `EnumIter`.
- Existing serde round-trips unchanged.