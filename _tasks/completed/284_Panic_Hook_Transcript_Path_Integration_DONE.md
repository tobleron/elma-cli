# Task 284: Panic Hook Transcript Path Integration

**Status**: Pending  
**Priority**: Medium  
**Depends on**: T281 (transcript persistence), T282 (session index)  
**Elma Philosophy**: Crash diagnostics, preserve evidence, enable debugging

## Goal

When Elma crashes (panics), include the path to the session's `display/terminal_transcript.txt` and artifact directory in the error report.

This allows the agent coder (or user) to quickly inspect:
- What the user asked
- What the assistant was reasoning about
- Which tool caused the crash
- Full tool outputs and shell transcripts

## Requirements

### Part 1: Update `install_panic_hook()`

In `src/session_error.rs`:

```rust
pub(crate) fn install_panic_hook(session_root: Option<PathBuf>) {
    // ... existing code ...
    
    if let Some(ref root) = session_root {
        // NEW: Compute paths to transcript and artifacts for easy access
        let transcript_path = root.join("display").join("terminal_transcript.txt");
        let artifacts_path = root.join("artifacts");
        
        let error = SessionError::panic("runtime", &full_message, None);
        if let Ok(path) = write_session_error(root, &error) {
            eprintln!("   Error report: {}", path.display());
        }
        
        // NEW: Log transcript and artifact paths for quick recovery
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&trace_path)
        {
            use std::io::Write;
            let _ = writeln!(file, "[PANIC] Transcript: {}", transcript_path.display());
            let _ = writeln!(file, "[PANIC] Artifacts: {}", artifacts_path.display());
        }
    }
}
```

### Part 2: Update SessionError Struct

Add transcript_path field to `SessionError`:

```rust
pub(crate) struct SessionError {
    pub error_type: SessionErrorType,
    pub component: String,
    pub message: String,
    pub timestamp: u64,
    pub last_action: Option<String>,
    pub context: serde_json::Value,
    pub transcript_path: Option<String>,  // NEW
    pub artifacts_path: Option<String>,   // NEW
}
```

### Part 3: Update `write_session_error()`

Include transcript and artifacts paths in JSON:

```json
{
  "error_type": "Panic",
  "component": "runtime",
  "message": "Panic at src/main.rs:123: index out of bounds",
  "timestamp": 1777223900,
  "last_action": "reading artifact",
  "context": {},
  "transcript_path": "/Users/r2/elma-cli/sessions/s_1777223805_63539000/display/terminal_transcript.txt",
  "artifacts_path": "/Users/r2/elma-cli/sessions/s_1777223805_63539000/artifacts"
}
```

### Part 4: Wire Into Session Startup

In `src/app_bootstrap_core.rs`:
- After `ensure_session_layout()`, store the session root in a global or thread-local
- Pass it to `install_panic_hook()` so it knows the session path on crash

Currently it's already passed; ensure it's always set before any code that might panic.

### Part 5: Error Report Printout

On panic, also print a user-friendly message:

```
❌ FATAL: Panic at src/main.rs:123: index out of bounds

   Error report: /Users/r2/Library/Application Support/rs.elma.elma-cli/sessions/s_1777223805_63539000/error.json
   
   For debugging, inspect:
   - Transcript: .../s_1777223805_63539000/display/terminal_transcript.txt
   - Artifacts: .../s_1777223805_63539000/artifacts/
   - Trace: .../s_1777223805_63539000/trace_debug.log
```

## Non-Requirements (Out of Scope)

- Automatic upload of error reports (user decision)
- Real-time error notifications (desktop notification)
- UI recovery (handled by session manager)

## Testing

- [ ] Intentionally cause a panic and verify error.json contains transcript_path
- [ ] Verify trace_debug.log includes transcript path
- [ ] Simulate panic during tool execution and verify artifacts are recoverable
- [ ] Test panic before session is initialized (session_root is None)

## Acceptance Criteria

1. error.json always includes transcript_path and artifacts_path (when session exists)
2. trace_debug.log includes human-readable paths on panic
3. Panic message prints artifact discovery guidance
4. Works even if panic happens before session is fully initialized

## Notes

- This task does not require changes to the UI
- Purely informational: helps the agent coder debug crashes
- Complements task 281 (transcript persistence) and 282 (index/GC)
- Follows Elma's principle: "Preserve evidence for debugging"
