# Task 323: Enhanced Read/View Tool + Global Stagnation Detection

**Status:** pending  
**Depends on:** None (independent improvements to existing `read` tool + global stagnation tracker)
**Merged from:** Task 330 (Stagnation Detection Consecutive Loop Guard) — read dedup already covered here; global stagnation tracker added as Step 6B-6D

## Summary

Enhance the existing `read` tool in `elma-tools/src/tools/read.rs` with production-grade features drawn from Opencode, Hermes Agent, and Claude Code: read deduplication, consecutive-loop detection, image detection, binary file guards, LSP diagnostics, large-file hints, multi-encoding support, and tracker caps to prevent memory bloat.

## Why

The current `read` tool is bare-bones: read a file at offset/limit. It has no protection against re-reading the same file, no image detection (model gets garbage bytes), no encoding handling, no LSP integration, and no defense against the model looping on `read` calls. Every major CLI agent has hardened this tool extensively — we should adopt the best patterns.

## Reference Implementations

### Hermes Agent (`_knowledge_base/_source_code_agents/hermes-agent/tools/file_tools.py`)
**READ DEDUP** (lines 82-140):
```python
# Tracks (resolved_path, offset, limit) → mtime
# On re-read with same params + unchanged mtime → returns stub "File unchanged since last read"
# Consecutive-loop detection: 3 identical reads → warning; 4 → hard block with error
# Tracker caps: _READ_HISTORY_CAP=500, _DEDUP_CAP=1000, _READ_TIMESTAMPS_CAP=1000
```

**BINARY FILE GUARD** (lines 45-60):
```python
# Extension-based binary detection: has_binary_extension()
# Blocks reading binary files (images, executables, archives, media)
```

**LARGE-FILE HINT** (lines 65-75):
```python
# For files >512KB with limit>200, hints to use offset+limit
# Character-count guard: file_read_max_chars=100K
```

**DEVICE PATH BLOCKING** (lines 30-44):
```python
# Blocks: /dev/zero, /dev/random, /dev/urandom, /dev/stdin, /dev/tty,
#         /dev/stdout, /dev/stderr, /proc/*/fd/[0-2]
```

**SECRETS REDACTION** (lines 78-82):
```python
# redact_sensitive_text() applied after size guard
# Strips API keys, tokens, passwords from output before sending to model
```

### Claude Code (`_knowledge_base/_source_code_agents/claude-code/tools/FileReadTool/`)
```typescript
// readFileForEdit() — sync read for atomicity, handles UTF-16LE BOM detection
// 1 GiB file size limit
// WeakMap-based dedup: tracks last read position per file
// Image detection: identifies images and returns metadata instead of raw bytes
// Text extraction: handles PDF, DOCX, XLSX via external converters
```

### Opencode (`_knowledge_base/_source_code_agents/opencode/internal/llm/tools/view.go`)
```go
// Parameters: file_path (string), offset (int, 0-based), limit (int, default 2000)
// Max file size: 250KB; max line length: 2000 chars
// Line numbers: 6-char padded format "     1|content"
// File-not-found: suggests similar filenames (up to 3) via substring matching
// Image detection: JPEG, PNG, GIF, BMP, SVG, WebP by extension
// LSP diagnostics attached to output
```

## Implementation Steps

### Step 1: Add image detection

Use extension-based detection (no external crate needed for MVP):

```rust
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "bmp", "svg", "webp", "ico", "tiff", "tif",
    "avif", "heic", "heif",
];

const BINARY_EXTENSIONS: &[&str] = &[
    // Executables
    "exe", "dll", "so", "dylib", "bin", "app",
    // Archives
    "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "zst",
    // Media
    "mp3", "mp4", "wav", "ogg", "flac", "avi", "mov", "mkv", "webm",
    // Compiled
    "class", "o", "a", "wasm", "pyc", "pyo",
    // Documents (binary)
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    // Database
    "db", "sqlite", "sqlite3",
    // Fonts
    "ttf", "otf", "woff", "woff2",
    // Other
    "DS_Store",
];

const DEVICE_PATHS: &[&str] = &[
    "/dev/zero", "/dev/random", "/dev/urandom",
    "/dev/stdin", "/dev/stdout", "/dev/stderr",
    "/dev/tty", "/dev/null",
];
```

**Behavior:**
- If image extension → return: `"[Image: {filename} ({size}, {dimensions if detectable})]"` — no raw bytes
- If binary extension → return: `"[Binary file: {filename} ({size})]"` with hint "Use shell tools to inspect if needed"
- If device path → block with error: `"Cannot read device file: {path}"`
- Else → read as text as normal

