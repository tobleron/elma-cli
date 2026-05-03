# T316: Evidence-Aware Read Compaction

## Summary

Replace the current inline compaction for read-heavy sessions with evidence-aware
compaction that preserves file inventory, per-file summaries, raw artifact references,
and enough semantic detail to answer after compaction.

## Motivation

Current `auto_compact` summarizes old tool messages to short snippets, so it is not a
sufficient guarantee that "all required files were read and remain comprehensible after
compaction."  When the model reads many files in a session and the context window fills,
compaction may drop critical file contents in favor of brief summaries.

## Scope

- Track which files have been read (file inventory) across all `read` tool calls.
- Generate per-file semantic summaries that retain key facts, signatures, and structure.
- Preserve artifact paths so the model can reference raw content from the session
  artifacts directory when needed.
- Ensure compaction decisions are visible in the transcript as collapsible rows
  (per AGENTS.md rule 6).
- Integrate with the new session layout (`session.json` for summaries, `artifacts/` for
  raw outputs).

## Constraints

- Must not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT`.
- Must not degrade small-model effectiveness (per AGENTS.md success standard).
- Must preserve enough semantic continuity that the model can answer questions about
  previously-read files after compaction fires.

## Related

- Session layout restructure (new `session.md`, `session.json`, `thinking.jsonl`, `artifacts/`).
- Evidence cap removal in `src/tool_loop.rs` (20-entry cap already removed).
- Document read budget default changed to `Full` mode in `src/document_adapter.rs`.
