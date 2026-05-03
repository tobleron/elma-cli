# Task 559: Tool Parameter Schema Propagation to Model

## Session Evidence
In session `s_1777820401_246730000`, the model called the `read` tool with no arguments:
```
Tool: read
Arguments: {}  (or equivalent missing filePath)
```
This happened twice. The model either:
1. Didn't know `read` requires `filePath`
2. Knew it needs a path but wasn't told the parameter name is `filePath` (not `path`)
3. Received the schema but the 4B model couldn't reliably produce correct JSON args

## Problem
The tool schema sent to the model in the system prompt may not include complete parameter definitions, or the definitions are embedded in prose that the 4B model can't parse reliably. The model needs explicit, unabbreviated parameter info.

## Solution
1. Audit the tool description in the system prompt for each tool — ensure parameter names, types, and required/optional status are explicit
2. Add a compact tool parameter table to each tool's description: `Params: filePath (string, required), offset (number, optional), limit (number, optional)`
3. For the `read` tool specifically, add: "ALWAYS provide the filePath parameter. If you're unsure of the path, list the directory first."
4. Add a lightweight schema validation on the MODEL side — if the model is about to call `read` without `filePath`, inject a corrective prompt BEFORE attempting tool execution
5. Consider adding a pre-execution guard that checks: if `read` was called and `filePath` is missing, inject a one-shot prompt "You called read without specifying which file to read. Which file do you want to read?" and re-prompt the model, saving the iteration
