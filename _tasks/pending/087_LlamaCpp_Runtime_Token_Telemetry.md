# Task 087: llama.cpp Runtime Token Telemetry

## Priority
**P1 - TOKEN-AWARE LOCAL RUNTIME FOUNDATION**

## Objective
Give Elma a first-class runtime view of token consumption and remaining context capacity when running against llama.cpp-compatible local endpoints, so orchestration decisions can be grounded in actual budget signals instead of blind approximation.

## Why This Exists
Elma is being built for local small models on constrained hardware. On that target, token waste is not a cosmetic issue; it directly reduces answer quality, planning headroom, and long-task survivability.

llama.cpp exposes operational details that are unusually valuable for Elma:
- prompt tokens consumed
- completion tokens generated
- total tokens used
- context window limits
- remaining room before overflow

That information should become part of Elma's live runtime state rather than staying buried in raw response metadata or debug logs.

## Scope
- Define a dedicated runtime token telemetry module for local inference.
- Normalize token-usage data from llama.cpp responses into a stable internal structure.
- Capture, persist, and expose at least:
  - prompt/input tokens
  - completion/output tokens
  - total tokens
  - configured context maximum
  - estimated remaining context budget
  - request-level and turn-level accumulation
- Distinguish authoritative provider-reported values from inferred fallbacks.
- Keep this llama.cpp-first, while leaving the internal API extensible for future OpenAI-compatible endpoints.

## Deliverables
- A runtime token telemetry type and collection pipeline.
- Session-level accumulation of token usage across the turn and across the conversation.
- Trace/debug visibility for token budget state.
- Clean fallback behavior when an endpoint does not report all fields.

## Design Notes
- This task is about **measuring** runtime token economics accurately.
- It is not yet the forecasting/planning task.
- It should integrate with existing runtime/session structures rather than remaining a loose utility.
- The result should be usable by later budgeting and compaction tasks without provider-specific branching spread throughout the codebase.

## Acceptance Criteria
- When running on llama.cpp, Elma can reliably know how many tokens were consumed and how much context remains.
- Token telemetry is available to orchestration code, not only logs.
- Missing provider fields degrade gracefully with explicit fallback labeling.
- Real CLI traces show token budget state clearly enough to debug long-task failures.

