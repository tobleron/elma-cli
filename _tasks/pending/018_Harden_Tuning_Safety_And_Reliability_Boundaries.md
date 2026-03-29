# Task 018: Harden Tuning Safety And Reliability Boundaries

## Objective
Deeply analyze Elma's current tuning pipeline and implement the safest changes that increase reliability without changing Elma's identity, safety model, or prompt semantics.

This task exists to keep tuning accountable:
- tuning must improve reliability
- tuning must stay reproducible
- tuning must not rewrite Elma through prompt drift
- tuning must prefer runtime defaults when they are equally good or more stable

## Why This Matters
Recent work improved Elma's runtime substantially:
- model behavior probing
- quick startup tuning
- standalone quick corpus
- structured JSON output repair
- workflow planning, OODA, verification, critics, artifacts, snapshots

The tuning pipeline now needs to catch up with those changes cleanly.

If tuning is too weak:
- new models remain unreliable
- startup tuning gives misleading confidence
- full tune wastes time and evaluates bad candidates too deeply

If tuning is too powerful:
- it can silently mutate behavior
- profile changes become hard to explain
- reliability becomes unaccountable

So the correct direction is:
- deeper analysis
- safer boundaries
- stronger measurement
- stricter activation policy

## Required Analysis
Perform a full audit of the current tuning system and document the answers in a short analysis artifact under `docs/` or `_tasks/analysis/`:

1. What tuning currently changes
- enumerate every file and every field that tuning can modify or activate
- distinguish:
  - tuned numeric parameters
  - reasoning mode changes
  - profile activation/switching
  - immutable items

2. What quick tuning actually guarantees
- identify which runtime layers quick tuning evaluates
- identify which layers it does not meaningfully validate
- document the remaining reliability risk after a quick tune

3. What full tuning currently validates
- routing
- workflow/program quality
- execution quality
- response quality
- efficiency
- variance/stability if any

4. Where the current full tune is still weak
- missing runtime-safe scenarios
- platform portability gaps
- stateful scenario leakage
- insufficient variance penalties
- poor early-abort logic
- missing protected baselines

5. What tuning must never be allowed to change
- system prompts by default
- deterministic safety/policy behavior
- validation schemas
- slash command semantics
- corpus expectations during a tune run

## Safe Changes To Implement

### A. Protected Baseline Anchors
Full tuning must always evaluate and preserve three baseline anchors:
- active live Elma profile set
- llama.cpp runtime-default baseline
- immutable shipped baseline

Requirements:
- save separate score outputs for all three
- include them in run artifacts and final summary
- prefer runtime defaults when tuned candidates are only marginally better or less stable

### B. Explicit Allowed-Tuning Surface
Create a clear allowlist for tunable fields per intel unit.

Allowed examples:
- `temperature`
- `top_p`
- `repeat_penalty`
- `max_tokens`
- bounded `reasoning_format` changes only when compatible with `model_behavior.toml`

Disallowed examples:
- `system_prompt`
- route labels
- schemas
- slash command behaviors
- safety rules

Implementation requirements:
- enforce the allowlist in code, not just docs
- fail fast if a tuning stage attempts to mutate disallowed fields

### C. Runtime Default Baseline Extraction
Use actual endpoint/runtime defaults when available and build a protected candidate from them.

Requirements:
- read the runtime-default generation settings from the server if exposed
- map them into Elma profiles conservatively
- write them as a candidate profile set under the tune run
- compare them directly against active and shipped baselines

### D. Stability And Variance Penalty
Full tuning should not only optimize mean score.

Requirements:
- rerun a small subset of critical scenarios multiple times
- measure variance/instability
- penalize candidates that are less stable even if their mean score is slightly higher
- expose stability metrics in tune artifacts

### E. Safer Activation Policy
Do not activate a tuned winner just because it has the highest raw score.

Requirements:
- require meaningful improvement over the protected runtime-default baseline
- if improvement is marginal, prefer the more stable baseline
- expose the activation reason in the run summary
- record whether activation happened because of:
  - higher score
  - higher stability
  - baseline preference

### F. Better Progress And Abort Conditions
Full tuning should remain transparent and bounded.

Requirements:
- show stage progress clearly
- show which corpus is being used
- abort early when protected baselines and early routing stages already prove the model is not viable
- do not continue deep workflow/response tuning on hopeless candidates

### G. Corpus Safety And Coverage Review
Strengthen the corpus without introducing stateful fragility.

Requirements:
- keep quick corpus exactly standalone-safe
- review full corpus for runtime safety flags
- add missing runtime-safe scenarios for:
  - portability failures
  - command-not-found behavior
  - artifact-mode presentation
  - exact-output vs summarized-output behavior
  - model reasoning leakage handling

## Acceptance Criteria
- tuning can explain exactly what fields it changed
- tuning cannot mutate system prompts by default
- tune runs always include:
  - active baseline
  - runtime-default baseline
  - shipped baseline
- quick tuning remains fast and standalone-safe
- full tuning is more reliable and more accountable
- activation decisions are explainable from saved artifacts
- weak models abort earlier instead of wasting deep-stage time

## Verification
Run after implementation:
- `cargo build`
- `cargo test`
- a quick tune on a known weak model
- a quick tune on a known stronger model
- a full tune dry run or bounded full tune validation on one model

Manually verify:
- baseline artifacts exist and are distinct
- activation manifest explains why the chosen profile won
- no prompt text changed during tuning

## Notes
This task is about tuning discipline, not prompt rewriting.
The goal is not to make tuning more powerful in an unconstrained way.
The goal is to make tuning:
- safer
- more reliable
- more transparent
- more stable across models
