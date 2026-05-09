# Task 741: Replace Global Static Evidence Ledger With Session-Scoped Instance

## Type

Architecture / Reliability / Testing

## Severity

High

## Scope

Evidence system, session management, tool loop

## Problem

`src/evidence_ledger.rs` uses a **global mutable static** to hold the session's evidence:

```rust
static SESSION_LEDGER: OnceLock<RwLock<Option<EvidenceLedger>>> = OnceLock::new();

fn session_ledger() -> &'static RwLock<Option<EvidenceLedger>> {
    SESSION_LEDGER.get_or_init(|| RwLock::new(None))
}
```

This design has severe problems:

1. **Test contamination**: tests that use the evidence ledger can pollute each other because the static survives across test cases
2. **Session leakage**: if a session ends but the static isn't cleared, the next session inherits stale evidence
3. **No ownership**: the ledger has no clear owner; `init_session_ledger`, `clear_session_ledger`, and `with_session_ledger` operate on a global without lifecycle management
4. **Thread safety concerns**: `RwLock` around `Option` is awkward; panics or poisoned locks can leave the ledger in an invalid state
5. **Violates task persistence principle**: AGENTS.md Rule 9 says "Tasks Must Be Persisted, Not Memory-Only." The evidence ledger is the canonical source of evidence but lives in memory via a global static, with disk persistence as a secondary flush.

The docs say:
> "If a profile fails to load, Elma reports the error and falls back to defaults."

But if the evidence ledger fails (lock poisoned, static already initialized), there is no fallback — evidence is silently lost.

## Root Cause

The evidence ledger was designed for convenience (accessible from anywhere without passing references). This was a shortcut that avoided refactoring the tool loop to carry a ledger reference.

## Proposed Solution

### Phase 1 — Add ledger field to AppRuntime

1. Add `pub evidence_ledger: Option<EvidenceLedger>` to `AppRuntime`
2. Initialize it during bootstrap in `app_bootstrap.rs`
3. Pass `&mut EvidenceLedger` (or `&EvidenceLedger` for reads) into functions that need it

### Phase 2 — Refactor tool loop to use instance

1. `tool_loop.rs`: pass `&mut EvidenceLedger` through `run_tool_loop()` and related functions
2. `tool_calling.rs`: pass `&mut EvidenceLedger` into `execute_tool_call()`
3. `evidence_ledger.rs`: change `flush_tool_result()` to take `&mut EvidenceLedger` instead of using global
4. `app_chat_loop.rs`: use `runtime.evidence_ledger` instead of global functions

### Phase 3 — Delete global static API

1. Remove `SESSION_LEDGER` static
2. Remove `session_ledger()`, `init_session_ledger()`, `get_session_ledger()`, `with_session_ledger()`, `clear_session_ledger()`
3. Keep `persist_session_ledger()` but change signature to `fn persist(ledger: &EvidenceLedger) -> Result<()>`
4. Update `main.rs` if any re-exports reference the deleted functions

### Phase 4 — Fix tests

1. All tests that called `init_session_ledger()` must now create an `EvidenceLedger` instance
2. Tests can run in parallel without contamination
3. Add a test that verifies two independent ledgers don't share state

## Acceptance Criteria

- [ ] `SESSION_LEDGER` static is deleted from `evidence_ledger.rs`
- [ ] `AppRuntime` owns the session's `EvidenceLedger`
- [ ] No function in the tool loop accesses evidence via global state
- [ ] Tests run in parallel without evidence contamination
- [ ] Session end always persists and drops the ledger (no leakage)
- [ ] `cargo build && cargo test` passes
- [ ] Evidence persistence paths remain unchanged (`sessions/<id>/evidence/`)

## Verification Plan

- `grep -n "static SESSION_LEDGER" src/evidence_ledger.rs` → no match
- `grep -n "get_session_ledger\|with_session_ledger\|init_session_ledger\|clear_session_ledger" src/` → no matches outside of persistence functions
- Unit test: create two `EvidenceLedger` instances, add evidence to one, verify the other is empty
- Unit test: drop `AppRuntime`, verify ledger is persisted
- Integration test: run two sessions sequentially, verify second session starts with empty evidence

## Dependencies

- `src/app.rs` (AppRuntime struct)
- `src/app_bootstrap.rs` (initialization)
- `src/tool_loop.rs` (primary consumer)
- `src/tool_calling.rs` (secondary consumer)

## Notes

This refactor is tedious but high-value. The global static is a testing and reliability trap. Every new feature that touches evidence makes the problem worse.

Do not replace the global static with a different global abstraction (e.g., `thread_local!` or `Arc<Mutex<...>>`). The correct fix is instance-based ownership, following Rust's ownership principles.

The `EvidenceLedger::new()` constructor already takes `session_id` and `base_dir` — it was designed for instance use. The global static was an unnecessary wrapper.
