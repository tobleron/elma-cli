# Task 731: Final Answer Numeric Claim Verification

## Type

Bug / Finalization

## Severity

Medium

## Scope

System-wide

## Session Evidence

Session `s_1778149997_285478000`, Turn 1:

Tool output (`ls docs`):
```
docs/  (55 item(s))
    001-fetch-sandboxing.md  (3999 B, Apr 27)
    001-template.md  (1227 B, Apr 27)
    ... (50 files listed)
_directives/
_proposals/
decisions/
dsl/
scripts/
ARCHITECTURAL_RULES.md  (6945 B, 5d ago)
ARCHITECTURE.md  (48684 B, 3d ago)
... (17 more files)
```

Final answer (`0007_final_answer.md`):
```
**55 items total** (24 files + 31 subdirectories)
```

The `ls` output explicitly states `55 item(s)` but does NOT distinguish files from subdirectories. The model fabricated the breakdown into "24 files + 31 subdirectories". A manual count shows ~50 files and 5 subdirectories. The breakdown is wrong in both numbers and ratio (heavily overcounting subdirectories).

## Problem

The model's final answer includes a numerical claim (24 files, 31 subdirectories) that is not supported by the tool evidence. The system has no guardrail to flag or correct numeric claims that:
- Are derived from evidence but contain fabricated specifics
- Contradict the observable tool output
- Mix correct aggregate numbers (55) with hallucinated breakdowns

This erodes user trust and is especially common with small models that try to "add value" by providing detail beyond what the evidence supports.

## Root Cause Hypothesis

Confirmed: The `finalization_verifier.rs` `validate_file_claims` function only checks file-creation and online-verification claims. It does not check any numeric claims against tool evidence. There is no mechanism to validate derived numbers.

## Proposed Solution

Add bounded numeric claim verification to the finalization pipeline:

1. **`src/finalization_verifier.rs`**: Add a new `ClaimKind::NumericAssertion` variant for numeric claims (patterns like "X files", "Y items", "Z percent").
2. Add `extract_numeric_claims()` function that scans the final answer for numeric assertions.
3. Add `validate_numeric_claims()` that checks whether the asserted number appears in nearby tool evidence.
4. Append a correction note when a numeric claim is unsubstantiated by tool evidence.
5. The check should be lightweight (regex-based) and not block finalization — just append a transparent note.

Implementation plan:

- `src/finalization_verifier.rs`:
  - Add `ClaimKind::NumericAssertion { value: u64, description: String }`
  - Add `extract_numeric_claims(text: &str) -> Vec<FinalClaim>` that uses regex `\b(\d+)\s*(files?|items?|docs?|subdirector(y|ies)|folders?|entries?)\b`
  - Exclude patterns matching `\d+\s*B\b` and `\d+\s*(KB|MB|GB)\b` (file sizes) and `\d+\s*(days?|hrs?|ago)\b` (timestamps) — these come from tool output formatting, not model claims
  - Update `validate_file_claims` (rename to `validate_claims`) to also validate numeric claims
  - For each numeric claim, search nearby tool messages (last 3) for the same number
  - If the number doesn't appear in any tool output, mark as unsupported
  - Add "This number could not be verified against tool evidence" to the unsupported claims appendix
- Update callers in `tool_loop.rs` if the function signatures change.

## Acceptance Criteria

- [ ] `extract_numeric_claims` extracts "24 files" and "31 subdirectories" from the example final answer.
- [ ] `validate_numeric_claims` flags "24 files" as unsupported (ls output shows "55 item(s)" not "24 files").
- [ ] Unsupported numeric claims appear in the `build_unsupported_claims_appendix` output.
- [ ] The evidence gate catches fabricated breakdowns without blocking valid aggregate claims (e.g. "55 items").

## Verification Plan

- Unit tests for `extract_numeric_claims` with various numeric patterns.
- Unit test: session `s_1778149997_285478000` Turn 1 final answer → flags "24 files" as unsupported.
- Unit test: valid claim "55 items" → not flagged (present in tool output).
- Replay session to verify correction appendix is appended.

## Dependencies

None.

## Notes

This is intentionally lightweight — just a regex extraction + tool-evidence cross-check. A more sophisticated approach could use parse trees of numeric expressions, but the simple check catches the most common class of hallucination (fabricated breakdown numbers) with minimal code.
