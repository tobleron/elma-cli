# 228: Derive Macro Utilities via `derive_more` and `itertools`

## Status
`pending`

## Crates
`derive_more` (#219) — Derive common trait boilerplate (`Display`, `From`, `IntoIterator`, arithmetic ops).
`itertools` (#225) — Extra iterator adapters (`kmerge`, `group_by`, `unique`, `intersperse`, etc.).

## Rationale
`derive_more` eliminates repetitive `impl Display`, `impl From<T>`, `impl Add`, etc. that Elma maintains manually. `itertools` provides battle-tested iterator combinators that are significantly more ergonomic than hand-rolled loops. Both are zero-cost abstractions with no runtime overhead.

## Implementation Boundary
- Add `derive_more = "0.99"` and `itertools = "0.13"` to `Cargo.toml`.
- Audit `impl Display` blocks in `src/types_core.rs` and other modules — replace with `#[derive_more::Display]` where straightforward.
- Audit loops over collections that could use `itertools`: grouping file paths, merging result sets, deduplication.
- Example `derive_more`:

  ```rust
  use derive_more::{Display, From, Add};

  #[derive(Display, From, Add)]
  #[display("IntelUnitError: {kind} at stage {stage}")]
  pub struct IntelUnitError {
      kind: String,
      stage: u8,
  }
  ```

- Example `itertools`:

  ```rust
  use itertools::Itertools;
  let unique_tags: Vec<_> = all_tags.into_iter().unique().collect();
  ```

- Do NOT replace hand-written impls that do non-trivial logic.
- Do NOT add `itertools` to hot paths without benchmarking.

## Verification
- `cargo build` passes.
- Existing functionality unchanged.
- At least two `derive_more` derives and two `itertools` usages in the codebase.