# 017_Align_Tuning_With_Current_Runtime_Architecture

## Objective
Bring Elma's tuning pipeline into full alignment with the current runtime architecture so that tuning remains safe, reproducible, model-specific, and accountable.

This task must explicitly treat **system prompts as frozen**. Prompt mutation, prompt bundle search, and automatic prompt rewriting are out of scope.

The goal is to tune only the parts that are safe to vary without making the model's behavior drift unpredictably across runs or across models.

## Why This Task Exists
Elma's runtime stack has grown materially:
- model behavior probing
- reasoning-format adaptation
- final-answer extraction
- workflow planning
- OODA / recovery loops
- command preflight and semantics guards
- execution sufficiency / outcome verification
- multi-reviewer recovery
- snapshot / rollback support
- formula memory
- artifact-mode execution

The tuning code was originally designed around an older, smaller stack.

Even after several upgrades, the tuning process still risks these gaps:
- tuning does not fully reflect the current live execution path
- unsafe parameters may be varied too broadly
- candidate comparisons may reward lucky runs instead of stable runs
- model behavior profiles may not be incorporated into parameter policy
- activation may favor score spikes over reliable, repeatable performance
- llama.cpp runtime defaults may be better than tuned candidates, but are not yet treated as a protected baseline

## Non-Goals
- Do not tune system prompts.
- Do not let tune invent prompt variants automatically.
- Do not turn tuning into open-ended search over all request fields.
- Do not optimize for a single lucky scenario at the expense of stability.
- Do not silently activate weak candidates with high variance.

## Safe Tuning Policy

### 1. What Is Safe To Tune
- `temperature`
- `top_p`
- `repeat_penalty`
- `max_tokens`
- bounded use of `reasoning_format`, but only when allowed by `model_behavior.toml`

### 2. What Must Stay Fixed
- system prompts
- route label schema
- JSON schema contracts
- safety policy
- verification criteria
- formula memory acceptance rules
- snapshot / rollback mechanics
- shell safety guards and output caps

### 3. Tuning Philosophy
- Deterministic or schema-sensitive units should stay near deterministic.
- Creative units may vary only within narrow, justified bands.
- The model's currently served llama.cpp runtime defaults must be treated as a first-class protected baseline.
- If two candidates have similar score, prefer:
  1. lower variance
  2. fewer parse failures
  3. lower latency
  4. lower token usage
- If a tuned candidate does not beat the runtime-default baseline by a meaningful and stable margin, do not activate it.
- Tuning must optimize for repeatability, not just peak score.

## Technical Tasks

- [ ] **Formalize Tuneable Unit Families**
  Define explicit tuning families and lock their allowed parameter ranges:
  - routing family: `speech_act`, `router`, `mode_router`
  - structured-output family: `json_outputter`, `final_answer_extractor`, `command_preflight`, `task_semantics_guard`, `execution_sufficiency`, `outcome_verifier`, `critic`, `logical_reviewer`, `efficiency_reviewer`, `risk_reviewer`
  - orchestration family: `workflow_planner`, `formula_selector`, `orchestrator`, `selector`, `command_repair`, `refinement`, `reflection`
  - response family: `_elma`, `summarizer`, `result_presenter`, `formatter`, `claim_checker`

- [ ] **Enforce Frozen Prompt Tuning Mode**
  Remove or permanently disable any remaining prompt-bundle or prompt-mutation search path from tuning.
  Tune must operate only on parameter sets and profile activation, never on prompt text.

- [ ] **Make Tuning Capability-Aware**
  Integrate `model_behavior.toml` into the tuning policy:
  - if a model is `auto_reasoning_separated` but `auto_truncated_before_final`, allow tuning of response-side `max_tokens` and final-answer-extractor settings within bounded limits
  - if JSON reliability is poor, keep structured-output units in conservative settings and penalize parse failures more heavily
  - do not let tune try incompatible reasoning-format strategies that contradict the capability profile

- [ ] **Capture llama.cpp Runtime Defaults As A Protected Baseline**
  Read and persist the endpoint's active generation defaults from llama.cpp before candidate search begins.
  This runtime-default profile must be evaluated as its own named baseline candidate, separate from:
  - the current active Elma profile set
  - the immutable shipped baseline profile set

- [ ] **Add Baseline Priority Rules**
  Update candidate selection so that runtime defaults are preferred when:
  - the tuned candidate wins only by a tiny margin
  - the tuned candidate has higher variance
  - the tuned candidate has higher parse-failure rate
  - the tuned candidate is materially slower without clear reliability gain

