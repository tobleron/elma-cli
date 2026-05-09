# Task 780: Fix Architecture Doc Line Count Drift

## Type
Documentation / Truthfulness

## Severity
Medium

## Scope
Documentation

## Problem

`docs/ARCHITECTURE.md` Module Map contains line counts that are **wildly incorrect** for several modules. This is a truthfulness violation — documentation claims about the codebase must match reality.

**Confirmed mismatches:**

| Module | Claimed | Actual | Error Factor |
|--------|---------|--------|-------------|
| `env_utils.rs` | 5,851 | 204 | 29x |
| `approach_engine.rs` | 16K+ | 487 | 33x |
| `background_task.rs` | 17K+ | 541 | 31x |
| `document_adapter.rs` | 63K+ | 1,892 | 33x |
| `intel_units/mod.rs` | 19K+ | 612 | 31x |
| `shell_preflight.rs` | 964 | 1,003 | ~1x (within tolerance) |

The "K+" suffix numbers appear to be byte counts accidentally reported as line counts, or copy-paste errors from a previous audit.

## Root Cause

`_scripts/update_docs_line_counts.sh` was not run after Task 777 ("Audit and Sync ARCHITECTURE.md Line Counts") completed. The task may have been archived prematurely, or the script produced byte-based figures that were mistaken for lines.

## Proposed Solution

Phase 1: Run `wc -l` on every file listed in the Module Map and update ARCHITECTURE.md with correct values.

Phase 2: Update `_scripts/update_docs_line_counts.sh` to explicitly use `wc -l` (line count), not `wc -c` (byte count) or `stat -f %z` (file size).

Phase 3: Add a CI check that compares ARCHITECTURE.md line counts against actual `wc -l` output and fails if any module differs by more than 20%.

## Acceptance Criteria
- [ ] Every line count in `docs/ARCHITECTURE.md` Module Map matches `wc -l` output within ±5%
- [ ] `_scripts/update_docs_line_counts.sh` produces correct line counts
- [ ] No module size is listed in bytes while labeled as "lines"

## Verification Plan
- Unit test: Run update script, diff output against ARCHITECTURE.md
- Integration test: Manual spot-check of 10 modules
- Regression test: Script-based CI comparison

## Dependencies
None.

## Notes
- **Architectural Rule violated:** Rule 5 (Grounded Answers Only) — documentation claims about system state must be evidence-backed
- Task 777 is completed but its output is stale or incorrect — this is a follow-up correction
