# 229: Chainable Debug Transforms via `tap`

## Status
`pending`

## Crate
`tap` — Chainable `.tap()` debugging and transforms.

## Rationale
`tap` provides `.tap()` for inspecting values mid-expression without breaking the chain, and `.pipe()` for right-to-left function composition. Useful for debug logging of intermediate values in complex expressions, transforming data structures inline, and readable pipeline-style code. Small enough to be worth having everywhere.

## Implementation Boundary
- Add `tap = "1.0"` to `Cargo.toml`.
- Audit complex expressions or chained operations where intermediate values are hard to inspect (e.g., JSON deserialization → transformation → filtering).
- Add `.tap()` for inspection:

  ```rust
  use tap::Tap;

  let processed = data
      .tap(|d| tracing::debug!("input size: {}", d.len()))
      .transform()
      .tap(|r| tracing::trace!("output: {:?}", r));
  ```

- Use `.pipe()` for right-to-left composition where it improves readability.
- Keep `tap` usage in non-critical paths — do NOT add to hot loops.
- Do NOT replace `tracing` instrument points with `tap` — they serve different purposes.

## Verification
- `cargo build` passes.
- Existing code behavior unchanged.
- At least one `.tap()` usage in a pipeline.