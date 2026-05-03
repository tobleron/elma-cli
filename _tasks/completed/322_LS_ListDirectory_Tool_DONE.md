# Task 322: LS / ListDirectory Tool

**Status:** pending  
**Depends on:** None (can implement in parallel with Task 321)  
**Session trace:** `s_1777380479_751323000` — Elma repeatedly ran `ls -la` at root to find files, never listed subdirectories

## Summary

Add a dedicated `ls` tool to `elma-tools/src/tools/` that lists directory contents as a tree with file metadata (size, type, modification time). Unlike `glob` (pattern-based search), `ls` is for exploring unknown directory structures — "what's in this directory?"

## Why

Elma's only way to list directory contents is `ls -la` via raw shell — fragile, no tree view, no ignore support, no result limits. When the model needs to explore a directory structure (not search for a known filename), there's no safe, structured way to do it. Opencode, Qwen-Code, Roo-Code, and Kolosal-CLI all ship dedicated `ls`/`list_directory` tools.

## Reference Implementations (Best-In-Class Patterns)

### Opencode (`_knowledge_base/_source_code_agents/opencode/internal/llm/tools/ls.go`)
```go
// Parameters: path (string), ignore ([]string — glob patterns)
// Max 1000 files
// Tree-structured output using TreeNode
// Skips: dotfiles, __pycache__, node_modules, dist, build, target, vendor, bin, obj,
//        .git, .idea, .vscode, .DS_Store, *.pyc, *.so, *.dll, *.exe
// Output uses unicode box-drawing chars for tree structure
```

### Qwen-Code (`_knowledge_base/_source_code_agents/qwen-code/packages/core/src/tools/ls.ts`)
```typescript
// Parameters: path (string), ignore? (string[]), file_filtering_options?
// Respects .gitignore and .qwenignore
// Returns FileEntry[] with isDirectory, size, modifiedTime
// Files modified within last 24h sorted first (recency bias)
// Uses ripgrep under the hood for fast traversal
```

### Roo-Code (`_knowledge_base/_source_code_agents/Roo-Code/src/core/tools/`)
- `list_files` tool: `path`, `recursive?` — recursive toggle, simpler than glob
- Not tree-structured, flat list with relative paths

### Kolosal-CLI (`_knowledge_base/_source_code_agents/kolosal-cli/packages/core/src/tools/list_directory.ts`)
- Mirrors Qwen-Code implementation
- Uses `FileFilteringOptions` for fine-grained control

## Rust Crates To Use

- **`ignore`** — Fast traversal respecting `.gitignore` (same as Task 321)
- **`walkdir`** — Fallback traversal
- **`chrono`** or **`time`** — Human-readable timestamps

## Implementation Steps

### Step 1: Create `elma-tools/src/tools/ls.rs`

**Tool definition:**
- Name: `"ls"`
- Description: `"List files and directories in a given path. Shows a tree view with file sizes and modification times. Skips hidden files and common system/generated directories. Use this to explore unknown directory structures or inspect what files exist in a specific location. Max 1000 entries."`
- Parameters:
  - `path` (string, optional) — Directory to list (defaults to workspace root)
  - `depth` (integer, optional) — Maximum recursion depth (default: 2, max: 5)
  - `ignore` (array of strings, optional) — Additional glob patterns to exclude beyond defaults

**Search hints:**
```
"list files in directory",
"show directory contents",
"list directory tree",
"explore directory structure",
"what files are in",
"directory listing",
"show folder contents",
"tree view of directory",
```

### Step 2: Implement the listing logic

**Default ignore patterns** (Opencode + Qwen-Code combined):
```
// Hidden files/dirs
.*
// Build artifacts
target/, build/, dist/, out/, node_modules/, vendor/, bin/, obj/, __pycache__/
// Version control
.git/, .svn/, .hg/
// IDE files
.idea/, .vscode/, .vs/, *.swp, *.swo, *~
// OS files
.DS_Store, Thumbs.db, desktop.ini
// Compiled artifacts
*.pyc, *.pyo, *.so, *.dll, *.dylib, *.exe, *.class, *.o, *.a
// Archives (usually noise)
*.zip, *.tar.gz, *.tgz, *.rar, *.7z
// Large binary formats
*.wasm, *.bin, *.dat
// Elma internal dirs
sessions/, _knowledge_base/, project_tmp/, .cargo/
```

