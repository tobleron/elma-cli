# Task 116: Destructive Command Detection & Preflight Validation

## Priority
**P0 — Critical Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** None (builds on tool-calling from Phase 2)

## Problem

The model generated: `find . -type f -name "*.sh" | while read f; do mv "$f" "stress_testing/"; done`

This would have moved 641+ files across the entire project tree into a non-existent directory. Only the missing target directory prevented disaster. No preflight, no validation, no safety net.

## Scope

### 1. Destructive Command Detection
- Classify shell commands by risk level before execution:
  - **Safe** (green): `ls`, `cat`, `head`, `wc`, `find` (read-only), `rg`, `grep`, `pwd`, `whoami`
  - **Caution** (yellow): `cp`, `mkdir`, `touch`, `chmod`, `mv` (with verified destination)
  - **Dangerous** (red): `rm`, `rmdir`, `mv` (unverified), `git reset --hard`, `git clean -f`, `dd`, `chmod -R 777`, `find ... -delete`, `| while read ... do mv/rm`

### 2. Preflight Validation (Before Execution)
- For `mv`: verify source exists AND destination parent directory exists
- For `rm`: verify file exists AND warn if >5 files match
- For pipe-to-mv/rm patterns (`find ... | while read ... do mv`): flag as bulk operation, require explicit approval
- If preflight fails: return error message to model with specific guidance (e.g., "Destination directory 'stress_testing/' does not exist. Did you mean '_stress_testing/'?")

### 3. Integration Points
- `src/tool_calling.rs` — `exec_shell()` calls preflight before `run_shell_one_liner()`
- `src/shell_preflight.rs` (new) — detection + validation logic
- `src/ui_trace.rs` — show risk level in trace output

## Design Principles
- **Small-model-friendly:** Preflight errors are explicit, not heuristic-based. "Directory X doesn't exist" not "This seems wrong"
- **No keyword routing only:** Detection uses structural analysis (pipe patterns, loop constructs) not just word lists
- **Model learns:** Error messages feed back to model so it self-corrects next attempt
- **Never silently block:** Preflight returns guidance, doesn't just say "no"

## Verification
1. `cargo build` clean
2. `cargo test` — detection accuracy, preflight validation, error messages
3. Real CLI: `mv nonexistent_file somewhere/` → preflight error, not execution failure
4. Real CLI: `rm *.sh` (many files) → warning + count, requires confirmation
5. Real CLI: `find . | while read f; do rm "$f"; done` → flagged as bulk destructive, requires confirmation

## Acceptance Criteria
- [ ] Destructive commands detected before execution (not after failure)
- [ ] Preflight validates source/destination for mv/cp/rm
- [ ] Bulk operations (>20 files) flagged with explicit file count
- [ ] Model receives specific error guidance (not generic "command failed")
- [ ] Risk level shown in trace output (safe/caution/dangerous)
- [ ] No regression on safe commands (they execute without delay)
