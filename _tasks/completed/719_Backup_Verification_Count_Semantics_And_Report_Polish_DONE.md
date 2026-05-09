# Task 719: Backup Verification Count Semantics And Report Polish

## Type

Backup Tool / Verification UX

## Severity

Low

## Evidence

Round 6 prompt 08 completed successfully and used the `backup` tool, which is strong progress. The final report still mixed different count semantics:

- backup tool: `323 files matched, 323 copied`
- shell count: `323`
- destination count: `324` because manifest is included
- `ls`: `332 item(s)` because directories are counted too

Session:

- `project_tmp/round6_sessions/prompt_08_s_1778140807_118803000/session.md`

## Problem

The backup is operational, but the verification report should not make the user reconcile file count, manifest count, and directory-entry count manually.

## Requirements

- Have the `backup` tool return canonical verification fields:
  - source_files_matched
  - payload_files_copied
  - manifest_files_created
  - directories_created
  - verification_ok
- Prefer the tool's canonical verification output over follow-up shell counting for final summaries.
- If shell verification is used, label payload files versus manifest files versus directories clearly.
- Update tests for backup manifest/count semantics.

## Acceptance Criteria

- [ ] Prompt 08 final report clearly states that 323 source files were copied and 1 manifest was added.
- [ ] Directory counts are not compared directly against file counts.
- [ ] Backup tests cover the returned count fields.

