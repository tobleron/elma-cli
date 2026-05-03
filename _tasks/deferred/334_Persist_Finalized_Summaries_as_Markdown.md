# 334: Persist Finalized Summaries as Markdown in Session Folders

## Backlog Reconciliation (2026-05-02)

Superseded by completed Task 385. If reopened, reconcile with Task 469 session-state ownership before adding new summary storage.


## Objective

Add a `summaries/` folder inside each session directory where every finalized summary generated during that session is stored as individual markdown files. This allows users to later review, revise, and reuse their session summaries.

## Motivation

Session summaries are valuable artifacts that users may want to:
- Reference later to recall key decisions and conclusions
- Edit or expand upon after the session ends
- Build documentation or knowledge bases from chat history
- Reuse reasoning or conclusions across multiple sessions

Currently, summaries exist only in the chat transcript and are lost once the session ends.

## Success Criteria

- Each finalized summary creates a new markdown file in `session_<id>/summaries/`
- Files are named with timestamps and incremental numbering (e.g., `2026-04-29_04-50-01_summary_1.md`)
- Summary files contain timestamp, the summary content, and optionally metadata (model, tokens, duration)
- The summaries folder is created automatically on first summary write
- No changes to existing summary generation logic — only post-processing storage
- Feature works for both interactive and non-interactive sessions
- Zero impact on session performance or storage quota

## Scope

- **In scope:**
  - Writing finalized summaries to markdown files
  - Creating the `summaries/` directory per session
  - Robust filename generation (collision-safe, readable)
  - Integration into the existing summary-finalization flow

- **Out of scope:**
  - Editing or revising summaries from CLI (just storage)
  - Indexing or search over summaries
  - Backfilling summaries from past sessions
  - Synchronization or sharing of summaries

## Implementation Plan

### 1. Locate Summary Finalization Hook

Identify where in the codebase finalized summaries are emitted. This is likely in:
- The response rendering or finalization step
- The place where "final" message type is determined
- The chat transcript serialization logic

Search for keywords: "summary", "finalized", "final answer", message type final.

### 2. Define Session Directory Structure

Confirm session storage location and structure:
- Sessions are stored under a base directory (likely `~/.elma/sessions/` or project-specific)
- Each session has a unique ID folder with metadata and transcript
- Add `summaries/` as a subdirectory alongside existing artifacts

### 3. Add Markdown Writer

Create a small module that:
- Accepts summary text and session context
- Generates a YYYY-MM-DD_HH-MM-SS prefixed filename
- Writes a markdown file with frontmatter (optional) and the summary body
- Handles I/O errors gracefully (do not fail the session on disk error)
- Ensures `summaries/` directory exists before writing

### 4. Wire Into Finalization Pathway

Integrate the markdown writer into the summary finalization hook so that every time a summary is finalized, it is concurrently written to disk.

### 5. Error Handling & Edge Cases

- Session directory missing (should not happen but log warning)
- Disk full / permission errors (log and continue)
- Concurrent writes (lock-free is fine; filenames are unique by timestamp)
- Non-UTF8 content (summary should already be UTF-8)

### 6. Verification

- Run a test session that produces at least two summaries
- Check that `summaries/` directory appears with correctly named `.md` files
- Verify file contents match the displayed summary
- Confirm no performance degradation or unexpected errors

## Questions & Open Issues

- Should summaries be written even if the session is transient (non-persisted)? Assume yes — user controls session persistence separately.
- Should summaries include the full prompt context or just the answer? Just the finalized summary text (what user sees).
- Should we add a command to list/read summaries later? Out of scope for this task, but easy extension. Leave as future work.
