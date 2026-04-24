# Task T208: Context Bar And Model Budget Divergence For Large Tool Output

## Priority
P0

## Objective
Fix the misleading 100% context indicator when the UI transcript is bloated by large tool traces even though the model-facing message set has already been budgeted or persisted.

## Why This Exists
In session [`sessions/s_1776985838_345642000/trace_debug.log`](/Users/r2/elma-cli/sessions/s_1776985838_345642000/trace_debug.log), Elma generated enormous shell output from bad `stat` loops.

The UI then showed the context bar at 100%, but the code path reveals two different notions of "context":
- model-side messages are budgeted in [`src/tool_result_storage.rs`](/Users/r2/elma-cli/src/tool_result_storage.rs)
- footer context estimate is computed from the full transcript in [`src/ui/ui_terminal.rs`](/Users/r2/elma-cli/src/ui/ui_terminal.rs)

The current footer estimate counts:
- transcript assistant text
- transcript thinking text
- full tool-trace output
- live streaming buffers

This makes the bar a poor proxy for the actual model budget during shell-heavy turns.

## Required Behavior
1. Distinguish **model budget** from **transcript volume**.
2. The footer context bar must primarily reflect the **model-facing message budget**, not raw transcript history size.
3. If transcript size is also worth showing, it must be shown as a separate concept, not merged into the same percentage.
4. Large persisted tool outputs must not cause the footer context bar to imply the model has already consumed that full payload when it has not.

## Required Design
Introduce two clearly named quantities:
- `model_context_tokens_estimate`
- `transcript_tokens_estimate`

Use `model_context_tokens_estimate` as the primary footer context percentage.

Optional transcript-volume disclosure may appear:
- in transcript telemetry
- in verbose mode
- or as secondary status text

but it must not masquerade as model context utilization.

## Session Findings To Preserve
- `find sessions -type f | while read f; do stat ...` produced outputs over 1 MB.
- tool-result persistence wrote large outputs to session files.
- despite that, the footer still appeared to saturate from transcript output accumulation.

## Integration Points
- `src/ui/ui_terminal.rs`
- `src/app_chat_loop.rs`
- `src/tool_loop.rs`
- `src/tool_result_storage.rs`
- any shared token-estimation helpers

## Non-Goals
- Do not remove useful transcript history.
- Do not hide tool traces to make the bar look better.
- Do not invent exact provider token counts where only an estimate exists.

## Acceptance Criteria
- Replaying the regression prompt from `s_1776985838_345642000` no longer pegs the footer at 100% solely because of transcript/tool-trace volume.
- The footer context percentage tracks the model-facing prompt budget more honestly than before.
- A user can still access transcript-volume information somewhere in the UI when needed.

## Required Tests
- unit test for footer estimate using persisted large tool results
- PTY/UI test showing transcript-heavy turn without false 100% model-budget saturation
- regression test for a shell-heavy turn where transcript volume and model-context estimate diverge

