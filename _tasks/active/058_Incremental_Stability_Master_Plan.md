# Task 058: Incremental Stability Master Plan

## Priority
**P0 - MASTER STABILIZATION TASK**
**Created:** 2026-04-03
**Status:** In Progress (Stability Phase)

## Progress Summary
- **Grounded Workspace Discovery**: Successfully implemented `ls -R` + `file` probe in `src/app_chat_core.rs` to ground orchestrator context, successfully resolving language-based (Go vs Rust) hallucinations.
- **Hallucination Mitigation**: Improved tool registry and prompt grounding to ensure the model uses accurate language/tool context.
- **Stress Harness Refinement**: 
    - Sloppy human tests (`H*`) are stable.
    - CLI stress tests (`S000A`-`S000I`) pass after resolving complex JSON repair issues and shell pipe syntax management.
- **JSON Stability**: Repaired `chat_json_with_repair_text` and associated parsers to ensure that retries operate on repaired JSON output.

## Remaining Objectives
1. **Validate S001-S008**: Complete full verification of the remaining stress scenarios.
2. **Context Efficiency Audit**: Further refine prompt size limits to ensure stable retries under extreme load.
3. **Task Sanitization**: Create a new P1 task for renaming `_stress_testing/_opencode_for_testing` to `_stress_testing/_sandbox` to eliminate legacy references.
4. **Final Architecture Review**: Conduct a final audit of the `src/orchestration_planning.rs` and `src/routing_infer.rs` changes to confirm they align with the long-term de-bloating plan.
