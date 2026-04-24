# Task T209: Proactive Budget Forecasting And Mid-Turn Compaction For Shell-Heavy Tasks

## Priority
P0

## Objective
Teach Elma to forecast when a shell-heavy strategy is likely to blow the context budget and to compact or switch strategy before the turn degrades.

## Why This Exists
Session [`sessions/s_1776985838_345642000`](/Users/r2/elma-cli/sessions/s_1776985838_345642000) showed that Elma did not compact or protect the turn even after generating huge shell outputs.

Current code explains why:
- auto-compaction runs in [`src/tool_loop.rs`](/Users/r2/elma-cli/src/tool_loop.rs)
- compaction uses [`src/auto_compact.rs`](/Users/r2/elma-cli/src/auto_compact.rs)
- `CompactTracker::should_compact()` currently requires:
  - enough total estimated message tokens
  - at least 4 user turns

That means a **single shell-heavy request** can produce damaging output without triggering compaction, even though the turn obviously needs budget protection.

## Required Behavior
1. Budget forecasting must happen **before** expensive shell strategies run.
2. Elma must be able to identify high-risk command shapes such as:
   - full-tree per-file loops
   - unbounded `find ... -exec stat ...`
   - commands likely to return thousands of lines
3. For high-risk commands, Elma must choose one of:
   - a cheaper aggregate strategy,
   - bounded preview strategy,
   - preemptive compaction,
   - explicit user-facing disclosure that the scope should be narrowed.
4. Auto-compaction must no longer depend only on multi-turn conversations; shell-heavy single-turn tasks must also be protected.

## Required Forecasting Inputs
At minimum, forecasting should consider:
- current model context estimate
- configured `ctx_max`
- recent tool-result sizes
- predicted output class of the next command
- whether the current request is single-turn but output-heavy

## Required Mid-Turn Policy
When a command result is already huge:
- do not wait until the next unrelated user turn to compact;
- either compact the evidence or aggressively reduce what is passed forward to the model;
- emit transcript telemetry explaining what happened.

## Relationship To Existing Work
This task revives the practical intent of older postponed budgeting work, but within the current Task 191 architecture:
- objective-level budget awareness
- shell-heavy turn protection
- early conservation, not only reactive cleanup

## Integration Points
- `src/tool_loop.rs`
- `src/auto_compact.rs`
- `src/tool_result_storage.rs`
- `src/skills.rs`
- `src/stop_policy.rs`

## Non-Goals
- Do not implement arbitrary heuristic keyword routing.
- Do not compact blindly after every command.
- Do not degrade groundedness by summarizing away critical evidence without preserving access paths.

## Acceptance Criteria
- A single-turn shell-heavy query like the one in `s_1776985838_345642000` is protected by forecasting and does not degrade the turn before Elma can recover.
- Elma can compact or reduce evidence during the same turn when shell output becomes too large.
- Transcript telemetry explains budget pressure and mitigation steps.
- The model retains enough evidence to answer with a grounded estimate instead of failing into repeated bad commands.

## Required Tests
- unit test for single-turn shell-heavy budget forecasting
- regression test for the session-retention size query
- test showing auto-compaction or equivalent mitigation can fire even when user-turn count is below the old minimum threshold
- UI/probe test showing budget/compaction telemetry during shell-heavy turns

