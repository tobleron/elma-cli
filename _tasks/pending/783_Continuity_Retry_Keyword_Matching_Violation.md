# Task 783: Continuity Retry — Tag-Based Validation is Keyword Matching

## Type
Rule Violation / Design

## Severity
High

## Scope
`src/app_chat_loop.rs` lines 1111-1118

## Problem

The continuity retry validation at lines 1111-1118 uses `.contains()` against hardcoded XML-like tag strings to detect if a model response contains tool proposals instead of text:

```rust
let is_valid_answer = trimmed.len() >= 20
    && !trimmed.starts_with('<')
    && !improved.contains("<search_files")
    && !improved.contains("<read ")
    && !improved.contains("<write ")
    && !improved.contains("<glob ")
    && !improved.contains("<shell")
    && !improved.contains("<bash");
```

**This violates Rule 1 (Do Not Turn Elma Into A Keyword Matcher):**
> "Routing, classification, and behavior selection must never use hardcoded word triggers."

The validation is brittle — adding a new tool requires updating this list, and legitimate answer text containing `<bash` (e.g., explaining shell syntax) would be falsely rejected.

## Additional Finding

A similar pattern exists at lines 1441-1444 for the finalization completeness check:

```rust
let looks_complete = final_text.len() > 200
    && !final_text.to_lowercase().contains("partial")
    && !final_text.to_lowercase().contains("incomplete")
    && !final_text.to_lowercase().contains("not yet");
```

This check rejects answers that discuss partial results or incomplete data, even when the answer itself is complete and well-supported by evidence.

## Root Cause

Quick-fix safety guard added to prevent tool-proposal leakage into user-facing answers. The fix worked but used the anti-pattern the project explicitly forbids.

## Proposed Solution

Phase 1: **Tool-proposal detection** — Replace the tag-list check with a structural test:
```rust
// A model response is a tool proposal if it contains well-formed XML tags
// matching the tool schema, not just the word "shell" in angle brackets.
let has_tool_xml = crate::routing_parse::contains_tool_call_xml(&improved);
```

Phase 2: **Completeness assessment** — Replace the keyword-based `looks_complete` check with evidence-based assessment:
```rust
let looks_complete = final_text.len() > 200
    && evidence_ledger.has_minimal_coverage()
    && !stop_outcome.is_bad_stop();
```

This grounds completeness in what the system *actually did* rather than what words appear in the answer.

## Acceptance Criteria
- [ ] No `.contains("tag_name")` patterns in continuity retry path
- [ ] No `.contains("partial")`/`.contains("incomplete")` in finalization completeness check
- [ ] Tool-proposal detection uses structural XML parsing
- [ ] Completeness assessment uses evidence ledger coverage
- [ ] All existing continuity retry tests pass
- [ ] `cargo test` passes

## Verification Plan
- Unit test: Test `contains_tool_call_xml()` with positive and negative cases
- Unit test: Test completeness assessment with various evidence states
- Integration test: Existing continuity retry tests pass
- Regression test: `cargo test`

## Dependencies
- Depends on `routing_parse.rs` having (or adding) `contains_tool_call_xml()`

## Notes
- **Architectural Rule violated:** Rule 1 (No Word-Based Routing) — explicitly forbidden
- **AGENTS.md:** "If you find yourself writing `if input.contains("word")`, you are violating Elma's philosophy"
- This is a **repeat violation** — shell_preflight.rs uses `.contains()` extensively, but that's for security classification which is arguably a different domain
