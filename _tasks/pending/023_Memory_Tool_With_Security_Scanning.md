# Task 163: Memory Tool with Security Scanning

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary

Implement bounded, file-backed memory that persists across sessions. Two stores:
- MEMORY.md: agent's observations, project conventions, tool quirks
- USER.md: user preferences, communication style

With security scanning to prevent prompt injection.

## Motivation

- Remember what was learned across sessions
- Store user preferences
- Prevent malicious memory injection

## Source

Hermes `tools/memory_tool.py`

## Implementation

### Memory Files

```
{hermes_home}/memories/
    MEMORY.md    # Agent's notes
    USER.md     # User preferences
```

### Entry Format

```
§
Entry text here (can be multiline)
§
```

### Tool Interface

```rust
enum MemoryAction {
    Add { content: String },
    Replace { old_substring: String, new_content: String },
    Remove { substring: String },
    Read { filter: Option<String> },
}

fn memory(action: MemoryAction) -> String
```

### Design Principles

1. **Frozen snapshot at session start**: System prompt gets a snapshot
2. **Mid-session writes update disk**: Immediate but don't change system prompt
3. **Next session gets fresh snapshot**: Refresh on new session

### Security Scanning

Pattern matching for injection/exfiltration:

```rust
const MEMORY_THREAT_PATTERNS = [
    // Prompt injection
    (r"ignore\s+(previous|all|above)", "prompt_injection"),
    (r"you\s+are\s+now\s+", "role_hijack"),
    (r"do\s+not\s+tell\s+the\s+user", "deception"),
    
    // Exfiltration
    (r"curl\s+[^\n]*\$(KEY|TOKEN|SECRET)", "exfil_curl"),
    (r"wget\s+[^\n]*\$(KEY|TOKEN|SECRET)", "exfil_wget"),
    (r"cat\s+[^\n]*(\.env|credentials)", "read_secrets"),
    
    // Persistence
    (r"authorized_keys", "ssh_backdoor"),
    (r"\.ssh", "ssh_access"),
]

// Invisible unicode
const INVISIBLE_CHARS = ['\u200b', '\u200c', '\u200d', '\ufeff']
```

### Injection Check

```rust
fn scan_memory_content(content: &str) -> Result<(), String> {
    // Check invisible unicode
    for char in INVISIBLE_CHARS {
        if content.contains(char) {
            return Err("Blocked: invisible unicode");
        }
    }
    
    // Check threat patterns
    for (pattern, _) in MEMORY_THREAT_PATTERNS {
        if content.contains(pattern) {
            return Err(format!("Blocked: pattern '{}'", pattern));
        }
    }
    
    Ok(())
}
```

### File Locking

Use platform-native file locking for concurrent safety:
- Unix: fcntl
- Windows: msvcrt

## Verification

- Memory persists across sessions
- Security patterns blocked
- Mid-session writes don't break prompt cache
- Replace/remove uses substring matching

## Dependencies

- File tools (existing)
- Session management

## Notes

- Character limits (not tokens) - model independent
- Entry delimiter: § (section sign)