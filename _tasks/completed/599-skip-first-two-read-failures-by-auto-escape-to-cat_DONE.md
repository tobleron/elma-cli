# Task 599: Skip First Two Read Failures — Auto-Escape to `cat`

## Session Evidence
Session `s_1777824575_8073000`: The model called `read` without `filePath` parameter 3 times per cycle:

```
Iter 5: TOOL FAIL [read] filePath: required field 'filePath' is missing
Iter 6: TOOL FAIL [read] filePath: required field 'filePath' is missing  
Iter 7: TOOL FAIL [read] filePath: required field 'filePath' is missing
Iter 8: TOOL OK [shell] cat docs/ARCHITECTURE.md  ← strategy shift finally works
```

The error feedback is perfectly clear: `Tool 'read' expects: filePath (path (string)) [required]... Example: {"filePath": "src/main.rs"}`. Yet the Huihui-Qwen3.5-4B model continues calling `read` without `filePath` until the identical-error loop fires at attempt 3. This wastes 12% of the budget (3 of 20 iterations) per cycle.

The arg repair (Task 586, `tool_calling.rs:158-188`) tries to extract a file path from raw args, but when the model sends `{}` or empty arguments, there's nothing to extract.

## Problem
The small model is fundamentally incapable of reading validation error messages and correcting its arguments. It requires the harsh identical-error injection (3 failures) to switch strategies. Every cycle wastes 3 iterations on this pattern.

This is not a system bug — it's a small-model limitation. But the system can compensate.

## Solution
After the FIRST `read` validation failure (not 3rd), automatically inject a corrective hint that preempts the identical-error loop. In `tool_loop.rs`, within the tool execution result handler, detect when a tool fails validation with a specific "missing required field" pattern and immediately suggest an alternative:

1. In `src/tool_loop.rs`, after `stop_policy.record_tool_result(tc, &result)` (line ~1396), check: if `result.ok == false` AND `tc.name == "read"` AND the error mentions "filePath: required field", AND this is the FIRST such failure, immediately inject a system message suggesting `cat`:

```rust
if !result.ok && tc.function.name == "read" 
    && result.content.contains("filePath: required field") 
    && read_failure_count == 1 
{
    // Preemptive strategy shift: the small model won't learn from error messages.
    // Skip the 3-iteration identical-error loop and suggest shell cat immediately.
    messages.push(ChatMessage::simple(
        "system",
        "The 'read' tool requires a filePath argument. Use 'shell cat <path>' instead. Example: shell command='cat docs/ARCHITECTURE.md'"
    ));
}
```

2. Track `read_failure_count` per cycle to limit this to the first occurrence.

This saves 2 iterations per cycle (6 iterations in the session studied), and prevents the model from burning through its budget on an unwinnable fight.
