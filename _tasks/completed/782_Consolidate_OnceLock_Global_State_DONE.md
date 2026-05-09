# Task 782: Audit and Consolidate OnceLock Global State Proliferation

## Type
Architecture / Maintainability

## Severity
Medium

## Scope
System-wide

## Problem

The codebase has **45 `OnceLock` statics** and **5 `LazyLock` statics** scattered across ~30 modules. While each individual use may be justified, the aggregate effect is:

1. **Hidden coupling** — modules communicate through global state rather than explicit parameters
2. **Test contamination** — globals leak state between test cases unless explicitly reset
3. **Init-order fragility** — some globals depend on others being initialized first, but nothing enforces the order
4. **Impossible mocking** — OnceLock values cannot be changed once set, making integration testing with different configurations difficult

**Highest-risk globals (mutable via Mutex/RwLock wrapping):**

| Module | Global | Concern |
|--------|--------|---------|
| `extension_gateway.rs` | `EXTENSIONS: OnceLock<Mutex<Vec>>` | Mutable global vec |
| `safe_mode.rs` | `SAFE_MODE_STATE: OnceLock<Mutex<SafeMode>>` | Runtime mode toggle |
| `json_error_handler.rs` | `ERROR_HANDLER: OnceLock<Mutex<JsonErrorHandler>>` | Mutable error state |
| `workspace_policy.rs` | `SCOPE_CONSTRAINT: OnceLock<RwLock<Option<ScopeConstraint>>>` | Mutable scope |
| `mutation_contract.rs` | `HAS_MUTATED: OnceLock<RwLock<bool>>` | Session-wide flag |
| `online_verification.rs` | `NETWORK_DISABLED: OnceLock<RwLock<bool>>` | Network toggle |
| `artifact_verifier.rs` | 3× OnceLock statics | Deliverable tracking |
| `event_log.rs` | `SESSION_EVENT_LOG` + `CURRENT_TURN_ID` | Session state |

## Root Cause

Incremental task-driven development introduced globals per-module without a central registry pattern. Each task solved its own state-passing problem by adding a new global.

## Proposed Solution

Phase 1: **Inventory** — Create a canonical list of all globals with justification status:
  - **Justified**: Truly init-once data (e.g., `CL100K` tokenizer, `SYNTAX_SET`, `THEME_SET`)
  - **Questionable**: Mutable state wrapped in OnceLock<Mutex/RwLock> — should be session-scoped
  - **Should delete**: Unwired module globals (cross-reference with Task 778)

Phase 2: **Extract session-scoped state** — Move mutable globals (`HAS_MUTATED`, `SAFE_MODE_STATE`, `SCOPE_CONSTRAINT`, `SESSION_EVENT_LOG`, etc.) into a `SessionState` struct passed through the runtime. This makes tests deterministic and enables multi-session support.

Phase 3: **Add reset contract** — For any remaining justified globals, add a `reset_for_testing()` function and call it in test setup.

## Acceptance Criteria
- [ ] All mutable globals are inventoried with justification
- [ ] At least 10 mutable globals are migrated to session-scoped state
- [ ] Test isolation is verified — running `cargo test` twice produces identical results
- [ ] No new OnceLock<Mutex/RwLock> patterns are introduced without review

## Verification Plan
- Unit test: `cargo test` passes deterministically
- Integration test: Run tests in random order (`cargo test -- --test-threads=1`)
- Regression test: `cargo clippy`

## Dependencies
- Task 778 (Purge Unwired Modules) should run first to eliminate dead globals

## Notes
- **Architectural Rule violated:** Rule 4 (Explicit State Threading) — globals hide dataflow and make reasoning about system state impossible
- Some globals are genuinely correct (tokenizer BPE data, syntax highlighting themes) — don't blindly delete
- The 50 OnceLock count is **5x** what a codebase this size should have
