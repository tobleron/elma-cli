# Task 327: Write Tool (File Creation/Overwrite)

**Status:** pending  
**Depends on:** None (independent tool, can be implemented standalone)

## Summary

Add a `write` tool to `elma-tools/src/tools/` for creating or overwriting entire files. This complements the `edit` tool — `write` is for new files or complete rewrites, `edit` is for surgical changes. Draws from Opencode's implementation with auto-directory creation, unchanged-content detection, and diff metadata.

## Why

Elma currently has no way to create or overwrite files except via raw shell commands (`echo "..." > file` or `cat > file`). This is error-prone (quoting issues, shell injection risk) and provides no feedback about what changed. Opencode's `write` tool solves this with auto-dir creation, staleness checks, and diff metadata.

## Reference Implementations

### Opencode (`_knowledge_base/_source_code_agents/opencode/internal/llm/tools/write.go`)
```go
// Parameters: file_path (string), content (string)
// Auto-creates parent directories
// Checks for external modifications since last read
// Avoids unnecessary writes when content unchanged
// Returns diff metadata: additions + removals

// Key behaviors:
// - If file exists and content is identical → no-op, reports "No changes needed"
// - If file exists and was modified externally → blocks, requires re-read
// - If parent dirs don't exist → creates them
// - Returns {additions, removals, files_changed}
```

### Claude Code (`_knowledge_base/_source_code_agents/claude-code/tools/FileWriteTool/`)
```typescript
// Similar to FileEditTool but for whole-file writes
// Checks file access permissions
// Tracks file history
// Notifies LSP of changes
```

### Roo-Code (`_knowledge_base/_source_code_agents/Roo-Code/src/core/tools/`)
```typescript
// write_to_file: path, content
// Simple, no staleness checks, no diff metadata
// Used alongside edit, search_and_replace, apply_diff, apply_patch
```

## Implementation Steps

### Step 1: Create `elma-tools/src/tools/write.rs`

**Tool definition:**
- Name: `"write"`
- Description: `"Create or overwrite a file with given content. Auto-creates parent directories. Use this for creating new files or complete rewrites. For surgical changes to existing files, use the 'edit' tool instead. Returns diff metadata showing what was added/removed."`
- Parameters:
  - `file_path` (string, required) — Path to the file (relative to workspace root)
  - `content` (string, required) — The content to write

**Search hints:**
```
"write file",
"create new file",
"overwrite file",
"save file content",
"create file with content",
"generate file",
```

### Step 2: Implement write logic

**Core algorithm:**
```rust
fn write_file(file_path: &str, content: &str, workspace_root: &Path) -> Result<WriteResult> {
    // 1. Resolve path
    let full_path = workspace_root.join(file_path);
    
    // 2. Validate path
    validate_path(&full_path)?;  // No device paths, no symlink escapes, etc.
    
    // 3. Auto-create parent directories
    if let Some(parent) = full_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    // 4. Check if file already exists
    let old_content = if full_path.exists() {
        // 4a. Staleness check (if Task 323 read tracker exists)
        if let Some(last_read) = read_tracker.get_last_read(&full_path) {
            let current_mtime = full_path.metadata()?.modified()?;
            if current_mtime > last_read {
                return Err(WriteError::StaleRead);
            }
        }
        Some(std::fs::read_to_string(&full_path)?)
    } else {
        None
    };
    
    // 5. Check if content is unchanged
    if let Some(ref old) = old_content {
        if old == content {
            return Ok(WriteResult::unchanged(file_path));
        }
    }
    
    // 6. Write atomically
    atomic_write(&full_path, content)?;
    
    // 7. Compute diff metadata
    let (additions, removals) = compute_diff(old_content.as_deref(), content);
    
    Ok(WriteResult {
        files_changed: 1,
        additions,
        removals,
        is_new_file: old_content.is_none(),
    })
}
```

### Step 3: Atomic writes

Use the standard atomic-write pattern:
```rust
fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    // 1. Write to temp file in the same directory
    let temp_path = path.with_extension("tmp.elma-write");
    std::fs::write(&temp_path, content)?;
    
    // 2. Rename temp to target (atomic on same filesystem)
    std::fs::rename(&temp_path, path)?;
    
    Ok(())
}
```

This prevents partial writes if the process crashes mid-write.

### Step 4: Diff Metadata

Compute simple line-based diff:
```rust
struct WriteResult {
    files_changed: u32,
    additions: u32,
    removals: u32,
    is_new_file: bool,
    file_size: u64,
}

fn compute_diff(old: Option<&str>, new: &str) -> (u32, u32) {
    let old_lines: Vec<&str> = old.map(|s| s.lines().collect()).unwrap_or_default();
    let new_lines: Vec<&str> = new.lines().collect();
    
    // Simple line count diff (not full sequence diff)
    let old_count = old_lines.len() as u32;
    let new_count = new_lines.len() as u32;
    
    if new_count > old_count {
        (new_count - old_count, 0)
    } else {
        (0, old_count - new_count)
    }
}
```

Note: For a full diff, defer to Task 328 (Patch Tool) which will use `similar` or `diffy` crate. This is a lightweight approximation.

### Step 5: Output format

**New file created:**
```
✓ Created: src/new_module.rs (156 lines, 4.2 KB)

File content written successfully.
```

**File overwritten:**
```
✓ Updated: src/main.rs
  +12 lines added, -3 lines removed
  File size: 5.2 KB (was 4.8 KB)
```

**No changes needed:**
```
✓ No changes: src/main.rs (content unchanged)

File already contains the desired content. No write needed.
```

**Error — stale read:**
```
✗ Cannot write: src/main.rs
  File has been modified externally since you last read it.
  Please re-read the file first before writing.
```

### Step 6: Edge cases

| Edge Case | Behavior |
|-----------|----------|
| Parent dirs don't exist | Auto-create with `create_dir_all()` |
| File exists, content identical | No-op, returns "content unchanged" |
| File exists, modified since last read | Block (stale read) |
| File path is a directory | Error: "Cannot write: path is a directory" |
| File path contains `..` | Error: "Cannot write outside workspace" |
| Content is empty string | Allow (creates empty file) — some workflows need this |
| File size > 1 GiB | Error: "File too large" |
| Permission denied | Clear OS error message |
| Disk full | Clear OS error message |
| Device/special file | Block via `validate_path()` |

### Step 7: Register and wire

Same pattern as Tasks 321-322: register in `mod.rs`, wire in `tool_calling.rs`.

## Success Criteria

- [ ] `write` tool registered in `elma-tools` with correct schema
- [ ] Auto-creates parent directories
- [ ] Atomic writes via temp-file + rename
- [ ] Unchanged-content detection (no-op when content identical)
- [ ] Stale-write detection (blocks if file changed since last read)
- [ ] Diff metadata: additions, removals, file size
- [ ] Clear output format for all states (created, updated, unchanged, error)
- [ ] Path validation (no escapes, no device files, no dirs)
- [ ] `cargo build -p elma-tools` succeeds
- [ ] `cargo build` (full workspace) succeeds
- [ ] Unit tests: create new file, overwrite existing, unchanged content, stale read, missing parent dirs, invalid path, empty content

## Anti-Patterns To Avoid

- **Do NOT use shell redirection** — that's what we're replacing
- **Do NOT truncate content silently** — if content is truncated, report it
- **Do NOT write without parent directory creation** — `create_dir_all()` is mandatory
- **Do NOT allow writing outside workspace** — `..` path traversal blocked
- **Do NOT add append mode** — use `edit` tool for appending; `write` is create/overwrite only
