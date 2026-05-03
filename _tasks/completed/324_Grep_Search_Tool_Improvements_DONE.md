# Task 324: Grep/Search Tool Improvements

**Status:** pending  
**Depends on:** None (independent improvements to existing `search` tool)

## Summary

Enhance the existing `search` (ripgrep) tool with literal text mode, 3-tier fallback chain, result grouping by file, truncation-aware output, and `include` parameter for file-type filtering — all drawn from Opencode and Qwen-Code's battle-tested implementations.

## Why

The current `search` tool is a thin wrapper around `rg`. It lacks literal text mode (small models struggle with regex escaping), has no fallback when `rg` is unavailable, doesn't group results by file, and offers no file-type filtering. These are solved problems in Opencode and Qwen-Code.

## Reference Implementations

### Opencode (`_knowledge_base/_source_code_agents/opencode/internal/llm/tools/grep.go`)
```go
// Parameters: pattern, path?, include?, literal_text? (boolean)
// literal_text=true → auto-escapes all regex special chars with regexp.QuoteMeta()
// Uses: rg -H -n (with --line-number --no-heading --color=never)
// Fallback: Go's filepath.Walk + regexp with 200-match limit
// Results sorted by modification time (newest first), truncated to 100
// Results grouped by file with line numbers
```

### Qwen-Code (`_knowledge_base/_source_code_agents/qwen-code/packages/core/src/tools/grep.ts`)
```typescript
// Parameters: pattern, path?, glob?, limit?
// 3-tier fallback: git grep → system grep → pure JS fallback
// glob parameter for file-type filtering (e.g., "*.rs", "*.toml")
// limit parameter controls max results
```

### Claude Code (`_knowledge_base/_source_code_agents/claude-code/tools/GrepTool/`)
```typescript
// Uses ripgrep exclusively (assumes it's always available)
// Handles large result sets with truncation + "X more matches" message
// Respects .gitignore by default
// -n (line numbers), -H (filename), --no-heading flags standard
```

## Implementation Steps

### Step 1: Add `literal_text` parameter

When `literal_text: true`, escape all regex special characters before passing to ripgrep:

```rust
fn escape_literal_pattern(pattern: &str) -> String {
    // Escape: . * + ? ^ $ { } [ ] ( ) | \
    let special_chars = r".*+?^${}[]()|\#";
    let mut escaped = String::with_capacity(pattern.len());
    for ch in pattern.chars() {
        if special_chars.contains(ch) {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}
```

This prevents small models from generating broken regex patterns. A user asking "find file GEMINI.md" should not fail because `.` in the regex matches any character.

### Step 2: Add `include` parameter for file-type filtering

```json
{
  "include": {
    "type": "string",
    "description": "File pattern to filter by (e.g., '*.rs', '*.{ts,tsx}', '*.toml'). Passed to ripgrep as --glob."
  }
}
```

This maps to `rg --glob=<include>`. Multiple patterns? Either accept comma-separated or only one `include` per call (keep it simple — one pattern per call, use shell for complex filtering).

### Step 3: Implement 3-tier fallback chain

**Tier 1: `rg` (ripgrep)** — always preferred, fastest, respects .gitignore natively

```bash
rg -H -n --no-heading --color=never [--glob=<include>] <pattern> [path]
```

**Tier 2: `grep -r`** — fallback if `rg` not installed

```bash
grep -rn --color=never [--include=<include_pattern>] <pattern> [path]
```

**Tier 3: Pure Rust fallback** — if neither binary exists

```rust
use ignore::WalkBuilder;
use regex::Regex;

// Walk the directory, read each file, search with regex
// Limit to 200 total matches across all files
// Skip binary files
// Respect .gitignore via ignore crate
```

Each tier should log which was used (for debugging) but NOT show to the model.

### Step 4: Improve result formatting

**Current format** (bare rg output):
```
src/main.rs:42:fn main() {
src/main.rs:58:    let args = parse();
src/tool_registry.rs:15:pub fn register() {
```

**New format** (grouped by file, Opencode pattern):
```
Found 15 matches across 3 files:

--- src/main.rs ---
  42|fn main() {
  58|    let args = parse();
  89|    run(args);

--- src/tool_registry.rs ---
  15|pub fn register() {
  32|    tools.insert("read", ...);
  78|    tools.insert("search", ...);

--- src/tool_calling.rs ---
   4|//! Tool Calling Registry
  12|pub(crate) fn execute_tool_call(
  ... (9 more matches in this file)

15 matches across 3 files (search took 0.3s)
```

**Key formatting rules:**
- Line numbers right-padded to 4 chars
- Pipe separator `|` between line number and content
- Ellipsis `...` when a file has more matches than can be shown
- Summary line at bottom: total matches, file count, search time
- Files sorted by modification time (newest first — recency bias)

### Step 5: Max results and truncation

- **Max total results**: 100 (Opencode limit)
- **Max results per file**: 50
- **Truncation message**: `"... (and {N} more matches in {M} more files)"`
- **Empty results**: `"No matches found for '{pattern}' in {path}"` (not an error)
- **Pattern too short**: If pattern < 2 chars, warn: `"Pattern too short — may produce many results. Consider a more specific pattern."`
- **Path doesn't exist**: Clear error with suggestion to use `ls` or `glob`

### Step 6: Edge cases

| Edge Case | Behavior |
|-----------|----------|
| Pattern with only whitespace | Error: "Pattern cannot be empty or whitespace-only" |
| Binary file encountered | Skip, don't include in results |
| File too large (>1MB) | Skip, log warning: "Skipping large file: {path}" |
| Search path doesn't exist | Error: "Path not found: {path}" |
| 0 results | Graceful empty message with pattern shown |
| >100 results | Truncate with clear count of truncated matches |
| Regex syntax error | Error: "Invalid regex pattern: {error}. Use literal_text=true for plain text search." |
| Permission denied on dir | Skip dir, continue, log warning |

### Step 7: Add prerequisite check improvements

Current `search` tool has `check_fn` for `rg` OR `grep`. Add the pure Rust fallback so the tool is ALWAYS available (even if no external binary exists). Mark `check_fn` to always return `true` — the tool degrades gracefully through the fallback chain.

## Success Criteria

- [ ] `literal_text` parameter auto-escapes regex special chars
- [ ] `include` parameter filters by file glob pattern
- [ ] 3-tier fallback: rg → grep → pure Rust
- [ ] Results grouped by file with per-file section headers
- [ ] Line numbers right-padded to 4 chars
- [ ] Max 100 total results, 50 per file
- [ ] Files sorted by modification time (newest first)
- [ ] Truncation message shows hidden match count
- [ ] Empty results return graceful message (not error)
- [ ] Binary files and large files skipped gracefully
- [ ] Pattern validation (not empty, not whitespace-only)
- [ ] Tool always available (pure Rust fallback ensures this)
- [ ] `cargo build -p elma-tools` succeeds
- [ ] `cargo build` (full workspace) succeeds
- [ ] Unit tests: literal text escaping, regex pattern, empty results, 100+ results, binary skip, invalid regex, grep fallback, pure rust fallback

## Anti-Patterns To Avoid

- **Do NOT run grep on binary files** — check extension before reading
- **Do NOT search hidden dirs by default** — `sessions/`, `.git/`, `target/`, `_knowledge_base/`
- **Do NOT show raw ripgrep output** — always group and format
- **Do NOT search the entire workspace for a 1-char pattern** — warn the model
- **Do NOT use `find | xargs grep`** — that's the old broken pattern. Use `rg` or `ignore` crate.
