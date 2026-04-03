# Task 061: Role-Based Temperature And Retry Strategy Calibration

## Priority
**P0 - RELIABILITY HARDENING**

## Objective
Turn temperature usage, retry escalation, and strategy switching into a calibrated system that is explicitly tuned by role instead of being an accidental mixture of defaults, retries, and legacy prompt variants.

## Why This Exists
Elma already has:
- per-profile temperatures
- retry temperature escalation
- strategy chains
- old strategy-specific prompt files

But they are not yet governed as one coherent reliability feature. Some roles are too hot for small local models, some escalation paths still regenerate similar bad programs, and some strategist prompts exist without being integrated as first-class controlled behavior.

## Problems To Solve
- Classifier, planner, reviewer, presenter, and repair roles do not have a clearly enforced temperature policy by job type.
- Retry escalation currently adjusts orchestrator temperature, but it is not tied to measured failure types.
- Strategy-chain behavior exists, but its prompts/configs are only partially integrated and not audited for live effectiveness.
- Local small models need narrower cognitive burden and lower stochasticity for JSON-critical or judgment-critical roles.
- Repeated stale retries need a more principled “change strategy, not just temperature” policy.
- Small-model utility roles can still hallucinate success from failed evidence:
  - `evidence_compactor` summarized a successful rename even when the raw shell step only showed `rg: unrecognized flag --no-color`
  - `selector` later returned a plausible identifier (`oldUtilityFunc`) that was not grounded in successful upstream evidence
- Retry logic still allows near-identical bad command shapes to recur after semantic rejection, especially in shell repair loops.

## Scope
- Audit every live profile temperature by role.
- Define temperature bands by role class:
  - hard-deterministic JSON classifiers
  - bounded judgment units
  - orchestration/program generation
  - presentation/final-answer roles
  - repair/refinement/meta-review roles
- Audit the current retry ladder in `src/orchestration_retry.rs` and `src/strategy.rs`.
- Decide which retry behaviors are truly live and which are dead or redundant.
- Integrate strategy-specific prompt variants only where they are actually useful.
- Prevent futile same-shape retries by combining:
  - stale-program detection
  - failure-type-aware strategy switching
  - bounded temperature envelopes
- Audit low-intelligence-sensitive roles specifically:
  - `command_repair`
  - `task_semantics_guard`
  - `evidence_compactor`
  - `selector`
  - `rename_suggester`
- Define when a role should be skipped, downgraded, or forced deterministic because upstream evidence is failed, empty, or semantically rejected.
- Keep the system suitable for small local models.

## Deliverables
- A documented temperature policy by role.
- Calibrated default temperatures in `config/defaults/` and/or startup normalization.
- A cleaner retry strategy ladder with explicit failure-based transitions.
- Trace output that shows why strategy and temperature changed.
- Regression tests for retry/strategy selection behavior.

## Acceptance Criteria
- JSON-critical intel units remain deterministic and stable under test.
- Retry attempts do not simply regenerate the same stale program shape without changing strategy.
- Strategy transitions are visible in traces and grounded in failure type.
- Failed shell evidence cannot be converted into a success-looking compact summary by downstream intel units.
- Selector-style units do not fabricate candidates when upstream evidence is empty or failed.
- `cargo build`, `cargo test`, and targeted CLI probes pass.
- The resulting policy is explicitly optimized for local small LLM behavior.

## Additional Session Evidence
- Session `s_1775235404_589084000` exposed a real low-model-calibration seam:
  - `command_repair` kept proposing `rg --no-color` on macOS even after rejection
  - `evidence_compactor` hallucinated a completed rename from a failed shell step
  - `selector` produced a plausible rename target despite missing grounded candidate evidence
  - this is exactly the kind of role-calibration and failure-aware stochasticity problem this task must close
