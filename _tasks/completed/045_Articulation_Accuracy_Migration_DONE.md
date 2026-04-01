# TASK_045: Articulation Accuracy Migration

## Objective
Update the codebase terminology to use more articulate and accurate words for system prompts, classification, and AI decision-making. This ensures the model has a clearer understanding of the expected output and workflow states.

## Research Findings
The current terminology often uses technical or ambiguous terms that can lead to subtle misclassifications or less-than-optimal prompt adherence.

### Recommended Terminology Updates

| Category | Current Term(s) | Recommended Term(s) | Rationale |
| :--- | :--- | :--- | :--- |
| **Workflow Gate** | `CHAT`, `WORKFLOW` | `DIRECT_ANSWER`, `ORCHESTRATED_TASK` | Distinguishes between immediate responses and process-heavy tasks. |
| **Workflow Mode** | `INSPECT`, `EXECUTE`, `PLAN`, `MASTERPLAN`, `DECIDE` | `DISCOVER`, `ACT`, `OPERATIONAL_PLAN`, `STRATEGIC_PLAN`, `EVALUATE` | More descriptive of the actual cognitive or operational mode. |
| **Action Type** | `CHAT`, `SHELL`, `PLAN`, `MASTERPLAN`, `DECIDE` | `CONVERSATION`, `TERMINAL_ACTION`, `OPERATIONAL_PLAN`, `STRATEGIC_PLAN`, `JUDGMENT` | Aligns with user-facing concepts rather than internal command names. |
| **Complexity** | `DIRECT`, `INVESTIGATE`, `MULTISTEP`, `OPEN_ENDED` | `ATOMIC`, `DISCOVERY`, `SEQUENTIAL`, `STRATEGIC` | Better represents the structural nature of the request. |
| **Evidence Mode** | `RAW`, `COMPACT`, `RAW_PLUS_COMPACT` | `VERBATIM`, `SYNTHESIZED`, `HYBRID` | Focuses on the *nature* of the content rather than just its size. |
| **Artifact Status**| `safe`, `maybe`, `keep`, `ignore` | `DISPOSABLE`, `EPHEMERAL`, `PERSISTENT`, `IRRELEVANT` | Clearer lifecycle semantics for workspace files. |
| **Step Status** | `ok`, `retry` | `SATISFIED`, `INCOMPLETE` | Describes the *state* of the result rather than the next action. |
| **Formula Action** | `save`, `skip` | `MEMORIZE`, `DISCARD` | More accurate verbs for long-term knowledge management. |
| **Shell Safety** | `accept`, `revise`, `reject` | `APPROVED`, `REQUIRES_REVISION`, `BLOCKED` | Standard security/safety terminology. |
| **Intention Type**| `ACTION`, `INFO`, `CHAT` | `INSTRUCTION`, `INQUIRY`, `CONVERSATION` | Standard linguistic classification of speech acts. |

## Implementation Plan

### Phase 1: Core Definitions (src/routing_calc.rs, src/intel_units.rs)
- [ ] Update `workflow_code_pairs`, `mode_code_pairs`, `speech_act_code_pairs` in `src/routing_calc.rs`.
- [ ] Update validation logic in `src/intel_units.rs` to reflect new labels.

### Phase 2: System Prompts (src/defaults_router.rs, src/defaults_evidence.rs)
- [ ] Update system prompts in `src/defaults_router.rs` to use new terms and descriptions.
- [ ] Update system prompts in `src/defaults_evidence.rs` (especially `PRESENT_EVIDENCE`, `VERIFY_STEP`, `SAVE_FORMULA`, `SHELL_SAFETY`, `CLASSIFY_ARTIFACTS`).

### Phase 3: Logic & Matching (src/routing_parse.rs, src/evaluation.rs, src/formulas/patterns.rs)
- [ ] Update any hardcoded string matching or mapping logic.
- [ ] Update intent patterns in `src/formulas/patterns.rs`.

### Phase 4: Verification
- [ ] Run `cargo build` to ensure no breaking changes in logic.
- [ ] Run `./run_intention_scenarios.sh` to verify that classification accuracy is maintained or improved.
- [ ] Run `./reliability_probe.sh`.

## Justification
AI models perform better when the semantic labels provided in prompts are precise and mutually exclusive. Moving away from technical jargon (`SHELL`, `WORKFLOW`) toward descriptive intents (`TERMINAL_ACTION`, `ORCHESTRATED_TASK`) helps the model align its internal representations with the system's operational goals.
