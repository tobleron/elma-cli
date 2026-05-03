# Task 603: Deterministic Evidence-Answer Contradiction Detector

## Type

Model Robustness

## Severity

Critical

## Scope

System-wide

## Session Evidence

Session `s_1777833151_802415000` (2026-05-03 21:32):

- `date +%A` returned: `Sunday`
- Model thinking `0005_thinking.txt`: "The shell command returned 'Sunday'"
- Model thinking `0006_thinking.txt`: "What day of the week? - Sunday (from the shell command)"
- Task 601 evidence finalizer ran (trace): `routing voluntary stop through evidence finalizer`
- Faith model output `0007_final_answer.txt`: "However, I don't have the current date/time from the system in my available evidence. To get the exact day of the week, you could run: date +%A"

Even with a clean context containing only the question + evidence block ("Sunday"), the 4B model STILL produced a wrong answer. The continuity retry also failed to fix it.

The evidence finalizer (Task 601) helps but is not sufficient. The small model simply cannot reliably use explicit evidence.

## Problem

The model produces factual answers that directly contradict tool outputs. The system has no defense against this — it trusts whatever the model says. When the model says "I don't have the current date" but `date +%A` returned "Sunday", the answer is provably wrong and the system does nothing about it.

This breaks semantic continuity absolutely: the user asked a factual question, the system gathered the correct answer, and then presented a wrong answer.

## Root Cause Hypothesis

**Confirmed**: 4B model exhibits "evidence blindness" — even when factual evidence is placed directly in its context with explicit instructions to use it, the model falls back on its default knowledge or the workspace context. This is a fundamental limitation, not a prompt problem.

**Confirmed**: No system-level check verifies that the final answer is consistent with gathered evidence.

## Proposed Solution

Add a lightweight, deterministic post-hoc contradiction detector that runs after the final answer is generated (both tool loop path and continuity retry path). If specific contradictions are found, replace the wrong claims with the correct evidence-derived facts.

### Implementation in `src/final_answer.rs`:

```rust
/// Check the final answer against evidence and correct known contradictions.
/// Returns the corrected answer (unchanged if no contradictions found).
fn correct_evidence_contradictions(answer: &str, tool_results: &[ToolResult]) -> String {
    let mut corrected = answer.to_string();

    for result in tool_results {
        // Pattern 1: shell "date +%A" returned a day name, but answer says "don't have date"
        if result.command.contains("date") && result.success {
            let output = result.output.trim();
            // Detect day-of-week pattern matches: Sunday, Monday, etc.
            if let Some(day) = extract_day_of_week(output) {
                // Check if answer contains negation patterns about date/day
                if contains_date_denial(&corrected) {
                    // Replace or append the correction
                    corrected = inject_corrected_claim(&corrected, "day of week", &format!("Today is {day}"));
                }
            }
        }

        // Pattern 2: glob found files, but answer says a WRONG location
        // Pattern 3: stat/file_size returned data, but answer says "don't know"
        // ... extensible for other contradictions
    }

    corrected
}
```

Key design:
- **Not LLM-based**: deterministic string/regex checks, no extra HTTP calls
- **Extensible**: new contradiction patterns can be added as discovered
- **Safe**: only modifies answers when there's a CLEAR contradiction (false positive = minor formatting issue, false negative = same bug as now)
- **Runs at finalization time**: in `process_final_answer` or right before TUI push

### Contradiction patterns to detect:

1. **Date denial**: answer says "don't have/can't access/don't know" about current date/time, but `date +%A` or `date` was run successfully
2. **File location denial**: answer says file is at "docs/X" but `glob` found it at "project_tmp/X"
3. **File existence denial**: answer says "doesn't exist" but `exists` or `stat` returned success

### Python-like regex approach:

```rust
/// Detect "I don't have access to the date" patterns
fn contains_date_denial(text: &str) -> bool {
    let denials = [
        "don't have access to the real-time system clock",
        "cannot access the real-time system clock",
        "don't have the current date",
        "don't have the exact current day",
        "can't access the current date",
        "cannot determine the current day",
        "don't have the current day of the week",
        "cannot determine the day of the week",
    ];
    let lower = text.to_lowercase();
    denials.iter().any(|d| lower.contains(d))
}
```

## Acceptance Criteria

- [ ] When `date +%A` returned "Sunday" and the answer contains date-denial language, the answer is corrected to include "Today is Sunday"
- [ ] When the answer DOES correctly mention the day, it is not modified
- [ ] The correction is transparent: the original model output is preserved in the artifact, with a note that a contradiction was corrected
- [ ] The contradiction detector does not make new HTTP calls (deterministic only)
- [ ] New contradiction patterns can be added easily

## Verification Plan

1. Create a test fixture: tool outputs contain `Sunday` via shell, model answer says "I don't have the current date"
2. Run the contradiction detector on the answer
3. Verify the output contains the correct day of week
4. Create a test fixture: tool outputs contain `Sunday`, model answer correctly says "Today is Sunday"
5. Verify the answer is NOT modified
6. Run against the session `s_1777833151_802415000` final answer
7. Verify it corrects the date claim

## Dependencies

Task 601 (evidence finalizer) — this task adds the next layer of defense when 601 is insufficient.

## Notes

This approach follows Elma philosophy section 7: "Never blame the model — improve the system." Instead of expecting the 4B model to reliably use evidence (which it can't), the system adds a deterministic safety net.

The contradiction detector should run as the LAST step in `process_final_answer`, after all other sanitization/stripping.
