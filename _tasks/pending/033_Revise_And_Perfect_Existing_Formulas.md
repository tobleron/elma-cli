# Task 001: Revise And Perfect Existing Formulas

## Objective
Improve Elma's existing built-in formulas through disciplined trial and error so they become more reliable, concise, and grounded without expanding the deterministic core unnecessarily.

## Context
Elma already uses formula-like reasoning patterns such as `reply_only`, `capability_reply`, `inspect_reply`, `inspect_summarize_reply`, `inspect_decide_reply`, `execute_reply`, `plan_reply`, `masterplan_reply`, `cleanup_safety_review`, `code_search_and_quote`, and `config_compare`.

Several current issues indicate that the formulas need iterative refinement:
- over-inspection on simple conversational turns
- weak presentation on "show/list/print" requests
- occasional path-handling mistakes
- noisy or overly broad evidence gathering
- imperfect decision quality on workspace cleanup and comparison tasks

This remains an umbrella task, but its refinements should increasingly rely on the stronger verification and workflow-planning foundations introduced in later pending tasks.

## Work Items
- [ ] Inventory the currently shipped formulas and the implicit formula patterns used by the orchestrator.
- [ ] For each formula, define:
  - intended use case
  - expected evidence pattern
  - expected reply pattern
  - common failure modes
- [ ] Build a small formula evaluation matrix using representative prompts for each formula.
- [ ] Run trial-and-error refinements one formula at a time, preferring prompt and orchestration corrections before adding new deterministic code.
- [ ] Record which refinements improved accuracy and which degraded it.
- [ ] Update the active formula-related prompts and/or orchestration logic only where evidence shows a real gain.
- [ ] Revalidate revised formulas against calibration scenarios rather than relying only on ad hoc chat impressions.

## Acceptance Criteria
- Each existing formula has a documented purpose and known failure pattern.
- Each existing formula has at least one representative validation prompt.
- Weak formulas are improved individually rather than changed all at once.
- Changes are grounded in observed behavior, not guesswork.
- Elma performs better on formula-driven tasks without introducing keyword-based hardcoding.

## Verification
- `cargo build`
- `cargo test`
- run targeted live prompts for each revised formula
- verify at least one before/after example per revised formula