### Step 2: Implement read deduplication

**Data structure:**
```rust
use std::collections::HashMap;
use std::time::SystemTime;

struct ReadTracker {
    // Key: (canonical_path, offset, limit), Value: (mtime, timestamp_of_read)
    dedup_map: HashMap<(PathBuf, u64, u64), (SystemTime, SystemTime)>,
    // Tracks consecutive identical reads for loop detection
    consecutive_reads: Vec<(PathBuf, u64, u64)>,
    // Caps to prevent memory bloat (Hermes Agent pattern)
    dedup_cap: usize,       // 1000
    consecutive_cap: usize,  // 500
    history_cap: usize,      // 500
}
```

**Dedup logic:**
1. On every `read` call, canonicalize the path
2. Check if `(path, offset, limit)` exists in `dedup_map`
3. If exists AND file mtime hasn't changed → return stub:
   ```
   "[File unchanged since last read: {filename} (last read {X} seconds ago)]"
   ```
4. If exists BUT mtime changed → update entry, re-read normally
5. If not exists → read normally, add to map

**Consecutive-loop detection:**
1. Track the last N reads in `consecutive_reads` vector
2. If the last 3 reads are identical `(path, offset, limit)` → emit warning in output:
   ```
   "⚠️ You've read this file section 3 times consecutively. Consider a different approach."
   ```
3. If the last 4 reads are identical → HARD BLOCK:
   ```
   "🛑 Stopping: You've read this file section 4 times consecutively without progress. 
   Please finalize your answer or try a different approach."
   ```
4. Reset the counter when ANY other tool is used (requires cross-tool notification, see Step 6)

**Tracker caps:**
- `dedup_map` capped at 1000 entries — evict oldest on overflow
- `consecutive_reads` capped at 500 entries
- Use `lru` crate for automatic eviction policy

### Step 3: Add file-not-found suggestions (Opencode pattern)

When a file is not found:
1. Extract the filename from the path
2. Search the workspace for files with similar names (substring match, max 3)
3. Return:
   ```
   "File not found: src/tool_registry.rs
   
   Did you mean:
     src/tool_registry.rs → (not found)
   Similar files:
     src/tool_registry2.rs
     src/tools/registry.rs
     elma-tools/src/registry.rs"
   ```

Implementation: Walk the workspace root with `ignore` crate, collect all relative paths, find substring matches. Cache the path list for 30 seconds to avoid re-walking.

### Step 4: Add large-file hints and size limits

