# Task 560: Tool Usage Correction via Feed-Forward Recovery

## Session Evidence
In session `s_1777820401_246730000`, the model called `read` wrong twice in a row:
```
Turn 3: read → FAIL (missing filePath)
Turn 4: restart cycle
Turn 5: workspace_info (wasteful)
Turn 6: ls (wasteful)  
Turn 7: read → FAIL (missing filePath) AGAIN
```

The error feedback "Argument validation failed: path: required field 'path' is missing" was injected into the conversation but the model still repeated the error. The correction wasn't actionable.

## Problem
Merely reporting a validation error to the model is insufficient for a 4B model. The model needs explicit corrective guidance, not just an error message. Without it, the model retries the same broken call until budget runs out.

## Solution
Create a `ToolErrorCorrector` that runs after any tool failure:
1. Analyzes the specific error (missing parameter, wrong type, invalid path, etc.)
2. Generates a corrective prompt that includes the correct usage
3. Injects it into the conversation BEFORE the next model call
4. Example output: `Read (error): You tried to call read without the required 'filePath' parameter. For your next call use: read filePath="docs/ARCHITECTURE.md". Available docs include: ARCHITECTURE.md, DEVELOPMENT.md, CONFIGURATION.md...`

This is different from Task 559 (schema propagation) — that fixes what the model SEES initially. This fixes what happens AFTER a failed call, using the failure context + available evidence to guide the next attempt.
