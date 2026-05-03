# Task 330: Stagnation Detection — Consecutive Loop Guard & Read Dedup

**Status:** pending  
**Depends on:** Task 323 (needs read tracker for dedup), Task 321 (glob reduces stagnation triggers)

## Summary

Implement proactive stagnation detection that catches loops BEFORE they consume the entire iteration budget. Combine Hermes Agent's consecutive-loop detection (3→warn, 4→block), read deduplication, and cross-tool awareness to prevent the kind of stagnation seen in `s_1777380479_751323000` where the model ran `ls -la *.md` four times with identical results.

## Why

The `s_1777380479_751323000` session demonstrates the current stagnation system's failure modes:
- The model ran `ls -la *.md` 4 times with IDENTICAL results (same two files each time)
- The model ran `find . -maxdepth 1 -name "*.md"` 3 more times (same output)
- Stagnation wasn't detected until iteration 13/15 (`repeated_same_command`)
- By then, only 2 iterations remained — barely found the answer

Hermes Agent detects loops at repetition 3 (warn) and 4 (block). We need this level of granularity, applied across ALL tools, not just shell commands.

## Reference Implementations

### Hermes Agent (`_knowledge_base/_source_code_agents/hermes-agent/tools/file_tools.py`)

**Read dedup** (lines 82-140):
```python
# Tracks (resolved_path, offset, limit) → mtime
# On re-read with same params + unchanged mtime: returns stub
# Consecutive-loop detection: 3 identical reads → warning; 4 → hard block
# notify_other_tool_call() resets counter when any other tool is used
```

**Tracker caps** (lines 50-57):
```python
_READ_HISTORY_CAP = 500
_DEDUP_CAP = 1000
_READ_TIMESTAMPS_CAP = 1000
```

### Elma's current stagnation detection

From `src/tool_loop.rs:733-765` and `src/stop_policy.rs`:
- Detects `repeated_same_command` (exact command match)
- Detects `repeated_same_output` (exact output match)
- Triggers finalization after threshold
- Problem: threshold is too high, detection is per-command-type only, no cross-tool awareness

## Implementation Steps

### Step 1: Create a global `StagnationTracker`

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

### Step 2: Implement multi-level stagnation detection

**Level 1: Exact repetition (same tool + same params + same output)**
```
Iter 4: ls -la *.md → AGENTS.md, README.md
Iter 5: ls -la *.md → AGENTS.md, README.md  ← WARNING (2nd identical)
Iter 6: ls -la *.md → AGENTS.md, README.md  ← CRITICAL (3rd identical)
Iter 7: ls -la *.md → AGENTS.md, README.md  ← BLOCK (4th identical)
```

**Level 2: Same output, different params**
```
Iter 4: find . -maxdepth 1 -name "*.md" → AGENTS.md, README.md
Iter 5: ls -la *.md && find . -maxdepth 1 -name "*.md" → AGENTS.md, README.md
Iter 6: find . -maxdepth 1 -iname "*.md" → AGENTS.md, README.md  ← WARNING
```

**Level 3: Stagnation pattern (repeating cycle)**
```
Iter 4: ls -la *.md → AGENTS.md, README.md
Iter 5: find . -name "*gemini*" → _knowledge_base files
Iter 6: ls -la *.md → AGENTS.md, README.md  ← CYCLE DETECTED
```

**Level 4: Output-identical across different tools**
```
Tool A: rg GEMINI.md → no results
Tool B: find -name "GEMINI.md" → no results
Tool C: ls -la *.md → AGENTS.md, README.md (GEMINI.md not found)
Tool D: grep gemini → _knowledge_base files only
— All fail to find the target, all effectively say "not found at root"
```

### Step 3: Warning and block actions

| Detection Level | Action | Message to Model |
|----------------|--------|------------------|
| 2nd identical call | Soft warning | `"⚠️ You've run '{command}' twice with the same result. Consider a different approach."` |
| 3rd identical call | Strong warning | `"⚠️ You've run '{command}' 3 times with identical results. The file is not at root. Try searching subdirectories or using 'glob'."` |
| 4th identical call | HARD BLOCK | `"🛑 Blocked: repeated command 4 times. Finalize your answer with the evidence you have, or try a fundamentally different approach."` |
| Same output 4+ times | Cycle warning | `"⚠️ You've received the same output 4 times across different commands. Consider whether the information you're looking for exists."` |

### Step 4: Output-identity hashing

Hash tool outputs to detect identical results across different commands:

```rust
fn hash_output(output: &str) -> u64 {
    // Normalize before hashing:
    // 1. Strip ANSI escape codes
    // 2. Strip trailing whitespace from each line
    // 3. Hash the normalized content
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    normalize_output(output).hash(&mut hasher);
    hasher.finish()
}

fn normalize_output(output: &str) -> String {
    // Remove ANSI escapes
    // Trim trailing whitespace per line
    // Remove empty trailing lines
    // This makes "ls -la" and "find . -maxdepth 1" output comparable
    let stripped = strip_ansi_escapes::strip(output);
    let text = String::from_utf8_lossy(&stripped);
    text.lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}
```

