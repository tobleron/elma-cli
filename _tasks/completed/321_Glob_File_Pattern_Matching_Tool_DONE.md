# Task 321: Glob File Pattern Matching Tool

**Status:** pending  
**Depends on:** None  
**Session trace:** `s_1777380479_751323000` — Elma took 14/15 iterations to find a file by name using raw shell commands

## Summary

Add a dedicated `glob` tool to `elma-tools/src/tools/` that finds files by name/pattern. This is the #1 missing tool — the `s_1777380479_751323000` session demonstrates that without it, Elma falls into stagnation loops running raw `find` commands with `head -N` truncation that masks results behind noise directories.

## Why

Elma's current tool set (`search` for content, `shell` for raw commands) has no way to find files by name except constructing shell `find` commands — error-prone for small models. Every major CLI agent (Opencode, Qwen-Code, Kolosal-CLI, Roo-Code) ships a `glob` or `list_files` tool. This is table-stakes functionality.

## Reference Implementations (Best-In-Class Patterns)

### Opencode (`_knowledge_base/_source_code_agents/opencode/internal/llm/tools/glob.go`)
```go
// Parameters: pattern (string), path (string, optional)
// Uses: ripgrep for speed, falls back to doublestar glob library
// Hidden file skipping via fileutil.SkipHidden()
// Max 100 results, sorted by shortest path first
// Response metadata: number_of_files + truncated flag
```

### Qwen-Code (`_knowledge_base/_source_code_agents/qwen-code/packages/core/src/tools/glob.ts`)
```typescript
// Parameters: pattern (string), path? (string)
// Results sorted by modification time (files modified <24h appear first for recency bias)
// Uses fast-glob library under the hood
// Respects .gitignore and .qwenignore
```

### Roo-Code (`_knowledge_base/_source_code_agents/Roo-Code/src/core/tools/`)
- `list_files` tool: `path`, `recursive?` — simpler than glob but covers the filename-listing use case
- `search_files` tool: `path`, `regex`, `file_pattern?` — ripgrep-based content search with glob filtering

### Kolosal-CLI (`_knowledge_base/_source_code_agents/kolosal-cli/packages/core/src/tools/glob.ts`)
- Mirrors Qwen-Code implementation
- Also has `read_many_files` — batch read via glob patterns (compound tool)

## Rust Crates To Use

From `_knowledge_base/_source_code_agents/_rust_crates.md`:
- **`glob`** — Library for matching file paths against glob patterns (supports `**`, `*`, `?`, `[]`, `{}`)
- **`globset`** — Optimized library for matching multiple glob patterns simultaneously (useful for ignore patterns)
- **`ignore`** — Fast directory traversal that respects `.gitignore` and other ignore files (walkdir + gitignore in one)
- **`walkdir`** — Library for recursively traversing directory structures (fallback when ripgrep not available)

Prefer **`ignore`** crate for the core traversal since it respects `.gitignore` natively and is the same library ripgrep uses internally. Fall back to `walkdir` + `glob` crate if `ignore` is unavailable.

## Implementation Steps

### Step 1: Add crates to `elma-tools/Cargo.toml`
```toml
ignore = "0.4"
glob = "0.3"
walkdir = "2"
```

### Step 2: Create `elma-tools/src/tools/glob.rs`
Register the tool in the registry with:

**Tool definition:**
- Name: `"glob"`
- Description: `"Find files matching a glob pattern. Use for filename-based search (e.g., '**/*.rs', 'src/**/mod.rs', '*.toml'). Returns relative file paths sorted by modification time. Respects .gitignore. Max 100 results."`
- Parameters:
  - `pattern` (string, required) — The glob pattern to match (e.g., `**/*.rs`, `src/**/*.toml`, `*GEMINI*`)
  - `path` (string, optional) — Directory to search in (defaults to workspace root)

**Search hints:**
```
"find files by name",
"search filename pattern",
"list files matching pattern",
"glob search files",
"find file by name",
"locate file",
"file pattern matching",
```

### Step 3: Implement the glob execution logic

```rust
use ignore::WalkBuilder;
use std::path::PathBuf;
use std::time::SystemTime;

struct GlobResult {
    files: Vec<FileEntry>,
    truncated: bool,
    total_matches: usize,
}

struct FileEntry {
    path: String,          // Relative path from workspace root
    size: u64,             // File size in bytes
    modified: SystemTime,  // Modification timestamp
    is_dir: bool,
}
```

