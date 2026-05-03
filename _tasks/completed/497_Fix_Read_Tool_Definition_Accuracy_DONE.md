# Task 497: Fix Read Tool Definition Description To Match Actual Behavior

**Status:** pending
**Priority:** MEDIUM
**Source:** Session s_1777735825_94786000 deep trace analysis (2026-05-02)

## Evidence From Session

The `read` tool definition (`elma-tools/src/tools/read.rs:11`) states:
```json
"path": {"type": "string", "description": "Absolute or workspace-relative path to the file to read"}
```

But the executor (`src/tool_calling.rs:584-596`) rejects absolute paths:
```rust
} else if std::path::Path::new(&path).is_absolute() {
    let error_msg = format!("absolute_path_not_allowed: {} — use workspace-relative path", path);
    return ToolExecutionResult { ... ok: false ... };
}
```

This mismatch caused the model to try absolute paths (visible in thinking artifact 0012: "The paths need to be absolute or relative to workspace root. Let me use the correct format."), leading to confusion and wasted tool calls.

## Problem

The tool definition promises a capability (absolute paths) that the executor expressly blocks. This creates a semantic contract violation. Small models are particularly vulnerable to this because they faithfully follow the tool definition description.

## Fix

Update the tool definition description to accurately reflect what the executor accepts:
- `"path"` description: Change from "Absolute or workspace-relative path" to "Workspace-relative path to the file to read"
- `"paths"` description: Add "Workspace-relative paths"

Also audit other tool definitions for similar mismatches between definition and executor behavior.

## Implementation Plan

1. Update `elma-tools/src/tools/read.rs` line 11:
   ```
   "path": {"type": "string", "description": "Workspace-relative path to the file to read"}
   ```

2. Audit all tool definitions for similar description mismatches:
   - `write`: check if it accepts absolute paths
   - `edit`: check if it accepts absolute paths  
   - `glob`: check path description
   - `stat`, `copy`, `move`, `mkdir`, `trash`, `touch`, `file_size`, `exists`: check path descriptions

3. For any tool that rejects absolute paths, update the description

## Success Criteria

- [ ] Read tool definition says "Workspace-relative path" not "Absolute or workspace-relative path"
- [ ] All other tools with path parameters have accurate descriptions
- [ ] Models no longer try absolute paths for read/write/edit
