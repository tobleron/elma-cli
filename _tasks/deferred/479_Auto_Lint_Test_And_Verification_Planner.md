# Task 479: Auto Lint/Test And Verification Planner

**Status:** pending
**Source patterns:** Aider lint/test integration, Qwen-code test runners, Goose workflow recipes
**Depends on:** Task 451 (recipe workflow system), Task 478 (headless event API)
**Input from:** Task 437 — Clippy is a mandatory verification gate for Rust changes.

## Summary

Add a verification planner that selects and runs relevant build, lint, format, and test commands after code changes, using project metadata and prior evidence rather than hardcoded request keywords.

## Why

Elma currently depends on ad hoc command choices or task instructions for verification. Mature coding agents integrate targeted verification into the edit lifecycle, making the default behavior safer after modifications.

## Implementation Plan

1. Detect project verification commands from manifest files and config.
   - Clippy (`cargo clippy --all-targets`) is a mandatory gate for all Rust changes.
2. Record successful/failed verification commands in project memory with evidence.
3. Select a minimal verification plan based on changed files and tool metadata.
4. Ask permission for commands that are not clearly safe or already approved.
5. Surface verification plan and results as transcript events.

## Success Criteria

- [ ] Rust changes propose or run `cargo build`/targeted tests when appropriate.
- [ ] Verification selection is based on manifests and changed files, not prompt keywords.
- [ ] Failures include useful next-step evidence.
- [ ] Commands respect shell preflight and permission policy.
- [ ] Tests cover manifest detection and plan selection.

## Anti-Patterns To Avoid

- Do not run expensive full suites blindly after every small edit.
- Do not invent test commands without evidence.
- Do not bypass user approval for risky commands.
