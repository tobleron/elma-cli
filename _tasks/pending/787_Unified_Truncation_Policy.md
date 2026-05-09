# Task 787: Unified Truncation Policy

## Objective
Consolidate disparate tool output truncation logic into a single, cohesive `TruncationPolicy` hierarchy (P0 Priority).

## Background
Elma currently has ad-hoc `truncate` calls scattered across multiple files (`execution_steps_shell.rs`, `execution_steps_read.rs`, `execution_steps_search.rs`, etc.), each with different strategies and thresholds. Codex-RS uses an explicit `TruncationPolicy` enum (`Tokens(n)`, `Lines(n)`) applied uniformly, which makes behavior predictable and easily configurable.

## Requirements
1. Create a new module (e.g., `output_truncation.rs`) defining a `TruncationPolicy` enum.
   - Example variants: `Tokens(usize)`, `Lines(usize)`, `HeadAndTail(usize, usize)`.
2. Implement a `truncate_text` function that takes a string and a `TruncationPolicy`.
3. Audit all existing tool execution modules (`execution_steps_*.rs`).
4. Replace existing ad-hoc truncation logic with the unified `TruncationPolicy` system.
5. Define standard default policies for different tools (e.g., shell commands might prefer `HeadAndTail`, while search might prefer `Lines`).

## Success Criteria
- [ ] `TruncationPolicy` enum is defined and implemented.
- [ ] At least 3 tool modules (`shell`, `read`, `search`) use the unified policy.
- [ ] No ad-hoc string slicing for truncation remains in the tool execution modules.
- [ ] Tests verify that different policies truncate correctly.
