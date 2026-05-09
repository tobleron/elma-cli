# Task 781: Remove Unsafe Static Mut in experimental_reasoning.rs

## Type
Safety / Code Quality

## Severity
High

## Scope
`src/experimental_reasoning.rs`

## Problem

`experimental_reasoning.rs:55-59` contains `static mut COUNTER: u64` accessed via `unsafe` block in the `randish()` function. This is unsound in a multi-threaded Tokio runtime — concurrent access to `static mut` is undefined behavior.

```rust
fn randish() -> f64 {
    static mut COUNTER: u64 = 0;
    unsafe {
        COUNTER = COUNTER.wrapping_add(1);
        (COUNTER as f64) / (u64::MAX as f64)
    }
}
```

## Root Cause

Quick implementation of a pseudo-random number generator without considering thread safety. The function exists in the `CreativeRecovery::vary_temperature()` path.

## Proposed Solution

Replace `static mut` with `AtomicU64::fetch_add(1, Ordering::Relaxed)`:

```rust
fn randish() -> f64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let val = COUNTER.fetch_add(1, Ordering::Relaxed);
    (val as f64) / (u64::MAX as f64)
}
```

This is lock-free, sound, and produces the same sequence.

## Acceptance Criteria
- [ ] No `static mut` or `unsafe` block in `experimental_reasoning.rs`
- [ ] `randish()` uses `AtomicU64` instead
- [ ] `cargo test` passes
- [ ] `cargo clippy` reports no new warnings

## Verification Plan
- Unit test: `test_vary_temperature_stays_in_bounds` still passes
- Integration test: `cargo build`
- Regression test: `cargo clippy -- -D warnings`

## Dependencies
None.

## Notes
- **Architectural Rule violated:** Rule 13 (Reliability & Hardening) — unsafe code is forbidden when safe alternatives exist
- This is the **only** `static mut` in the entire codebase (confirmed by grep)
