# Task 119: Dry-Run Mode for Destructive Commands

## Priority
**P1 — Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection)

## Problem

The model executed `mv ...` directly without verifying what would happen. A dry-run mode lets it (or the user) preview the effect before committing.

## Scope

### 1. Automatic Dry-Run for Destructive Commands
- Before executing `mv`, `rm`, or bulk operations: run equivalent dry-run
  - `mv` → `echo` source → destination mapping
  - `rm` → list files that would be deleted
  - `find ... | xargs mv` → preview the full file list
- Show preview to model as tool result: "These 641 files would be moved to stress_testing/: ..."
- Model can then confirm, adjust, or cancel

### 2. Explicit Dry-Run Tool
- New `shell` variant: `shell_dry_run` — executes command with echo/preview instead
- Model can explicitly request dry-run: `shell_dry` tool or `--dry-run` flag in command

### 3. Integration Points
- `src/tool_calling.rs` — add dry-run execution path
- `src/shell_preflight.rs` — generate dry-run preview commands

## Design Principles
- **Model-first:** Dry-run results fed back as tool results, model decides next step
- **Small-model-friendly:** Concrete file lists, not abstract descriptions
- **Offline-first:** No network needed

## Verification
1. `cargo build` clean
2. Real CLI: `mv *.sh dest/` → shows preview of files that would move
3. Real CLI: `rm test_*` → lists files that would be deleted

## Acceptance Criteria
- [ ] Destructive commands show dry-run preview before execution
- [ ] Preview includes exact file paths (not summaries)
- [ ] Model can adjust command based on preview
- [ ] Already-safe commands skip dry-run (no overhead)
