# Task 542: Search Scope Grounding for Risk Analysis

**Status:** pending
**Priority:** MEDIUM
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P6 — High Confidence

## Summary

When Elma performs searches for risks or patterns, it currently searches the entire workspace including `_knowledge_base/` which contains third-party reference source code. During the session, a search for hardcoded string trigger patterns returned "1132 matches" — the majority of which were in `_knowledge_base/_source_code_agents/` (goose, codex-cli, dirac), not in Elma's own source. The model presented this as evidence of Elma's own violations.

## Evidence

- `session.md` line 145: "Search found 1132 matches for hardcoded patterns across workspace"
- `terminal_transcript.txt` line 481-485: matches shown in `_knowledge_base/_source_code_agents/goose/`, `codex-cli/`, etc.
- Risk 1 in final answer labeled Medium severity based on this inflated count

## Root Cause

The `search` tool's default scope is the workspace root. Analysis prompts do not instruct the model to scope searches to `src/` only. The model did not narrow the scope before citing match counts.

## Implementation Plan

1. In the search tool's description/prompt, add guidance: _"When searching for patterns in Elma's own code, scope to `src/` or `elma-tools/src/` to exclude third-party reference material in `_knowledge_base/`."_
2. In the analysis intel unit (or the session prompt for audit-type tasks), prepend: `Scope all code searches to src/ and elma-tools/src/ unless explicitly asked to include reference material.`
3. Consider adding a `--glob '!_knowledge_base/**'` default exclude pattern to the search tool when in "analysis" context
4. Add a post-search annotation to the tool result if matches include `_knowledge_base/`: `ℹ️ N of M matches are in _knowledge_base/ (third-party reference code) — excluding from risk analysis.`

## Success Criteria

- [ ] Risk analysis searches default to `src/` scope
- [ ] Match counts in the final answer reflect only Elma's own code
- [ ] Third-party matches are either excluded or clearly labeled as non-applicable

## Verification

```bash
cargo build
# Run an audit session and verify risk 1 search scope is src/ only
# Check that match counts are accurate
```
