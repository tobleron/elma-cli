# Task 492: Bounded Subagent Delegation Framework

**Status:** pending
**Source patterns:** OpenHands agent delegation, Claude subtask patterns, Crush coordinator/session agents
**Depends on:** Task 451 (recipe workflow system), Task 478 (headless event API)

## Summary

Add bounded local subagent delegation for independent sidecar tasks such as repo exploration, verification, summarization, and implementation slices. Subagents must have explicit budgets, scopes, and event visibility.

## Why

Elma's philosophy favors decomposition for small models. Current formulas decompose stages, but they do not run isolated subagents with separate context, budgets, and outputs. Reference agents use delegation to keep the main context cleaner and parallelize independent work.

## Implementation Plan

1. Define a subagent contract: input, workspace scope, tool permissions, budget, expected output, and merge behavior.
2. Start with sequential subagents before enabling parallel execution.
3. Persist subagent events under the parent session.
4. Require explicit scope for write-capable subagents.
5. Add finalizer integration that cites subagent evidence without treating it as unquestioned truth.

## Success Criteria

- [ ] A read-only explorer subagent can answer a bounded codebase question.
- [ ] Parent session records subagent start, tool use, output, and stop reason.
- [ ] Budgets prevent runaway subagent loops.
- [ ] Write-capable delegation is blocked until file ownership and conflict rules are implemented.
- [ ] Tests cover timeout and failed subagent handling.

## Anti-Patterns To Avoid

- Do not use subagents as hidden unbounded autonomy.
- Do not let subagent conclusions bypass evidence grounding.
- Do not parallelize write tasks before ownership rules exist.