**Output format — tree view with metadata:**
```
/Users/r2/elma-cli/ (12 items, 2 dirs)
├── AGENTS.md (7.0 KB, Apr 27)
├── Cargo.toml (2.1 KB, Apr 27)
├── README.md (749 B, Apr 24)
├── elma.toml (1.5 KB, Apr 27)
├── src/ (34 items, 0 subdirs shown)
│   ├── main.rs (5.2 KB, Apr 27)
│   ├── tool_registry.rs (8.1 KB, Apr 27)
│   ├── session.rs (12.3 KB, Apr 27)
│   └── ... (31 more files)
├── tests/ (8 items)
│   ├── integration_test.rs (3.4 KB, Apr 26)
│   └── ... (7 more files)
├── docs/ (15 items)
│   ├── ARCHITECTURE.md (24.1 KB, Apr 25)
│   └── ... (14 more files)
└── tools/ (3 items)

12 entries shown (4 dirs, 8 files). Total workspace: 847 files in 89 dirs.
```

**Key behaviors:**
1. Directories should show `(N items, M subdirs shown)` after the name
2. When a directory has more children than can be shown (capped by depth or max entries), show `... (K more files)` ellipsis
3. Use Unicode box-drawing characters: `├──`, `└──`, `│`
4. File sizes in human-readable format (KB, MB, GB)
5. Modification times as relative or short dates ("just now", "2 min ago", "Apr 27")
6. Top-level summary line shows total entries shown + workspace totals

### Step 3: Edge case handling

| Edge Case | Behavior |
|-----------|----------|
| Path doesn't exist | Return clear error: `"Directory not found: {path}"` |
| Path is a file | Return file metadata only: `"File: {name} ({size}, modified {date})"` |
| Empty directory | Return: `"{path}/ (empty)"` |
| Permission denied | Skip inaccessible dirs, log warning, show `(permission denied)` note |
| Symlink to dir | Show as directory but mark `→ {target}` |
| Symlink to file | Show as file but mark `→ {target}` |
| >1000 entries at depth 0 | Truncate, show `... and {N} more items` |
| Very deep tree | Cap at `depth` parameter (default 2, max 5) |
| Binary file detection | Don't try to read contents, just show metadata |
| Workspace root is huge | Graceful — show immediate children, note total count |

### Step 4: Register and wire

Same pattern as Task 321: register in `mod.rs`, wire in `tool_calling.rs`.

## Success Criteria

- [ ] `ls` tool registered in `elma-tools` with correct schema
- [ ] Tree-structured output with Unicode box-drawing characters
- [ ] Respects `.gitignore` and default ignore patterns
- [ ] Supports `ignore` parameter for additional exclusions
- [ ] Depth-limited traversal (default 2, max 5)
- [ ] Max 1000 entries with clear truncation message
- [ ] File sizes in human-readable format
- [ ] Modification times in relative/short format
- [ ] Symlinks handled safely (shown, not followed)
- [ ] Empty directories handled gracefully
- [ ] Permission errors logged, traversal continues
- [ ] `cargo build -p elma-tools` succeeds
- [ ] `cargo build` (full workspace) succeeds
- [ ] Unit tests: empty dir, single file, nested dirs, permission-denied dir, symlinks, >1000 entries

## Anti-Patterns To Avoid

- **Do NOT just wrap `ls -la`** — the whole point is structured, safe output
- **Do NOT show dotfiles by default** — they're almost never relevant
- **Do NOT recurse infinitely** — depth cap is mandatory
- **Do NOT show full absolute paths** — relative paths from workspace root
- **Do NOT show raw epoch timestamps** — human-readable dates only
- **Do NOT include `_knowledge_base/` or `sessions/` in default output** — they're Elma internals

## Relationship To Task 321 (Glob)

- **`glob`**: "Find me all files matching this pattern" (known target, pattern-based)
- **`ls`**: "Show me what's in this directory" (unknown target, exploration-based)
- Both tools share the `ignore` crate for `.gitignore` support
- Both should respect the same default ignore patterns
- Small models benefit from having both — `glob` for targeted search, `ls` for exploration
