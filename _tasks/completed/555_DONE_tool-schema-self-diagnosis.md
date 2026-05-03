# Task 555: Tool Schema Self-Diagnosis in Error Feedback

## Session Evidence
In session `s_1777820401_246730000`, the model called the `read` tool without the required `filePath` parameter TWICE in a row. The error message fed back was:
```
Argument validation failed: path: required field 'path' is missing
```
This is confusing because the actual parameter name is `filePath`, not `path`. The model never recovered and the entire task failed.

## Problem
When tool arg validation fails, the error message reported back to the model:
1. May use an internal field name that differs from the user-facing parameter name
2. Doesn't include the expected parameter schema (types, which are required)
3. Doesn't include an example of valid usage
4. The model interprets "validation failed" as a temporary error and retries with the same (wrong) args

## Solution
1. `ToolArgSchema` must store a `help_text` for each parameter and a `usage_example` for the tool
2. When validation fails, the error injected into the conversation context must include:
   - All required parameter names, types, and descriptions
   - A valid example call
   - The exact mismatch that occurred
3. Format: `Tool 'read' failed: missing required parameter 'filePath' (string). Required params: filePath (string, path to file), offset (number, optional), limit (number, optional). Example: {"filePath": "/path/to/file.md", "limit": 500}`
