# Task 789: Declarative Execution Policy Engine

## Objective
Replace keyword-based shell preflight checks with a robust, declarative Exec Policy engine (P1 Priority).

## Background
Elma currently uses brittle `cmd.contains()` keyword matching in `shell_preflight.rs` to determine if a command is safe or requires approval. This violates AGENTS.md Rule 1. Codex-RS uses a declarative policy engine (`exec_policy.rs`) that canonicalizes commands, applies prefix-based rules, and cascades decisions (Allow/Prompt/Forbidden). This task aligns Elma with that proven pattern and resolves Task 783.

## Requirements
1. Remove all hardcoded keyword matching (`.contains("sudo")`, `.contains("rm")`, etc.) from shell preflight logic.
2. Implement a `Policy` system that evaluates a command against a list of prefix rules (e.g., `["git", "status"] -> Allow`, `["rm"] -> Prompt`).
3. Implement command canonicalization using the `shlex` crate. Compound commands (e.g., `ls && rm -rf /`) must be split, and the policy must be evaluated against each sub-command independently.
4. Ensure the policy evaluation returns a strict `Decision::Allow`, `Decision::Prompt`, or `Decision::Forbidden`.

## Success Criteria
- [ ] Keyword matching is completely removed from shell preflight.
- [ ] Compound commands like `echo "hello" && rm -rf /` trigger the correct policy decision (e.g., `Prompt` or `Forbidden` due to the `rm`).
- [ ] Safe commands like `ls -la` are allowed without prompting based on declarative rules.
- [ ] Tests verify command splitting and policy evaluation.
