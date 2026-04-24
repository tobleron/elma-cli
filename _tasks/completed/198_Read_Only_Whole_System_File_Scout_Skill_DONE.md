# Task 198: Read-Only Whole-System File Scout Skill

## Priority
P1

## Objective
Add a read-only file-scout skill that can search beyond the workspace, disclose where it looked, and hand selected documents to later formula stages.

## Why This Exists
File discovery across the whole machine is distinct from content extraction. The scout should specialize in finding candidate files and reporting search coverage, not in parsing every possible file format itself.

## Required Behavior
- Search beyond the current workspace when requested.
- Stay read-only outside the workspace.
- Disclose searched roots and inspected candidates.
- Hand content extraction to `document_reader` when the selected formula requires document analysis.

## Discovery Rules
- Default to on-demand search, not a persistent index.
- Skip pathological or pseudo-filesystem roots by default unless explicitly targeted.
- Respect explicit user-provided paths or roots when present.
- Rank candidates by relevance and recency only if these signals are cheaply available.

## Default Exclusions
Exclude by default unless explicitly targeted:
- `/proc`
- `/sys`
- `/dev`
- transient mount points or equivalent pseudo-filesystems
- very large cache/vendor directories when they are clearly irrelevant

## Output Contract
Return enough structured information for the next stage or final answer:
- searched roots
- skipped roots
- candidate files
- inspected files
- short reason why candidates were chosen

## Boundaries
- no writes outside the workspace
- no hidden persistent index in v1
- no content extraction duplication; use `document_reader` for file parsing

## Acceptance Criteria
- Elma can discover and summarize files outside the workspace.
- External writes remain forbidden.
- The user can see which roots and files were inspected.
- File discovery and document extraction stay separated by skill responsibility.

## Required Tests
- search outside workspace returns clear root disclosure
- excluded pseudo-filesystems are skipped by default
- explicit root targeting can override default scope restrictions
- formula handoff to `document_reader` does not duplicate extraction logic in `file_scout`
