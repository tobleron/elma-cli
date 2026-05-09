# Task 718: Autonomous Continuation After Iteration Limit For Unfinished Objectives

## Type

Autonomy / Stop Policy / Approach Engine

## Severity

High

## Evidence

Round 6 prompts 01-05 frequently hit `iteration_limit_reached` and then finalized through deterministic artifact completion. The sessions were marked `completed` even when the requested report had not been substantively written.

Examples:

- prompt 01: `StopReason Budget limit: iteration_limit_reached`
- prompt 02: `StopReason Budget limit: iteration_limit_reached`
- prompt 05: `StopReason Budget limit: iteration_limit_reached`

## Problem

Elma should be a continuing autonomous agent by default. An iteration limit is a budget boundary, not proof that the objective is finished. When objective completion is not verified, Elma should continue through a bounded sibling approach, clean-context finalization, or explicit partial-completion state.

## Requirements

- Treat `iteration_limit_reached` as an approach failure or continuation trigger when required artifacts or requested outcomes are not verified.
- Add a continuation decision that considers:
  - required artifact verifier result
  - evidence sufficiency
  - repeated tool failure/stagnation class
  - elapsed runtime timeout budget
  - model runtime profile
- Prefer a clean-context synthesis/finalization continuation before raw deterministic fallback.
- Record continuation decisions in transcript rows and trace.
- Mark sessions as partial/failed when Elma cannot complete the objective after bounded continuation.
- Keep long-running defaults generous while preserving hang/stagnation detection.

## Acceptance Criteria

- [ ] Iteration limit with unverified artifacts does not produce a plain `completed` session status.
- [ ] The transcript shows whether Elma continued, forked a sibling approach, or ended partial.
- [ ] Prompt 01/02-style report tasks get at least one bounded continuation chance before deterministic evidence recovery.
- [ ] Tests cover verified artifact, unverified artifact, and no-artifact objectives.

