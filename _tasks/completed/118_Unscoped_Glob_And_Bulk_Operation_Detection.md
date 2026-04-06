# Task 118: Unscoped Glob & Bulk Operation Detection

## Priority
**P1 — Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection)

## Problem

`find . -type f -name "*.sh" | while read f; do mv "$f" "stress_testing/"; done` matched 641 files across the entire project tree — not just root-level scripts. The model didn't scope its operation and would have moved files from `_scripts/`, `tests/`, session artifacts, and subdirectories.

## Scope

### 1. Unscoped Pattern Detection
- Flag commands that operate on the entire tree without scoping:
  - `find .` without `-maxdepth`
  - `rm *` or `mv *` at project root
  - `rg --no-heading` on large directories
  - Pipeline patterns: `find ... | xargs ...`, `find ... | while read ...`

### 2. Match Count Estimation
- Before executing: run the read-only equivalent to count matches
  - `find . -name "*.sh" | wc -l` before `find . -name "*.sh" | xargs mv`
- If >20 matches: warn user with count, require confirmation
- If >100 matches: require explicit confirmation with file listing preview

### 3. Scoping Suggestions
- When unscoped command detected, suggest scoped alternative:
  - `find .` → `find . -maxdepth 1` (root only)
  - `rm *` → `rm *.sh` (specific extension)
  - Model receives suggestion as error feedback

### 4. Integration Points
- `src/shell_preflight.rs` — extend with unscoped detection
- `src/tool_calling.rs` — `exec_shell()` calls count estimation for bulk ops

## Design Principles
- **Truthful over polished:** Show real match counts, not estimates
- **Small-model-friendly:** Concrete numbers ("641 files match") not vague warnings ("this seems broad")
- **No keyword routing:** Structural pattern analysis, not word matching

## Verification
1. `cargo build` clean
2. `cargo test` — pattern detection accuracy, count estimation, scoping suggestions
3. Real CLI: `find . -name "*.rs" | wc -l` → shows count, suggests `-maxdepth` if >20
4. Real CLI: `find . -maxdepth 1 -name "*.sh"` → no warning (already scoped)

## Acceptance Criteria
- [ ] Unscoped glob/find patterns detected and flagged
- [ ] Match count estimated before destructive execution
- [ ] >20 matches triggers warning + confirmation
- [ ] Scoped alternatives suggested to model
- [ ] Already-scoped commands pass without warning