### Step 5: Cross-tool awareness

When ANY tool succeeds at a fundamentally different operation, reset stagnation counters:

```rust
fn notify_tool_result(
    tracker: &mut StagnationTracker,
    tool_name: &str,
    params_hash: u64,
    output: &str,
    output_hash: u64,
    success: bool,
) -> StagnationStatus {
    // Record the call
    tracker.call_history.push(ToolCallRecord { ... });
    
    // Check for exact repetition
    if params_hash == tracker.last_params_hash.unwrap_or(0) 
       && output_hash == tracker.last_output_hash.unwrap_or(0) {
        tracker.consecutive_count += 1;
    } else if output_hash == tracker.last_output_hash.unwrap_or(0) {
        tracker.same_output_count += 1;
    } else {
        // Different result → reset counters
        tracker.consecutive_count = 0;
        tracker.same_output_count = 0;
    }
    
    // Check for cycle (A → B → A pattern)
    if let Some(cycle) = detect_cycle(&tracker.call_history) {
        return StagnationStatus::CycleDetected(cycle);
    }
    
    // Evaluate thresholds
    match tracker.consecutive_count {
        0..=1 => StagnationStatus::Normal,
        2 => StagnationStatus::SoftWarning,
        3 => StagnationStatus::StrongWarning,
        _ => StagnationStatus::HardBlock,
    }
}
```

### Step 6: Integration with existing stop policy

Modify `src/stop_policy.rs` to use the new tracker:

1. On every tool execution, call `StagnationTracker::notify_tool_result()`
2. If status is `SoftWarning` → inject a hint message into the next model turn
3. If status is `StrongWarning` → inject a stronger hint + consider reducing remaining iterations
4. If status is `HardBlock` → force finalization immediately (don't wait for iteration limit)
5. If status is `CycleDetected` → force finalization with a message about the cycle

### Step 7: Special cases

**Read tool**: Tracked by Task 323's read tracker (separate system). Cross-notify: when any non-read tool executes, reset read consecutive counter.

**Shell tool**: Most prone to stagnation. Apply all levels.

**Search tool**: Apply output-identity hashing. If `rg PATTERN` returns empty 3 times with different patterns, warn.

**Glob tool**: After Task 321, glob should reduce shell-based file search stagnation. But if glob also returns empty repeatedly, detect it.

**Respond tool**: Never triggers stagnation detection (the model is trying to finish).

**Edit/Write/Patch tools**: If the model makes the same edit 3 times (same old_string, same file, same result), warn. But don't block — the model might be iterating.

### Step 8: Storage and caps

- `call_history`: capped at 200 entries (Hermes Agent pattern)
- Use a ring buffer or `VecDeque` with automatic eviction
- Reset all counters at the start of each new user turn
- Do NOT persist across sessions — tracker is per-turn only

### Step 9: Logging for debugging

Log all stagnation events for post-session analysis:
```
[STAGNATION] soft_warning: tool=shell, cmd="ls -la *.md", count=2, output_hash=0xABCD
[STAGNATION] strong_warning: tool=shell, cmd="ls -la *.md", count=3, output_hash=0xABCD
[STAGNATION] hard_block: tool=shell, cmd="ls -la *.md", count=4
```

## Success Criteria

- [ ] `StagnationTracker` global singleton per session
- [ ] Exact repetition detection: 2→warn, 3→strong warn, 4→block
- [ ] Same-output detection across different commands
- [ ] Cycle detection (A→B→A pattern)
- [ ] Output normalization for hash comparison (strip ANSI, trim whitespace)
- [ ] Cross-tool counter reset when different results appear
- [ ] Hard block forces immediate finalization
- [ ] Tracker caps prevent memory bloat (200 entries)
- [ ] All counters reset on new user turn
- [ ] Stagnation events logged for debugging
- [ ] Integration with existing `stop_policy.rs`
- [ ] `cargo build` succeeds
- [ ] Unit tests: 2x identical, 4x identical, same output different commands, cycle detection, counter reset, shell vs glob differentiation

## Anti-Patterns To Avoid

- **Do NOT block after 1 repetition** — some tasks legitimately need retries (e.g., "try again with different casing")
- **Do NOT compare raw output strings** — normalize first (ANSI, whitespace, trailing newlines)
- **Do NOT apply to respond/finalization attempts** — those are the goal, not stagnation
- **Do NOT track across user turns** — each turn is a fresh start
- **Do NOT block all shell commands after stagnation** — only block identical commands with identical outputs
- **Do NOT use this as a substitute for better tools** — Task 321 (glob) prevents stagnation at the root cause
