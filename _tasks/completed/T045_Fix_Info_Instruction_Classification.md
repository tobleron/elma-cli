# Task T045: Fix INFO vs INSTRUCTION Classification for Actionable Questions

## Problem
User request: "what is current date and current path?"

Was classified as:
```
speech=INFO (99%) → No self-question → Direct workflow
```

Should be classified as:
```
speech=INSTRUCTION → Self-question → SHELL method
```

**Root cause**: The classifier sees "what is" and assumes INFO (question), but the user actually wants Elma to DO something (get the date and path).

## Analysis

### Current Classification Rules (speech_act.toml):
```
- INFO: User seeks information, explanation, or answer (what is, how does, why, explain...)
- INSTRUCTION: User wants Elma to DO something (action verbs: count, list, show, find...)
```

### The Gap:
Questions that imply action are misclassified:
- "what is current date?" → INFO (wrong) → Should be INSTRUCTION
- "how many files?" → INFO (wrong) → Should be INSTRUCTION
- "where am I?" → INFO (wrong) → Should be INSTRUCTION

These are **implicit action requests** - the user wants Elma to run a command and report results.

## Solution

### Option A: Update Classification Prompt (Simple)

Update `config/{model}/speech_act.toml`:

```toml
# Add explicit guidance for implicit action requests
system_prompt = """
...

Special cases - classify as INSTRUCTION even if phrased as question:
- "what is current date?" → INSTRUCTION (run date command)
- "what is my current path?" → INSTRUCTION (run pwd command)
- "how many files?" → INSTRUCTION (run find | wc -l)
- "where am I?" → INSTRUCTION (run pwd)
- "what's in this directory?" → INSTRUCTION (run ls)
- "show me X" → INSTRUCTION (even though "show" is not a verb)

Rule of thumb: If answering requires running a shell command, it's INSTRUCTION.
"""
```

### Option B: Add Post-Classification Correction (Robust)

In `src/app_chat_core.rs`, after classification:

```rust
// Check for implicit action requests
let implicit_action_patterns = [
    "what is current date",
    "what is my current path",
    "what is current path",
    "how many files",
    "where am i",
    "what's in this directory",
    "what files",
    "how many lines",
];

let lower_line = line.to_lowercase();
if implicit_action_patterns.iter().any(|p| lower_line.contains(p))
    && route_decision.speech_act.choice.eq_ignore_ascii_case("INFO") 
{
    // Override to INSTRUCTION
    trace(&runtime.args, "classification_override INFO→INSTRUCTION (implicit action request)");
    route_decision.speech_act.choice = "INSTRUCTION".to_string();
}
```

### Option C: Combine Both (Best)

- Update prompt for general guidance
- Add post-classification override for specific patterns
- Log overrides for learning

## Implementation Steps

1. **Update speech_act.toml** with implicit action guidance
2. **Add post-classification override** in `app_chat_core.rs`
3. **Add logging** to track overrides
4. **Test** with problematic patterns

## Acceptance Criteria
- [ ] "what is current date?" → INSTRUCTION
- [ ] "what is my current path?" → INSTRUCTION
- [ ] "how many files?" → INSTRUCTION
- [ ] "where am I?" → INSTRUCTION
- [ ] Self-question triggered for these cases
- [ ] All existing tests still pass

## Files to Modify
- `config/{model}/speech_act.toml`
- `src/app_chat_core.rs`

## Priority
CRITICAL - Prevents wrong classification flow

## Expected Impact
- **Correct routing** for implicit action requests
- **Self-questioning triggered** → better method selection
- **Fewer verification failures** - right approach from start
