# Task 373: Skills And Formulas DSL Integration Suite

**Status:** pending
**Priority:** medium-high
**Suite:** DSL Protocol And Skills Certification
**Depends on:** Task 364 (DSL protocol coverage matrix), Task 365 (DSL protocol self-test harness), Task 380 (intel units DSL migration)

## Objective

Certify that built-in skills and formulas select coherent DSL action strategies and preserve semantic continuity from user intent to final answer.

## Required Deliverables

- prompt scenarios under `tests/dsl/prompts/skills_formulas.md`
- formula/skill expected-action matrix
- regression tests for skill selection and formula rendering

## Built-In Elma CLI Prompt Pack

```text
Use the appropriate repo-inspection behavior to find where Elma stores session transcripts. Answer with exact files and evidence.
```

```text
Use the document-reading behavior to inspect a markdown fixture in the sandbox and extract only the section titled TOOL_SKILL_ALPHA.
```

```text
Make a safe exact edit in the sandbox task folder for a tiny documented improvement, then verify the changed file. Do not touch real project tasks.
```

```text
Given this multi-step request, choose a plan, inspect the relevant files, and then answer: how does the DSL action dispatcher make commands available? Use evidence from source files.
```

```text
Analyze whether this request needs tools, a plan, or a direct answer: "What is 2+2?" Then answer directly without unnecessary tools.
```

## Verification

Required commands:

```bash
cargo fmt --check
cargo test skills
cargo test formulas
cargo test orchestration_core
cargo test program_policy
cargo build
```

Prompt pass criteria:

- simple tasks avoid unnecessary tools
- evidence-heavy tasks use inspection actions
- edit/task-steward tasks verify mutations
- formulas do not solve a different user intent
- final answers cite evidence for evidence-required prompts

## Done Criteria

- Every built-in skill/formula has at least one prompt test.
- DSL action choice aligns with formula expectations.
- Semantic continuity failures are regression-tested.