- **Max file size**: 250KB (Opencode's limit — prevents context window pollution)
- **Max line length**: 2000 chars (truncate lines longer than this, add `[truncated]` marker)
- **Large-file hint**: For files >50KB with limit >200 lines, append:
  ```
  "💡 Tip: This file is large ({size}). Use offset+limit to read specific sections."
  ```

### Step 5: Improve output formatting

- **Line numbers**: 6-char right-padded format (Opencode pattern):
  ```
       1|use std::path::PathBuf;
       2|
       3|fn main() {
  ```
- **Truncation notice**: When output is truncated due to limit:
  ```
  "... [Showing lines {offset}-{offset+limit} of {total_lines} total lines]"
  ```
- **Image metadata**: When image detected, try to get dimensions:
  ```
  "[Image: logo.png (24 KB, 512x512)]" 
  ```
  Use the `image` crate from `_rust_crates.md` for metadata extraction (lazy dependency — only if crate is available).

### Step 6: Cross-tool loop reset

When any other tool (shell, glob, search, edit, write, ls) is executed, reset the `consecutive_reads` counter. This follows Hermes Agent's `notify_other_tool_call()` pattern.

Implementation approach: The `ReadTracker` should be stored in a global `OnceLock<Mutex<ReadTracker>>` or passed via the session state. When other tools execute, they call `ReadTracker::reset_consecutive()`.

### Step 7: LSP diagnostics (future, optional)

Opencode attaches LSP diagnostics to file read output. For now, skip this — it requires LSP client infrastructure. Add a comment in the code marking where LSP diagnostics would be appended.

### Step 8: Multi-encoding support

Detect and handle common encodings:
- UTF-8 (default)
- UTF-16 LE/BE with BOM (Claude Code pattern)
- Latin-1 (ISO-8859-1) as fallback

Use the **`encoding_rs`** crate from `_rust_crates.md`:
```rust
use encoding_rs::Encoding;
// Detect BOM, then try UTF-8, fall back to windows-1252
```

## Success Criteria

- [ ] Image files return metadata instead of raw bytes
- [ ] Binary files blocked with clear message
- [ ] Device paths blocked with error
- [ ] Read dedup: identical re-read returns stub
- [ ] Consecutive-loop detection: 3→warn, 4→block
- [ ] Tracker caps prevent memory bloat (LRU eviction)
- [ ] File-not-found suggests similar filenames
- [ ] Large files show hint (>50KB + limit>200)
- [ ] Max file size enforced (250KB)
- [ ] Max line length enforced (2000 chars)
- [ ] Line numbers in 6-char padded format
- [ ] Multi-encoding: UTF-8, UTF-16 LE/BE with BOM
- [ ] Cross-tool loop reset works (other tools reset counter)
- [ ] `cargo build -p elma-tools` succeeds
- [ ] `cargo build` (full workspace) succeeds
- [ ] Unit tests: read text, read image, read binary, re-read same, 3-consecutive warn, 4-consecutive block, file not found, encoding detection

## Part B: Global Stagnation Tracker (merged from Task 330)

### Step 6B: Create a global `StagnationTracker` for all tools

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{SystemTime, Duration};

struct ToolCallRecord {
    tool_name: String,
    params_hash: u64,      // Hash of the tool parameters
    output_hash: u64,      // Hash of the tool output
    timestamp: SystemTime,
}

struct StagnationTracker {
    // History of all tool calls this turn
    call_history: Vec<ToolCallRecord>,
    // Consecutive identical calls (same tool + same params)
    consecutive_count: u32,
    last_params_hash: Option<u64>,
    // Consecutive identical outputs (different params, same result)
    same_output_count: u32,
    last_output_hash: Option<u64>,
    // Caps
    max_call_history: usize,  // 200
}
```

### Step 6C: Implement multi-level stagnation detection

| Detection Level | Action | Message to Model |
|----------------|--------|------------------|
| 2nd identical call | Soft warning | `"⚠️ You've run '{command}' twice with the same result. Consider a different approach."` |
| 3rd identical call | Strong warning | `"⚠️ You've run '{command}' 3 times with identical results."` |
| 4th identical call | HARD BLOCK | `"🛑 Blocked: repeated command 4 times. Finalize your answer with the evidence you have, or try a fundamentally different approach."` |
| Same output 4+ times | Cycle warning | `"⚠️ You've received the same output 4 times across different commands."` |
| A→B→A cycle detected | Force finalization | Message about the cycle |

### Step 6D: Output-identity hashing and cross-tool awareness

```rust
fn hash_output(output: &str) -> u64 {
    // Normalize: strip ANSI escapes, trim trailing whitespace per line, remove empty trailing lines
    let stripped = strip_ansi_escapes::strip(output);
    let text = String::from_utf8_lossy(&stripped);
    let normalized = text.lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    // Hash normalized content
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    normalized.hash(&mut hasher);
    hasher.finish()
}
```

When ANY tool produces a truly different result, reset stagnation counters. Read tool uses its own tracker (Step 2); global tracker covers shell, search, glob, edit, write, and patch tools. Respond tool never triggers stagnation detection.

### Step 6E: Integration with existing stop policy

Modify `src/stop_policy.rs`:
1. On every tool execution, call `StagnationTracker::notify_tool_result()`
2. SoftWarning → inject hint into next model turn
3. StrongWarning → inject stronger hint + consider reducing remaining iterations
4. HardBlock → force finalization immediately
5. CycleDetected → force finalization with cycle message
6. Reset all counters at the start of each new user turn
7. Do NOT persist across sessions — tracker is per-turn only

## Anti-Patterns To Avoid

- **Do NOT read the entire file into memory for size check** — use `fs::metadata()` first
- **Do NOT leak read history across sessions** — tracker is per-session only
- **Do NOT block legitimate re-reads** — only block when mtime is unchanged AND params identical
- **Do NOT add image processing** — just detect and describe; no OCR, no EXIF parsing
- **Do NOT add secrets redaction yet** — defer to a dedicated security task
- **Do NOT block after 1 repetition** — some tasks legitimately need retries
- **Do NOT compare raw output strings** — normalize first (ANSI, whitespace, trailing newlines)
- **Do NOT apply stagnation to respond/finalization attempts**
- **Do NOT track across user turns** — each turn is a fresh start
- **Do NOT block all shell commands after stagnation** — only block identical commands with identical outputs
