# Task 586: Pre-Execution Tool Call Argument Repair for `read`

## Session Evidence
Session `s_1777822834_658323000`: The model called `read` 16 times (8 per cycle × 2 cycles), every single one with a missing `filePath` parameter. The error feedback showed the full schema — `"Tool 'read' expects: filePath (path (string)) [required], offset (non-negative-integer) [optional], limit (non-negative-integer) [optional]"` — but the model still produced calls with `Arguments: {}`.

The trace shows all 16 failures had the same root cause:
```
[TOOL_VALIDATION_ERROR] tool=read error=filePath: required field 'filePath' is missing
```

## Problem
The 4B model cannot reliably produce valid JSON tool-call arguments with named parameters. Even with explicit schema examples in the error feedback, it repeatedly produces empty or malformed arguments. The tool validation layer rejects these calls but the model never recovers — it spends entire iteration budgets on repeated failures.

## Solution
Add a pre-execution repair pass that runs AFTER JSON parsing but BEFORE schema validation:

1. For the `read` tool specifically: if `filePath` is missing from parsed args but the raw arguments string contains a recognizable path (quoted string that looks like a file path), use regex to extract it and inject it as `filePath`

2. Regex pattern: match the first occurrence of a quoted path-like string in the raw arguments JSON, e.g., `"docs/ARCHITECTURE.md"` or `'/path/to/file'`

3. Apply only when:
   - Tool name is `read`
   - `filePath` is missing from validated args
   - A path-like string can be extracted from raw arguments
   - Log the repair: `"[TOOL_ARG_REPAIR] read: injected filePath={} from raw args"`

Implementation location: `src/tool_calling.rs`, inside `execute_tool_call()`, between JSON parse (line 65-104) and schema validation (line 107-127).
