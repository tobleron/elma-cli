# Task 699: Mutating Request Execution And Verification Contract

## Type

Execution Correctness

## Severity

High

## Scope

Edit/write/backup tasks, verification, finalization

## Session Evidence

Prompt 07 asked Elma to replace security TODO comments with ISSUE tags and create a summary. The final answer only proposed a plan and said it was ready to proceed, even though the user had already requested the transformation. No edit or summary file was created.

Prompt 08 asked Elma to create and verify a backup. It created backup directories, but the final answer reported a verification mismatch caused by comparing different source sets. The verifier accepted this incomplete verification path.

## Problem

For mutating tasks, Elma can degrade from "do the work" into "plan the work" or can perform an operation without a coherent verification contract. This violates semantic continuity and autonomous completion expectations.

## Proposed Solution

Add a mutating-task execution contract:

- Detect requested edits, replacements, backups, and generated summaries as concrete execution tasks.
- Require at least one mutating tool call or an explicit safety refusal before finalizing.
- For backups, persist the exact source selection manifest and verify against the same manifest.
- For edit tasks, verify changed files with search/diff and write a summary artifact.
- Never answer "ready to proceed" when the user already asked Elma to proceed.

## Acceptance Criteria

- [ ] Prompt 07 either edits matching TODOs and writes a summary, or explicitly reports no matching TODOs after scoped evidence.
- [ ] Prompt 08 verifies backup counts using the same file-selection predicate used to create the backup.
- [ ] Mutating requests cannot finalize with only a plan unless blocked by a documented safety gate.
- [ ] Final answers cite the created/modified artifacts and verification evidence.

## Verification Plan

Replay prompts 07 and 08 in a clean workspace snapshot. Inspect git diff, backup manifests, session traces, and generated summary artifacts.

