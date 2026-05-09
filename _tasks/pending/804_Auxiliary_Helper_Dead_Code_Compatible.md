# Task 804: Auxiliary Helper — Dead-Code Compatible (Compile-Out)

## Status
- **Priority:** High
- **Assignee:** Unassigned
- **Status:** Pending
- **Session:** s_1778364110_268838000

## Objective
Make the auxiliary LLM helper entirely dead-code compatible so it can be compiled out of the binary. When disabled, Elma should behave as if the helper module does not exist at all — zero runtime checks, zero conditional branches, zero logged warnings. In the session, `auxiliary_helper_disabled` was logged on every single iteration, adding noise and revealing that the system is aware of a missing component.

## Root Cause
The auxiliary helper is disabled via a runtime config flag (`runtime.auxiliary.enabled = false`), which means:
- The code still compiles into the binary (bloat)
- Conditional checks run on every iteration (`auxiliary_helper_disabled` trace log)
- The system "knows" something is missing, creating a semantic gap

## Requirements
- Gate all auxiliary helper code behind a Cargo feature flag (`auxiliary-helper`), disabled by default.
- When the feature is not enabled, the helper module is completely compiled out — no trace logs, no runtime checks, no conditional paths.
- `#[allow(dead_code)]` or feature-gated `#[cfg(feature = "auxiliary-helper")]` on all auxiliary-related structs, functions, and imports.
- Remove the `auxiliary_helper_disabled` trace log entirely — when compiled out, there is nothing to log.
- Keep the runtime config field for forward compatibility, but it should be inert when the feature is off.

## Failure Mode Fixed
- Binary bloat from unused helper code
- Noisy trace logs (`auxiliary_helper_disabled` on every iteration)
- Runtime awareness of a disabled subsystem (should be as if it never existed)

## Non-Goals
- Do NOT add runtime warnings about missing helper or model size checks.
- Do NOT auto-enable anything — the feature is either compiled in or it isn't.
- Do NOT change the default behavior for users who compile WITH the feature.
