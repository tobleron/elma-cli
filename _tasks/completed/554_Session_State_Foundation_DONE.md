# 005 — Replace Global Mutable State with Session-Scoped Ownership

- **Priority**: High
- **Category**: State Management
- **Depends on**: 001 (to simplify state surfaces)
- **Blocks**: 017, 028

## Problem Statement

The codebase uses `static OnceLock<Mutex<T>>` and `static OnceLock<RwLock<Option<T>>>` extensively as global mutable state. Examples:

```rust
// evidence_ledger.rs
static SESSION_LEDGER: OnceLock<RwLock<Option<EvidenceLedger>>> = OnceLock::new();

// event_log.rs
static CURRENT_TURN_ID: OnceLock<RwLock<Option<String>>> = OnceLock::new();

// permission_gate.rs
static PERMISSION_CACHE: OnceLock<Mutex<ApprovalCache>> = OnceLock::new();

// shell_preflight.rs
static CONFIRMED_COMMANDS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(...);

// safe_mode.rs
static SAFE_MODE: OnceLock<Mutex<SafeMode>> = OnceLock::new();

// command_budget.rs
static BUDGET: OnceLock<Mutex<CommandBudget>> = OnceLock::new();

// execution_profiles.rs
static EXECUTION_PROFILE: OnceLock<RwLock<Option<ExecutionProfile>>> = OnceLock::new();

// tool_registry.rs (elma-tools)
static DISCOVERED_TOOLS: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

// ui_state.rs
static PROGRESS_INDICATOR: OnceLock<Mutex<Option<ToolProgress>>> = OnceLock::new();
```

This pattern causes:
1. **Untestability**: Tests share global state, requiring careful cleanup between tests
2. **Hidden coupling**: Functions silently depend on global state initialized elsewhere
3. **No concurrent sessions**: The globals prevent running multiple Elma sessions in one process
4. **Debugging difficulty**: State mutations are invisible in stack traces
5. **Poisoning risk**: `Mutex::lock()` can panic if a previous thread panicked while holding the lock

## Why This Matters for Small Local LLMs

- Small model interactions are inherently stateful (conversation context, tool history, evidence accumulation)
- Global state makes it impossible to run parallel test scenarios or A/B test different prompt strategies
- State corruption from one bad model interaction can poison subsequent turns because state is global rather than session-scoped

## Current Behavior

Functions throughout the codebase call global accessors like:
```rust
let ledger = crate::evidence_ledger::get_session_ledger();
let budget = crate::command_budget::get_budget();
let mode = crate::safe_mode::get_safe_mode();
```

## Recommended Target Behavior

Create a `SessionState` struct that owns all mutable state for a session:

```rust
pub struct SessionState {
    pub evidence_ledger: Option<EvidenceLedger>,
    pub event_log: Option<SessionEventLog>,
    pub permission_cache: ApprovalCache,
    pub confirmed_commands: HashSet<String>,
    pub safe_mode: SafeMode,
    pub command_budget: CommandBudget,
    pub execution_profile: Option<ExecutionProfile>,
    pub discovered_tools: HashSet<String>,
    pub stop_policy: Option<StopPolicy>,
    // ... etc
}
```

Pass `SessionState` (or `&mut SessionState`) through the call chain instead of accessing globals.

For truly global configuration (safe_mode default, profile paths), use a read-only `OnceLock` without mutex:

```rust
static CONFIG: OnceLock<ElmaConfig> = OnceLock::new(); // set once at startup, read-only after
```

## Source Files That Need Modification

- `src/evidence_ledger.rs` — Remove global `SESSION_LEDGER`, accept `&mut EvidenceLedger` as parameter
- `src/event_log.rs` — Remove global `CURRENT_TURN_ID`, accept state parameter
- `src/permission_gate.rs` — Remove global `PERMISSION_CACHE`, accept `&mut ApprovalCache`
- `src/shell_preflight.rs` — Remove global `CONFIRMED_COMMANDS`
- `src/safe_mode.rs` — Keep read-only global config, move mutable mode to session state
- `src/command_budget.rs` — Remove global `BUDGET`
- `src/execution_profiles.rs` — Keep read-only profile config, move mutable state
- `src/tool_registry.rs` — Keep registry global (read-only after init), move `DISCOVERED_TOOLS`
- `src/ui_state.rs` — Move progress indicators to TUI-local state
- All callers of the above globals — Update function signatures

## New Files/Modules

- `src/session_state.rs` — `SessionState` struct definition, initialization

## Step-by-Step Implementation Plan

1. Create `src/session_state.rs` with `SessionState` struct
2. Add `SessionState` field to `AppRuntime` (already exists in `app.rs`)
3. One-by-one, for each global:
   a. Add the field to `SessionState`
   b. Initialize it in `SessionState::new()`
   c. Update the accessor function to accept `&mut SessionState` or `&SessionState`
   d. Update all call sites
   e. Remove the global static
4. For read-only config (safe_mode, profile paths, etc.):
   a. Keep as `OnceLock` without `Mutex`
   b. Set once at startup
5. Verify with `cargo check` after each migration
6. Run full test suite

## Recommended Crates

None new — this is a refactoring task.

## Validation/Sanitization Strategy

- SessionState must be `Send` (already satisfied by existing types)
- No `Rc` or `RefCell` in SessionState (must be thread-safe)
- Use `#[cfg(test)]` helpers to create test SessionState instances

## Testing Plan

1. After migration, run all existing tests
2. Add test that creates two SessionState instances independently
3. Verify no state leakage between sequential operations
4. Property: `SessionState::new()` followed by operations should never access uninitialized state

## Acceptance Criteria

- Zero global `static ... Mutex<...>` or `static ... RwLock<Option<...>>` for session-scoped state
- All session state flows through `SessionState` or `&mut SessionState` parameters
- Read-only configuration uses `OnceLock` without interior mutability
- All existing tests pass
- New test verifies two independent sessions don't share state

## Risks and Migration Notes

- **Very high touch surface**: ~15+ globals across 15+ files. Do this incrementally in small PRs.
- **Breaking change risk**: Functions that currently access globals will need new parameters. Use the "add parameter, remove global, fix callers" pattern.
- **`tool_loop.rs` complexity**: The `run_tool_loop` function is the biggest consumer of globals. Refactor it to accept a `&mut SessionState`.
- **TUI coupling**: The TUI layer also accesses some globals (`ui_state`). These should move to the `TerminalUI` struct itself.
- **Migration order**: Do `command_budget` and `confirmed_commands` first (simplest), `evidence_ledger` and `event_log` next (medium), `permission_cache` last (most complex).
