# Task 768: Add Relevance And Expiry To Effective History Context

## Type

Context Management / Small-Model Effectiveness / Long-Running Autonomy

## Severity

High

## Scope

Effective history, compaction, evidence injection, turn context packet

## Problem

Elma currently carries stale context in ways that confuse later turns. Tool failures and deliverables from previous turns can remain live in the model narrative even when they are irrelevant to the current objective. This caused stale artifact requirements and final answers in the latest session.

## Root Cause

Context retention is based on recency and summarization, not relevance to the current objective. Failed tool calls, stale artifact requirements, and old evidence are not expired from the live model packet even though they remain available in session trace.

## Proposed Solution

- Add relevance labels to evidence, failed tools, artifacts, and summaries.
- Keep trace/session artifacts complete, but build live model context from current objective plus relevant recent facts only.
- Expire irrelevant failed tool calls from live context after a few turns or after objective changes.
- Keep prior successful evidence only when it supports the current objective.
- Surface compaction/expiry decisions as transcript-native operational rows.

## Acceptance Criteria

- [ ] Prior turn deliverables do not appear in an unrelated current-turn context packet.
- [ ] Failed tool calls can expire from live context while remaining in trace.
- [ ] Effective history includes the current objective and relevant evidence, not stale artifacts.
- [ ] Transcript rows indicate when evidence or failures are expired from live context.

## Verification Plan

- Replay latest session and inspect model-facing context packets.
- Unit test relevance filtering for prior artifacts, prior failures, and current objective evidence.
- Long session test with five unrelated turns confirms no stale deliverable leakage.

## Dependencies

Depends on Task 761. Coordinates with Task 767.

## Notes

This should improve small-model accuracy by reducing context noise without hiding audit history.

