# Task 355: Keyword Heuristic Decomposition Audit

**Status:** pending
**Source patterns:** Elma AGENTS.md rules, OpenHands typed policy events, Roo model-aware budgeting
**Depends on:** Task 377 (DSL parser/error model), Task 339 (action/tool metadata policy), Task 344 (recipe workflow system)

## Summary

Audit and replace non-safety semantic keyword heuristics with model confidence, typed metadata, evidence state, entropy, or focused intel units. Keep parser lexing, DSL grammar checks, and defensive command/path safety checks separate and explicitly justified.

## Why

Elma's philosophy forbids turning routing and behavior selection into keyword matching. The codebase still contains several hardcoded phrase or command-pattern heuristics in areas like intent-only response detection, shell-risk forecasting, and tool availability. Some are valid parser or safety checks; others should be decomposed.

The DSL migration will add strict token and marker parsing. That is allowed when it validates syntax or safety. It must not become semantic routing by hardcoded user phrases.

## Implementation Plan

1. Inventory all `contains`, prefix, suffix, and regex heuristic sites that affect routing, compaction, finalization, tool choice, DSL parsing, and command/path safety.
2. Classify each as grammar/parser, safety-critical, UI-level, compatibility, or semantic decision.
3. Replace semantic decision heuristics with typed metadata or focused model/intel decisions.
4. Document justified DSL parser and safety heuristics and add tests around them.
5. Add a regression check for new semantic keyword matching in sensitive modules.

## Success Criteria

- [ ] Inventory identifies all high-impact heuristic sites.
- [ ] Semantic routing/finalization heuristics are reduced or replaced.
- [ ] DSL parser and safety heuristics remain conservative and tested.
- [ ] New checks prevent reintroducing keyword routing in core modules.
- [ ] `src/prompt_core.rs` remains unchanged unless explicitly approved.

## Anti-Patterns To Avoid

- Do not remove safety checks just because they use string patterns.
- Do not weaken strict DSL parser checks to satisfy the keyword-matching rule.
- Do not replace keyword lists with bigger prompt examples.
- Do not hide heuristic decisions in trace-only logs.
