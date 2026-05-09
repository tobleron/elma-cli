# Task 690: Evidence Grounded Finalization Honesty

## Type

Finalization

## Severity

High

## Scope

System-wide

## Session Evidence

Several final answers made claims not supported by session evidence:

- Prompt 03 final answer in `sessions/s_1778084708_633588000/artifacts/*final_answer.md` says external API references were verified and claims `Created: external_api_reference_report.md`, but no network/fetch evidence or file exists.
- Prompt 07 final answer in `sessions/s_1778085464_714465000/artifacts/*final_answer.md` claims 4 TODO comments were identified and describes example replacements, but the session evidence includes no successful edits and states the actual TODO content was not captured.
- Prompt 08 final answer in `sessions/s_1778085552_840248000/artifacts/*final_answer.md` says the backup completed successfully, but local verification found 1,905 files in the backup while the source query matched 10,860 `.rs` files.

## Problem

Final answers can overclaim completion, fabricate examples, or present unsupported verification as fact. This directly violates Elma's truth-grounded and enterprise-grade reliability goals.

## Root Cause Hypothesis

Confirmed: finalization receives partial evidence but does not enforce claim-to-evidence validation.

Likely: current continuity scoring can remain high even when required actions were not performed.

## Proposed Solution

Add a final-answer evidence gate:

- Inspect `src/final_answer.rs`, `src/continuity.rs`, `src/evidence_ledger.rs`, `src/session_forensics.rs`, and `src/trace_reducer.rs`.
- Extract atomic final-answer claims about files created, edits applied, online verification, command success, and completion.
- Validate each claim against successful tool events and artifact existence.
- Downgrade or rewrite unsupported claims into incomplete-status language before display.
- Make unsupported-claim corrections visible in trace and session events.

## Acceptance Criteria

- [ ] Final answers cannot say a file was created unless a successful write/shell event and filesystem check support it.
- [ ] Final answers cannot say online verification happened unless a network-capable tool event supports it.
- [ ] Example placeholders are labeled as examples and never presented as performed changes.

## Verification Plan

Replay prompts 03, 07, and 08. Inspect final answers and confirm unsupported claims are blocked or rewritten with explicit limitations.

## Dependencies

Task 688.

