# Task 736: Nudge Model Toward Scoped Directory Discovery After Root Listing

## Type

Model Robustness / Discovery

## Severity

High

## Scope

System-wide

## Session Evidence

Session `s_1778156134_139572000`, Turn 1:

User: "read all docs and tell me what you think about elma-cli project."

Model's first discovery tool call: `ls .` (workspace root) — returned **3543 items**. The listing was dominated by `project_tmp/backup_*` copies (each containing full source tree duplicates), `sessions/` archives, `.kilo/node_modules/`, and other noise. The `docs/` directory was invisible in the 1002-line output.

Subsequent calls: `glob **/*.md` returned 100 `.kilo/node_modules/` README files. `search rg pattern=\.md$` returned source code references. At no point did the model discover `docs/` — it got lost in workspace noise.

Coverage gate trace:
```
trace: coverage continuation 1/3 (reads=0, listings=1, threshold=4)
trace: coverage continuation 2/3 (reads=0, listings=1, threshold=4)
trace: coverage continuation 3/3 (reads=0, listings=1, threshold=4)
```

Coverage continuations were exhausted before the model navigated to `docs/` because the root `ls` overwhelmed it with 3543 items, mostly backup archives.

## Problem

When the model asks to list the root directory, the system returns an unfiltered dump including `project_tmp/backup_*`, `sessions/`, `.kilo/node_modules/`, etc. — directories that are excluded by `workspace_policy::DEFAULT_EXCLUDED_PATHS` but the `ls` tool doesn't apply exclusions. The model sees thousands of irrelevant entries and can't find the actual project structure.

Additionally, the model's first tool call strategy is poor: instead of `ls docs/` or `workspace_info`, it uses broad discovery (`ls .`, `glob **/*.md`). The system should nudge the model toward scoped, efficient discovery.

## Root Cause Hypothesis

Confirmed: `ls` tool execution does not apply `workspace_policy::DEFAULT_EXCLUDED_PATHS`. The `ls` tool passes the raw path to the OS `ls` command without filtering. Files in `project_tmp/`, `sessions/`, etc. flood the output.

Additionally, no hint is provided to the model after a broad, noisy listing to suggest narrowing scope. The model doesn't know which directories are important.

## Proposed Solution

### Part A — Apply default exclusions to `ls` results

Modify `exec_ls` in `src/tool_calling.rs` to:
1. After running `ls`, filter out lines that match any `DEFAULT_EXCLUDED_PATHS` components
2. Replace excluded entries with a compact note: `(excluded: project_tmp/ — 842 items)`
3. Keep the total count accurate but reduce noise

### Part B — Inject scoping hint after noisy root listing

After `ls` or `glob` returns > 100 entries, inject a hint:
```
"Found X items. The listing includes generated/vendor directories. Try a scoped search: 'ls docs/' to discover documentation, 'ls src/' for source code."
```

### Part C (future) — Detect user's stated path and suggest it

If user says "read all docs" and the model hasn't touched `docs/`, inject: `"Try 'ls docs/' to discover the documentation directory."`

## Acceptance Criteria

- [ ] `ls .` at root no longer shows `project_tmp/`, `sessions/`, `.kilo/`, `_knowledge_base/` entries inline
- [ ] Excluded directories show as compact summary entries: `(excluded: project_tmp/ — 842 items)`
- [ ] `ls docs/` still shows all docs/ entries (not excluded)
- [ ] After `ls` returns > 100 entries, a scoping hint is injected
- [ ] `cargo test` passes existing tests

## Verification Plan

- Unit test: `ls .` mock → verify default exclusions applied
- Manual replay: session `s_1778156134_139572000` Turn 1 → model navigates to `docs/` after ls hint
- Verify coverage gate only fires 1-2 times (not 3) because model discovers docs/ faster

## Dependencies

- Task 735 (coverage gate B1/B2 fixes needed for accurate counting)
- Task 726 (exclusion policy is defined but ls doesn't apply it)

## Notes

This task is about making discovery work efficiently. The current situation is: the coverage gate correctly demands more evidence (good), but the model's discovery tools return noise instead of signal (bad). The fix bridges this gap by making discovery tools respect the exclusion policy that the coverage system assumes exists.
