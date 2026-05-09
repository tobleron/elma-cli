# Task 738: Explicit Coverage Ledger For Read-All-Docs And Large-Scope Requests

## Type

Reliability / Instruction Following / Evidence Coverage

## Severity

Critical

## Scope

Workspace discovery, document intelligence, planning, evidence ledger, finalization verifier, session artifacts, transcript visibility

## Problem

When the user asks Elma to read a broad scope such as all docs, Elma can begin reading files but later drift away from the original coverage requirement. The failure mode is not just weak finalization; the system lacks a durable coverage contract that says which files were requested, which files were discovered, which were read, which were skipped, and why.

Without a coverage ledger, Elma can truthfully gather some evidence but still fail the user's actual instruction because it answers before covering the requested set.

Existing Tasks 741 and 742 improve evidence persistence and enforcement, but they do not define a coverage manifest or finalization gate for exhaustive user scopes. This task is required for the specific "read all docs" failure.

## Proposed Solution

Add a session-scoped `CoverageLedger` that is created when the planner determines the user is asking for broad or exhaustive workspace coverage. The decision must come from the existing intent, scope, and complexity pipeline, not hardcoded keyword triggers.

The ledger should record:

- Raw user request and normalized objective.
- Requested scope boundaries.
- Discovery method and exclude rules.
- Candidate files or artifacts.
- File type category: readable document, source code, config, binary, generated artifact, unsupported.
- Processing strategy per item.
- Status: `pending`, `reading`, `read`, `summarized`, `skipped`, `failed`.
- Evidence references: tool call id, line/page/section range when available, extracted summary path, error message if failed.
- Completion percentage and unresolved blockers.

Readable documents and source code need different first-pass strategies:

- Documents such as Markdown, TXT, PDF, HTML, EPUB, DOCX, CSV, and TSV should be converted or extracted into text with provenance such as page, heading, section, row range, or source offset when available.
- Source code should be scoped by module/import/signature/definition structure first, then read deeper only when the task requires implementation detail.
- Mixed requests should allow Elma to shift between document-analysis behavior and code-agent behavior based on the evidence and user objective, not by forcing a manual mode switch.

Finalization must consult the ledger. If the user requested full coverage and the ledger still has unresolved required items, Elma should continue autonomously, explain a blocker, or produce a clearly partial answer. It must not present a complete answer as if all requested material was read.

## Acceptance Criteria

- [ ] A broad-scope request creates a `CoverageLedger` persisted under `sessions/<id>/coverage/coverage.json`.
- [ ] The ledger is updated after discovery, every read/conversion action, every skip, and every failure.
- [ ] Finalization verifier blocks "complete" answers when required coverage remains pending or failed without explanation.
- [ ] Elma can state exactly which docs were read and which were not.
- [ ] Page, heading, section, or row provenance is preserved for normal readable documents when extractors provide it.
- [ ] Source files are initially summarized by structure before full content reads unless the task requires deeper inspection.
- [ ] The transcript includes collapsible coverage progress rows for large-scope tasks.
- [ ] No routing or mode decision is implemented with hardcoded user-input keyword matching.

## Verification Plan

- Fixture test: create five docs and ask Elma to read all docs; finalization fails until all five are marked read or explicitly skipped with reason.
- Fixture test: include a PDF/HTML/TXT mix and verify extracted text provenance appears in the ledger.
- Fixture test: include source files and docs in the same request; verify source files are handled structurally while docs are extracted as readable content.
- Regression test: ask for a targeted single file; ledger overhead should stay minimal and not force broad scanning.
- Session review test: `coverage.json` alone should explain what Elma did and did not read.

## Notes

Recommendation for the user-reported issue: the main fix is not a larger prompt. Elma needs an explicit coverage contract plus a finalization gate so the original "read all docs" instruction survives planning, tool use, retries, and final answer generation.
