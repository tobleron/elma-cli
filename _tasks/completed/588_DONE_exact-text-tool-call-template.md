# Task 588: Exact-Text Tool-Call Template Injection After Read Failure

## Session Evidence
Session `s_1777822834_658323000`: The corrective feedback `format_error_with_schema()` now shows the correct schema — `"Tool 'read' expects: filePath (path (string)) [required], offset..."` — but the model STILL produces empty args. The schema narrative is descriptive but not prescriptive enough for a 4B model.

## Problem
Schema descriptions tell the model WHAT to produce but not HOW. A 4B model needs exact, copyable templates. The current error message format `"Tool 'read' expects: filePath..."` is semantically equivalent to the original error — it describes the problem without providing a fix the model can directly apply.

## Solution
When `read` validation fails AND the model has evidence showing specific file paths (from `ls` output), inject an exact copyable template:

```
System message: "Copy this exact JSON for your next tool call:
read arguments={"filePath": "docs/ARCHITECTURE.md"}
```

The key difference from the current approach:
- Current: "Tool 'read' expects: filePath (path (string)) [required], offset (non-negative-integer) [optional]"
- New: Inject ONE specific, ready-to-call example using a path from recently gathered evidence
- Include the file path extracted from the `ls` output shown in the same session

This bridges the gap between "the model knows which files exist" and "the model can produce a valid read call."
