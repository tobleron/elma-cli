# Task 008: Multi Critic Reviewer System

## Objective
Add optional specialized reviewers after the single-critic and verification core have stabilized, so Elma can improve logical consistency, efficiency, and risk awareness without overcomplicating the main execution path too early.

## Context
This is a later-stage refinement task. Right now Elma's biggest problems are:
- weak recovery
- inconsistent outcome verification
- workflow drift before execution finishes

A multi-critic system can help, but only after the main planner and verification layers are trustworthy.

## Work Items
- [ ] Start with a staged design, not a full weighted council on day one.
- [ ] Implement distinct reviewer prompts for:
  - logical reviewer
  - efficiency reviewer
  - optional risk reviewer
- [ ] Keep the existing deterministic safety policy and preflight logic as the primary safety barrier.
- [ ] Make the risk reviewer advisory unless or until there is a strong reason to escalate it into a blocking gate.
- [ ] Route reviewer feedback into recovery/orientation logic only after the adaptive loop is in place.
- [ ] Measure latency and solve-rate impact before enabling multiple critics broadly.

## Acceptance Criteria
- Additional reviewers improve at least one meaningful quality dimension without materially degrading latency or reliability.
- Reviewer disagreements are handled coherently.
- The system can be enabled incrementally rather than all at once.
- Multi-critic review remains subordinate to the core safety policy.

## Verification
- `cargo build`
- `cargo test`
- controlled scenarios for redundancy detection, risky command proposals, and efficiency improvements
- compare solve rate and latency before/after enabling additional critics
