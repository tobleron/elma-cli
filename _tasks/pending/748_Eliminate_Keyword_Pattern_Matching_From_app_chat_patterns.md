# Task 748: Eliminate Keyword Pattern Matching From app_chat_patterns.rs

## Type

Architecture / Rule 1 Violation

## Severity

Critical

## Scope

Request classification, shape fallback, and policy routing

## Problem

`src/app_chat_patterns.rs` contains 51+ instances of `lower.contains("...")` used to classify user requests and drive behavior selection:

Active violations:
```rust
pub(crate) fn looks_like_natural_language_edit_request(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    (lower.contains("add ") || lower.contains("append ") || lower.contains("insert ") ...)
    && (lower.contains("section") || lower.contains("line") || lower.contains("end of") ...)
}

pub(crate) fn request_prefers_summary_output(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("summary") || lower.contains("summarize") || lower.contains("bullet point") ...
}

pub(crate) fn request_looks_like_scoped_list_request(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let wants_listing = lower.contains("list ") || lower.contains("show ") || lower.starts_with("ls ") ...;
    wants_listing && !lower.contains("entry point") && !lower.contains("primary entry") ...
}
```

The docs explicitly list `app_chat_patterns.rs` as a **related violation to watch for** under Rule 1:

> "Pattern-matching functions that map request shapes to routes (`app_chat_patterns.rs`)"

These functions are used in `src/app_chat_loop.rs` to apply "shape fallbacks" and "policy fallbacks" — directly mapping request text to program shapes and routing decisions based on keyword presence.

This violates:
- **Rule 1**: No word-based routing
- **Rule 3**: Preferred pattern is "one intel unit, one role, one narrow decision" — not keyword lists
- **Rule 4**: Semantic continuity fails when a user says "append this to the file" and the system treats it differently from "add this at the end of the file" because of keyword differences

## Root Cause

`app_chat_patterns.rs` was created as a set of "helper functions" for the chat loop to quickly detect request shapes. It was never refactored into the intel unit framework, and the commented-out "stress-test patterns" show it has accumulated dead code over time.

## Proposed Solution

### Phase 1 — Audit all call sites

1. Find every caller of functions in `app_chat_patterns.rs`
2. Determine which classifications are actually load-bearing (affect routing/program shape) vs. merely advisory
3. For advisory uses (e.g., UI hints), delete them
4. For load-bearing uses, replace with intel units

### Phase 2 — Replace with intel units or delete

For each load-bearing classification, create a focused intel unit or reuse an existing one:

- `looks_like_natural_language_edit_request` → replace with `IntentAnalysisUnit` (already exists in `intel_units_intent.rs`)
- `request_prefers_summary_output` → delete; the model can decide output format in the tool loop
- `request_looks_like_scoped_list_request` → delete; the model can call `glob` or `ls` directly
- `request_looks_like_scoped_rename_refactor` -> replace with planning/workflow classification that is not keyword-triggered
- `extract_first_path_from_user_text` → keep as a pure text utility (not classification), move to `text_utils.rs`

### Phase 3 — Delete the module

1. Delete `src/app_chat_patterns.rs`
2. Remove module declaration from `main.rs`
3. Update `app_chat_loop.rs` to remove all imports and call sites
4. Verify no other files reference it

## Acceptance Criteria

- [ ] `src/app_chat_patterns.rs` is deleted
- [ ] Zero `lower.contains("...")` keyword-based classification patterns remain in the codebase
- [ ] All former call sites either use intel units, use the model directly, or are deleted
- [ ] `extract_first_path_from_user_text` (if kept) lives in `text_utils.rs` and is documented as a pure utility, not a classifier
- [ ] `cargo build && cargo test` passes
- [ ] No regression in request handling: "list files in src" still works via model tool choice, not keyword routing

## Verification Plan

- `grep -r "contains(\"" src/app_chat_patterns.rs` → file does not exist
- `grep -r "app_chat_patterns" src/` → no references
- Scenario test: "add a section to README" → model calls `read` then `edit` via tool loop, not via keyword-triggered shape fallback
- Scenario test: "give me a summary" → model produces summary via `respond` tool, not via keyword-detected output preference

## Dependencies

- `src/intel_units_intent.rs` (existing intent analysis)
- `src/intel_units/` and the planning path (for narrow classification units)
- `src/text_utils.rs` (for path extraction utility)

## Notes

Do not migrate the keyword lists into prompts as "examples." That would violate Rule 7 (principle-first prompts) and Rule 3 (decomposition beats examples). The correct fix is to delete the keyword layer entirely and trust the model to make tool choices in the tool loop.

The stress-test patterns already commented out in the file should be permanently deleted, not restored.
