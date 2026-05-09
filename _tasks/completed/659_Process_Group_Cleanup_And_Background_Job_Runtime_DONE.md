# Task 659: Process Group Cleanup And Background Job Runtime

**Status:** pending
**Priority:** HIGH
**Type:** Reliability / Tooling
**Scope:** `src/persistent_shell.rs`, `src/background_task.rs`, `src/streaming_tool_executor.rs`, `src/tool_calling.rs`, `elma-tools/src/tools/job_*`
**Source:** old background shell task 024, deferred job concerns, `_knowledge_base` Codex/Roo process cleanup

## Summary

Ensure shell commands, code interpreters, and background jobs clean up process groups and descendants on timeout, cancellation, failure, and shutdown.

## Evidence And Gap

- `persistent_shell.rs` kills the child on idle timeout, but descendant cleanup needs verification.
- Background job tools expose start/status/output/stop, but enterprise-grade CLI behavior requires process-tree cleanup and replayable output.
- `_knowledge_base` includes process group cleanup tests from Codex/Roo.

## Implementation Plan

1. Add cross-platform process group/session helpers for Unix and Windows.
2. Ensure timeout/cancel/stop kills descendants, not only direct child.
3. Store job output with sequence numbers, byte offsets, truncation metadata, and exit status.
4. Surface cancellation and timeout as tool terminal states and transcript rows.

## Acceptance Criteria

- [ ] `sh -c "sleep 300 & sleep 300"` descendants exit after cancellation/timeout.
- [ ] Background output can be replayed without loading unbounded memory.
- [ ] Shutdown cleans active jobs or records abandoned state explicitly.
- [ ] Windows behavior is handled or clearly feature-gated.

## Verification Plan

Run process cleanup tests, job tool tests, and manual cancellation/timeout prompts.

