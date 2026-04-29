# Task 340: Native Search Execution And Query Safety

**Status:** pending
**Source patterns:** OpenHands typed file actions, Roo-Code codebase search, Codex structured tool execution
**Depends on:** Task 377 (DSL parser/error model), Task 379 (DSL path/command/edit safety)
**Blocks:** Task 378 (`S`/`Y` action execution)

## Summary

Replace shell-string search execution with structured command invocation or native search APIs for the DSL `S` and `Y` actions. Search arguments must never be interpolated into a shell command.

## Why

`exec_search` currently builds an `rg` command string with quoted pattern and path. This is fragile for patterns containing quotes and creates unnecessary shell parsing risk. Search should be a structured read-only action, not a shell command assembled from user/model strings.

The DSL migration makes this non-optional: model output for `S q="..." path="..."` and `Y q="..." path="..."` must become typed Rust search requests after parsing and path validation.

## Implementation Plan

1. Execute `rg` with `std::process::Command` arguments, or use an internal search implementation for supported modes.
2. Preserve ignore behavior, max result limits, and timeout controls.
3. Validate paths relative to the workspace and reject dangerous traversal where policy requires it.
4. Return structured results with file, line, column, match preview, and truncation metadata.
5. Record search observations in the evidence ledger and action-observation event log.
6. Wire the implementation behind `AgentAction::SearchText` and `AgentAction::SearchSymbol`.
7. Keep shell fallback unavailable to model actions; `X` is for verification commands only.

## Success Criteria

- [ ] Patterns containing single quotes, double quotes, spaces, and regex metacharacters work correctly.
- [ ] Search cannot run arbitrary shell fragments.
- [ ] Timeout and output size limits are preserved.
- [ ] Tests cover literal search, regex search, path scoping, no matches, and huge output truncation.
- [ ] DSL `S` and `Y` observations are compact and recoverable.

## Anti-Patterns To Avoid

- Do not escape strings by hand and keep using a shell.
- Do not make search behavior depend on hardcoded request keywords.
- Do not silently broaden path scope on failure.
