# Task 076: Trace Observability And Session Review Tools

## Priority
**P1 - MAINTAINABILITY**

## Objective
Make failed or suspicious sessions easier to inspect so Elma can be debugged like a premium tool rather than a black box of logs and artifacts.

## Why This Exists
Recent progress depended on repeatedly reading raw session traces by hand. The data is there, but the ergonomics are still poor.

## Scope
- Improve session review ergonomics for:
  - route decision
  - planning source
  - retry/strategy path
  - evidence artifacts produced
  - memory saves/skips
  - final-answer chain
- Add lightweight tooling or commands for inspecting the last failed session.
- Summarize the most important failure facts without hiding raw artifacts.

## Deliverables
- Better trace summaries or inspection utilities.
- A consistent workflow for reviewing suspicious sessions.
- Documentation for failure inspection.

## Acceptance Criteria
- A maintainer can understand the key failure path of a session quickly.
- Raw artifacts remain available, but important facts are surfaced first.
- The tooling works locally without external services.
