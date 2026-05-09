# Task 658: Parser Backed Shell Exec Policy And Permission Cache

**Status:** pending
**Priority:** CRITICAL
**Type:** Security / Reliability
**Scope:** `src/shell_preflight.rs`, `src/program_policy.rs`, `src/permission_gate.rs`, `src/tool_calling.rs`
**Source:** agent `_knowledge_base` audit; Codex `execpolicy` and shell-command parser references; old safe shell tasks

## Summary

Replace shell safety decisions based on raw substring matching with parser-backed command analysis, explainable policy, and canonical approval keys.

## Evidence And Gap

- `shell_preflight.rs` and `program_policy.rs` inspect command strings extensively.
- Permission cache currently supports exact/prefix matching and needs stronger command canonicalization.
- `_knowledge_base/_source_code_agents/codex-cli/codex-rs/execpolicy/` and shell command parsing references show a better architecture.

## Implementation Plan

1. Parse shell commands into segments, control operators, redirections, env assignments, subshells, and command words.
2. Evaluate policy over parsed command effects instead of ad hoc text triggers.
3. Canonicalize approval keys by command segments, cwd, network, sandbox/profile, and TTY mode.
4. Preserve transcript-visible permission reasons.
5. Add platform-specific parsing notes for zsh/bash/powershell.

## Acceptance Criteria

- [ ] Pipelines, redirections, `bash -lc`, nested shells, and env assignments are policy-evaluated structurally.
- [ ] Approval cache cannot be bypassed by prefix ambiguity.
- [ ] Safe false positives and unsafe false negatives have tests.
- [ ] Policy explanations are concise and visible.

## Verification Plan

Run shell policy tests covering destructive, network, read-only, nested, and platform-specific commands.

