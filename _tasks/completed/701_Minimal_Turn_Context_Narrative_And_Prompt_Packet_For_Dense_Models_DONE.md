# Task 701: Minimal Turn Context Narrative And Prompt Packet For Dense Models

## Type

Core Architecture / Context Hygiene

## Severity

High

## Session Evidence

Round 3 prompt sessions in `project_tmp/round3_sessions/` show dense-model fragility from noisy continuation and correction context:

- Prompt 01: `prompt_01_s_1778094001_292139000/trace_debug.log` hit `respond_abuse`, then persisted `project_tmp/security_report.md`.
- Prompt 05: `prompt_05_s_1778094570_505046000/trace_debug.log` hit repeated empty `read`, `respond_abuse`, finalization decode failures, and missing artifact notice for `project_tmp/testing_report.md`.
- Prompt 08: `prompt_08_s_1778094994_729906000_interrupted/trace_debug.log` continued after a failed approach and repeated bad `copy` repairs instead of receiving a simpler next-action packet.

The current model-facing narrative appears to mix original request, prior tool results, stop notices, duplicate-skip messages, strategy-shift hints, evidence gates, and finalization prompts as accumulated chat turns. This makes the next action contract hard for dense coder models to follow.

## Problem

Elma needs to stay autonomous for long tasks, but the model should not see an ever-growing operational transcript as the primary control surface. Dense models do better with a simple, stable turn packet:

- current objective
- exact remaining requirement
- evidence already gathered
- forbidden repeats
- next allowed action contract
- artifact/write target if any

## Proposed Solution

Implement a minimal per-turn context packet builder and use it before continuation, repair, and finalization turns.

Likely source areas:

- `src/tool_loop.rs`
- `src/intel_narrative.rs`
- `src/intel_narrative_planning.rs`
- `src/intel_narrative_utils.rs`
- `src/effective_history.rs`
- `src/trace_reducer.rs`
- `src/session_forensics.rs`

Requirements:

- Build a compact `TurnContextPacket` from raw user request, current objective, required artifacts, successful tool outcomes, failed tool signals, and stop reason.
- Replace repeated continuation prose with this packet for budget continuation, mutation enforcement, repair hints, and finalization.
- Keep raw detailed traces on disk, but keep model-facing context minimal.
- Persist the packet per turn under the session folder for forensic review.
- Do not modify `src/prompt_core.rs`.

## Acceptance Criteria

- [ ] Dense prompt sessions no longer show repeated empty action/correction loops caused by accumulated noisy context.
- [ ] Every continuation turn includes one concise current objective and one next-action contract.
- [ ] Duplicate-skip and failed-tool details are summarized as blocked signals, not appended repeatedly as prose.
- [ ] Session artifacts include a readable per-turn context packet.
- [ ] Existing transcript-native visibility is preserved for the user.

## Verification Plan

Run `_testing_prompts/01_prompt.txt`, `05_prompt.txt`, and `08_prompt.txt`.

Pass criteria:

- Prompt 01 and 05 produce required artifacts without repeated empty `read` loops.
- Prompt 08 changes strategy after the first backup failure instead of repeating bad copy/shell repairs.
- `trace_debug.log` shows compact packet creation and no prompt-core changes.

