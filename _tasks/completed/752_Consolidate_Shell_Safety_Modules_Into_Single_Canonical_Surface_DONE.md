# Task 752: Consolidate Shell Safety Modules Into Single Canonical Surface

## Type

Architecture / Reliability / Safety

## Severity

High

## Scope

Shell execution, command validation, permission system

## Problem

Shell command safety logic is scattered across **four separate modules** with overlapping responsibilities:

1. **`src/shell_preflight.rs`** (1125 lines, 964 lines of logic)
   - Risk classification: Safe / Caution / Dangerous
   - Destructive pattern detection (`rm `, `git reset --hard`, etc.)
   - Pipe chain analysis (`| xargs rm`)
   - While-loop destructive keyword detection
   - Unscoped operation detection (`find` without `-maxdepth`)
   - Protected path blocking (`sessions/`, `config/`, `.git/`)
   - Dry-run preview
   - Confirmation cache

2. **`src/execution_steps_shell_preflight.rs`** (4784 lines? No, `ls` shows it exists but `wc` didn't show it; likely smaller)
   - Unknown contents — module exists but size wasn't in top files
   - Likely duplicates or extends `shell_preflight.rs`

3. **`src/shell_exec_policy.rs`** (exists, unknown size)
   - Task 658: "Parser-backed shell execution policy and permission cache"
   - Another layer of shell policy

4. **`src/execution_steps_shell_exec.rs`** (25606 lines / bytes? Wait, `ls` shows 25606 bytes, which is ~600-800 lines)
   - Legacy shell execution
   - Called by `execution_steps_shell.rs`

This fragmentation creates:
- **Reliability hazard**: safety rules can diverge between modules
- **Maintenance burden**: a fix in one module may not reach the others
- **Confusion**: developers don't know which module to edit
- **Test gaps**: tests for shell safety may only cover one module

The docs call out `shell_preflight.rs` as part of the security & permissions layer, but don't mention the duplication. Task 658 ("Parser-backed shell execution policy") was supposed to unify this but appears to have created yet another module instead of consolidating existing ones.

## Root Cause

Incremental development without consolidation. Each new task (116, 118, 119, 120, 658) added a new file rather than refactoring the existing one. The "de-bloating" guidance in DEVELOPMENT_GUIDELINES.md was not applied.

## Proposed Solution

### Phase 1 — Audit all four modules

1. Read `execution_steps_shell_preflight.rs`, `shell_exec_policy.rs`, and `execution_steps_shell_exec.rs`
2. Map every function to its semantic responsibility (classification, validation, execution, caching)
3. Identify exact duplications
4. Identify which module is the "live" path in production (which one does `tool_calling.rs` call?)

### Phase 2 — Choose canonical policy surface

The canonical policy surface should be **`src/shell_preflight.rs`** or a small `src/shell_policy/` module if the implementation is too large for one file. Execution adapters may remain separate, but command classification, validation, permission caching, and protected-path policy must have one owner.

`src/shell_preflight.rs` is the likely owner because:
- It has the most complete implementation
- It has tests
- It has the confirmation cache and dry-run logic
- It is referenced from `app_chat_loop.rs` (`clear_confirmation_cache`)

Move all unique policy logic from the other modules into the canonical surface. Keep execution-only code in execution modules if it truly performs execution rather than policy decisions.

### Phase 3 — Delete or narrow redundant modules

1. Delete `src/execution_steps_shell_preflight.rs` if it duplicates policy logic.
2. Delete `src/shell_exec_policy.rs` or turn it into a thin compatibility wrapper if call-site migration needs to be staged.
3. Evaluate `execution_steps_shell_exec.rs`: if it is truly legacy and superseded by `execution_steps_shell.rs`, mark it `#[deprecated]` and plan deletion; if it is the live execution adapter, keep it but remove policy decisions from it.
4. Update `main.rs` to remove deleted modules

### Phase 4 — Unify the public API

`src/shell_preflight.rs` should expose a single, clean API:

```rust
pub(crate) fn validate_shell_command(cmd: &str, workdir: &Path) -> PreflightResult;
pub(crate) fn clear_confirmation_cache();
pub(crate) fn is_path_protected(path: &Path) -> bool;
```

All internal logic (pattern matching, pipe analysis, unscoped detection) should be private.

### Phase 5 — Harden the canonical module

Per Rule 13 and Task 658 requirements:

1. Replace keyword-based destructive detection with **command syntax parsing** (use `shlex` or a lightweight shell parser)
2. Distinguish syntax parsing (what the command actually does) from intent keyword matching (what words appear in the objective)
3. Add programmatic post-conditions: after `validate_shell_command`, the result must be verifiable without re-parsing
4. Add tests that prove the parser understands command structure, not just substrings

## Acceptance Criteria

- [ ] Exactly one canonical shell policy surface owns classification, validation, permission caching, and protected paths
- [ ] Execution modules do not duplicate shell safety decisions
- [ ] `tool_calling.rs` and `execution_steps_shell.rs` call only the canonical module
- [ ] The canonical module's public API has ≤ 5 functions
- [ ] Keyword-based destructive detection is replaced with syntax-aware parsing
- [ ] `cargo build && cargo test` passes
- [ ] All existing shell safety tests pass (or are migrated to the canonical module)

## Verification Plan

- Audit command shows one canonical policy owner; any remaining shell execution modules contain execution only, not policy duplication
- Unit test: `validate_shell_command("rm -rf /")` → Dangerous
- Unit test: `validate_shell_command("echo 'rm -rf /'")` → Safe (syntax parsing, not keyword matching)
- Unit test: `validate_shell_command("find . | xargs rm")` → Dangerous (pipe analysis)
- Integration test: destructive command triggers dry-run preview

## Dependencies

- Task 658 (Parser-backed shell execution policy) — should be absorbed, not duplicated
- `src/execution_steps_shell.rs` (live execution path)
- `src/tool_calling.rs` (tool dispatch)

## Notes

This task is about **consolidation first, enhancement second**. Do not expand scope to redesign the permission system or add new safety features. The goal is one canonical module with one clear API.

The keyword-based detection in `shell_preflight.rs` (`DESTRUCTIVE_PATTERNS`, `PIPE_DESTRUCTIVE_PATTERNS`) is acceptable as a **syntax parsing aid** only if it is backed by actual command structure analysis. A command containing the substring `"rm "` is not necessarily destructive (e.g., `echo "rm foo"`). Task 658's parser-backed approach should be the primary mechanism; keyword lists should only be fast-path hints, not authoritative decisions.