**Core algorithm:**
1. Resolve `path` parameter to absolute workspace path. Default to workspace root.
2. Build a `WalkBuilder` that:
   - Respects `.gitignore` (standard + any `.elmaignore`)
   - Skips hidden files/dirs by default (unless pattern explicitly starts with `.`)
   - Skips common noise dirs: `target/`, `node_modules/`, `.git/`, `_knowledge_base/`, `sessions/`
   - Follows symlinks? NO — too risky, could loop
3. Compile the user's pattern with the `glob` crate
4. Walk the directory tree, matching each entry against the pattern
5. Sort results by modification time (newest first — recency bias from Qwen-Code)
6. Truncate to 100 results max (Opencode pattern)
7. Return metadata: `number_of_files`, `truncated` flag

**Edge cases to handle:**
- **Empty results**: Return `"No files found matching pattern '{pattern}'"` — NOT an error
- **Pattern is a literal filename** (no glob characters): Auto-wrap in `**/{pattern}` to find it anywhere
- **Pattern starts with `/`**: Interpret as relative to workspace root (strip the leading `/`)
- **Pattern matches only directories**: Include them, mark `is_dir: true`
- **Workspace root does not exist**: Return clear error
- **Max depth**: Default unlimited, but consider a configurable `max_depth` for performance
- **Symlinks**: Do NOT follow; log a warning if encountered
- **Permission errors**: Skip inaccessible directories, log a warning, continue traversal
- **Very large workspaces**: After 2 seconds of traversal, return partial results with `truncated: true`
- **Case sensitivity**: Case-sensitive on Linux, case-insensitive on macOS/Windows (match OS behavior)

**Output format:**
```
Found 3 files matching '**/GEMINI.md':
  project_tmp/GEMINI.md (12.5 KB, modified Apr 6 14:11)
  docs/archive/GEMINI.md (8.2 KB, modified Mar 15 09:30)
  _knowledge_base/GEMINI.md (3.1 KB, modified Feb 1 12:00)

3 files found (search completed in 0.3s)
```

If truncated:
```
Found 100+ files matching '**/*.rs' (showing first 100, sorted by most recently modified):
  src/main.rs (15.3 KB, modified just now)
  src/tool_registry.rs (8.1 KB, modified 2 min ago)
  ...

100 files shown (180 total matches, search truncated after 2.0s)
```

### Step 4: Register in `elma-tools/src/tools/mod.rs`
Add `mod glob;` and `glob::register(builder);`

### Step 5: Wire in `src/tool_calling.rs`
Add `"glob" => exec_glob(&args_value, workdir, &call_id, tui)` to the match arm.

### Step 6: Add prerequisite check
Use `which` crate (already available from Task 317) to verify ripgrep OR `ignore` crate availability. The tool should always be available since it uses pure Rust libraries — no external binary needed.

## Success Criteria

- [ ] `glob` tool registered in `elma-tools` with correct schema
- [ ] Pattern matching works for `**/*.rs`, `*.toml`, `**/GEMINI.md`, `src/**/mod.rs`
- [ ] Results sorted by modification time (newest first)
- [ ] Truncated at 100 results with `truncated` metadata
- [ ] Respects `.gitignore`
- [ ] Skips hidden files by default
- [ ] Empty results return graceful message (not error)
- [ ] Literal filenames auto-wrapped in `**/` pattern
- [ ] Symlinks not followed
- [ ] Permission errors handled gracefully
- [ ] `cargo build -p elma-tools` succeeds
- [ ] `cargo build` (full workspace) succeeds
- [ ] Unit tests: empty workspace, pattern with no matches, pattern with >100 matches, hidden files, .gitignore respect
- [ ] Integration test: find `project_tmp/GEMINI.md` in one call

## Anti-Patterns To Avoid

- **Do NOT use `find` via shell** — this is the whole reason we need the tool
- **Do NOT sort alphabetically** — recency bias is critical for developer workflow (Qwen-Code pattern)
- **Do NOT return absolute paths** — relative paths are more readable and portable
- **Do NOT silently skip errors** — log warnings but keep going; the model needs to know about permission issues
- **Do NOT follow symlinks** — infinite loop risk
- **Do NOT hardcode path limits** — use time-based timeout instead (2s), since path depth varies wildly

## Why This Task Is #1 Priority

The `s_1777380479_751323000` session proves this is the most impactful missing tool:
- 14/15 iterations burned on a trivial file-location task
- `head -20`/`head -30` truncation masked results behind _knowledge_base noise
- Same command (`ls -la *.md`) repeated 4 times — stagnation loop
- Model concluded "file doesn't exist" multiple times despite it being in `project_tmp/`

A single `glob` call with `pattern: "**/GEMINI.md"` would have returned the answer in <1 second, in 1 iteration, with zero ambiguity. This is a 14x efficiency improvement on the simplest possible file-location task.
