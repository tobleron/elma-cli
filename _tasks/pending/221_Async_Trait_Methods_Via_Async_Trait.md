# 221: Async Trait Methods via `async-trait`

## Status
`pending`

## Crate
`async-trait` — Async functions in traits.

## Rationale
Rust does not yet support `async fn` in traits natively (stabilization is in progress but not complete). `async-trait` enables clean async trait methods without boxing to `Pin<Box<dyn Future>>`, keeping Elma's trait objects zero-cost. Useful for skill traits, tool trait objects, and provider abstractions.

## Implementation Boundary
- Add `async-trait = "0.1"` to `Cargo.toml`.
- Audit trait definitions in Elma that use `async fn` methods (likely in `src/providers/`, `src/skills/`, or tool traits).
- Apply `#[async_trait]` macro:

  ```rust
  use async_trait::async_trait;

  #[async_trait]
  pub trait SkillExecutor {
      async fn execute(&self, ctx: &SkillContext) -> anyhow::Result<SkillOutput>;
      async fn validate(&self) -> anyhow::Result<()>;
  }
  ```

- Prefer `async_trait` over `Pin<Box<dyn Future>>` for cleaner return types.
- Do NOT use `#[box_futures]` feature by default (adds heap allocation overhead).
- Do NOT replace all non-async traits — only apply where async methods are needed.

## Verification
- `cargo build` passes.
- At least one trait uses `#[async_trait]`.
- Async methods compile and execute correctly in tests.