- [ ] **Bound Parameter Search By Unit Type**
  Implement safe search bands, for example:
  - routing / verification / JSON units:
    - `temperature`: `0.0` to `0.1`
    - `top_p`: `1.0` or near-1.0 only
    - `repeat_penalty`: close to `1.0`
    - `max_tokens`: smallest sufficient band
  - orchestration units:
    - low creativity only, enough to recover intelligently but not drift
  - response units:
    - modest band for helpfulness and fluency, but capped to prevent style drift

- [ ] **Tune Finalizer And Formatter As First-Class Units**
  Include `final_answer_extractor` and `formatter` in the response-quality stage so leaky thinking models are evaluated through the same runtime rescue path used in production.

- [ ] **Measure Stability, Not Just Score**
  For each serious candidate, run repeated evaluations and record:
  - mean score
  - median score
  - standard deviation / variance
  - parse failure count
  - structured-output repair count
  - latency and token cost

- [ ] **Add Variance Penalty To Candidate Selection**
  Update the final candidate ranking so that unstable candidates are penalized even if their raw peak score is high.
  The selected winner should be the most reliable candidate, not the luckiest one.

- [ ] **Track Runtime Accountability Per Candidate**
  Save a machine-readable artifact for every serious candidate containing:
  - parameter diff from baseline
  - source baseline (`active`, `runtime_default`, or `shipped`)
  - model behavior profile snapshot
  - calibration report
  - efficiency report
  - parse failure metrics
  - activation decision rationale

- [ ] **Strengthen Activation Gate**
  Activation should require:
  - no hard reject
  - acceptable variance
  - acceptable parse-failure rate
  - acceptable policy-compliance rate
  - acceptable outcome-verification rate

- [ ] **Ensure First-Use Tuning Uses The Same Pipeline**
  The automatic first-use tuning path on startup must go through the same bounded and accountable tuning pipeline as explicit `--tune`, not a reduced or separate code path.

- [ ] **Cover Newly Added Runtime Features In Calibration**
  Extend the scenario suite and evaluation flow so tuning scores reflect:
  - reasoning-format adaptation
  - final-answer extraction behavior
  - artifact-mode execution
  - command preflight and semantics guard decisions
  - snapshot / rollback side effects where applicable
  - recovery / refinement loops

- [ ] **Emit Clear Tune Summary**
  At the end of tuning, present a concise accountable summary:
  - model id
  - active run id
  - active-profile baseline score
  - runtime-default baseline score
  - shipped baseline score
  - winner score
  - variance
  - certification state
  - whether activation happened
  - why the winner was chosen

## Recommended Safe Parameter Strategy

### Routing / Verification / JSON Units
- Default to near-deterministic settings.
- Favor stability over creativity.
- Penalize malformed JSON and recovery dependence heavily.

### Orchestration Units
- Allow only small creativity bands.
- Evaluate not only success but also plan discipline:
  - fewer redundant steps
  - fewer retries
  - fewer semantically drifting repairs

### Response Units
- Tune for:
  - concise directness
  - plain terminal-safe formatting
  - faithfulness to evidence
- Avoid settings that increase flourish, expansion, or persona drift.

## Acceptance Criteria
- A fresh model can be tuned with no missing-config failure.
- Tuning never mutates prompts.
- Candidate activation prefers low-variance, low-parse-failure profiles.
- A tuned profile is not activated when llama.cpp runtime defaults are equal or more reliable.
- Tuning reports clearly explain why a profile won.
- The tuned profile reflects the real runtime stack, including final-answer extraction and current verification layers.
- Running `--tune` twice on the same model with the same endpoint should produce similar winners or at least similar scores, not wildly different profiles.

## Verification
- Run `cargo run -- --tune` on:
  - one stable non-thinking model
  - one leaky thinking model
- Confirm:
  - no prompt text changes in the active model folder
  - all new runtime units are included in evaluation
  - `model_behavior.toml` influences safe parameter policy
  - llama.cpp runtime defaults appear as an explicit baseline candidate
  - final summary includes stability and accountability data
- Compare two repeated tune runs on the same model and inspect whether the chosen winner is materially consistent.

## Notes
- This task is about making tuning trustworthy.
- The correct outcome is not “the most creative profile.”
- The correct outcome is a bounded, explainable, reproducible profile that improves reliability without causing behavior drift.
