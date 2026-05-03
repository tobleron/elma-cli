# Task 454: Search Tool Rust-First Execution Rewrite

**Status:** pending
**Priority:** HIGH
**Source:** 2026-05-02 full codebase audit
**Related:** completed Task 324, completed Task 340, pending Task 457

## Summary

Rewrite the model-facing `search` tool execution path to avoid shell-string construction and align with rust-first tool policy.

## Evidence From Audit

- `src/tool_calling.rs::exec_search` constructs command strings like `rg -i --line-number --no-heading --color=never '{pattern}' '{path}'`.
- A pattern or path containing a single quote can break shell quoting.
- `src/execution_steps_search.rs` already uses `std::process::Command` with argv arguments.
- The `search` schema advertises `literal_text` and `include`, but the executor ignores those fields.

## User Decision Gate

Ask the user whether search should:

- Use `rg` through `std::process::Command` as the first rewrite.
- Move directly to a Rust-native `ignore`/regex search implementation.
- Support both, preferring Rust-native when feature-complete.

## Implementation Plan

1. Implement argv-safe `rg` search or Rust-native search per user choice.
2. Honor `literal_text` and `include` schema fields.
3. Apply workspace/path policy from Task 442.
4. Preserve no-match behavior as successful empty evidence.
5. Add injection, include-glob, literal, regex, and path-scope tests.

## Success Criteria

- [ ] Search no longer builds an executable shell string from user-controlled fields.
- [ ] Schema fields match executor behavior.
- [ ] No-match results are clear and non-failing.
- [ ] Tests cover quote characters and regex/literal modes.

## Anti-Patterns To Avoid

- Do not escape shell strings when argv APIs solve the problem.
- Do not drop regex search support.
- Do not search outside approved workspace policy.
