# Task 074: Model Capability Profiles For Local LLMs

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Priority
**P2 - EFFICIENCY & OBSERVABILITY (Tier B)**
**Depends on:** Tier A stability (tasks 065-069)

## Objective
Give Elma explicit per-model capability profiles so local small models are not treated like generic providers with identical reasoning, context, formatting, and JSON behavior.

## Why This Exists
Elma is designed for local small models. To feel premium, the system needs to adapt to the strengths and weaknesses of specific local models rather than assuming one-size-fits-all orchestration.

## Scope
- Define capability metadata for local models:
  - JSON reliability
  - context tolerance
  - answer verbosity tolerance
  - planning depth tolerance
  - formatting reliability
  - retry aggressiveness limits
  - evidence-compaction trust level
  - selector autonomy reliability
  - command-repair reliability
- Use those profiles to tune runtime behavior and defaults.
- Keep the system prompt constants stable while allowing capability-aware operational tuning.

## Deliverables
- A model capability profile schema.
- Runtime consumption of those capability profiles.
- Docs for adding new model capability definitions.

## Acceptance Criteria
- Runtime decisions can adapt to model capability without forking core prompts.
- Small local models receive safer bounded defaults.
- Capability profiles are inspectable and testable.
- Weak models can be configured to use stricter grounding rules for compaction/selection/repair roles when upstream evidence is thin or failed.

## Additional Session Evidence
- Session `s_1775235404_589084000` showed a classic small-model overreach pattern:
  - command repair stayed brittle
  - evidence compaction hallucinated successful rename details from failed raw evidence
  - selector produced a plausible identifier after empty evidence
- These are strong candidates for capability-profile-driven safeguards rather than prompt rewrites alone.
