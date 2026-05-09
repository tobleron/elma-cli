# Task 764: Add Scope Coverage Ledger As Completion Contract

## Type

Reliability / Evidence Coverage / Completion Gate

## Severity

Critical

## Scope

Discovery tools, read/search tools, finalization, session artifacts, transcript rows

## Problem

Elma can answer broad-scope requests after partial evidence. Earlier docs-wide testing showed only one doc read before an opinion was finalized. The latest testing-prompts turn did read all eight prompt files, but there is still no explicit required-item coverage contract proving that all requested items were covered before finalization.

## Root Cause

The evidence ledger counts gathered evidence but does not represent the required scope. A count of tool outputs is not the same as "all requested files were processed."

## Proposed Solution

- Add a session-scoped `ScopeCoverageLedger` for bounded file/document sets.
- Discovery creates required coverage items.
- Read/search/parse operations mark coverage items as covered, skipped, or failed.
- Finalization must check the ledger before claiming completion.
- Persist coverage under `sessions/<id>/coverage/coverage.json`.
- Surface coverage progress in transcript rows.

## Acceptance Criteria

- [ ] A request over `_testing_prompts/` records all 8 prompt files as required items.
- [ ] A request over `docs/` records discovered docs as required items.
- [ ] Finalization cannot claim completion while required items remain pending without explicit skip/failure explanation.
- [ ] Coverage rows appear in the transcript for broad-scope tasks.
- [ ] Coverage artifacts alone explain what was processed and what was not.

## Verification Plan

- Fixture with five docs: answer blocked until all five are covered or skipped.
- Real replay: `_testing_prompts` prompt records 8/8 covered.
- Real replay: docs-wide prompt records discovered docs and does not finalize after one read.

## Dependencies

Depends on Task 761. Coordinates with Tasks 763 and 766.

## Notes

Do not implement scope detection through user-input keyword lists. Use discovered bounded sets and semantic objective state.

