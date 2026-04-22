# Task 161: Self-Learning From Shell Command Errors

## Summary

Implement a mechanism that learns from shell command execution failures and uses the captured hindsight to improve future command suggestions for local models.

## Motivation

Local small models often output incorrect shell commands. This feature captures:
- The command that was attempted
- The error that occurred
- A hint about what went wrong

And uses this to provide better guidance for similar situations in the future.

This is inspired by Hermes Agent's On-Policy Distillation (OPD) but simplified for local/CPU usage.

## Source

Hermes Agent's `environments/agentic_opd_env.py` - specifically the hint extraction pipeline.

## Implementation

### Concept

```
User: "List files in src"
Model: Runs `ls src/`
Error: "ls: src: No such file or directory"
       ↓
Capture: {error: "No such file or directory", hint: "Check if directory exists first"}
       ↓
Future: Model about to run ls on path → Inject hint:
       "Directory may not exist. Consider checking with: ls -la <parent>"
```

### Data Model

```rust
struct ErrorHint {
    id: i64,              // Primary key
    error_pattern: String, // e.g., "No such file or directory"
    command_pattern: String, // e.g., "ls", "cat", "rm"
    hint: String,          // "Check if the path exists before operating"
    occurrence_count: i64, // How many times this error happened
    created_at: DateTime,
    updated_at: DateTime,
}
```

### SQLite Schema

```sql
CREATE TABLE error_hints (
    id INTEGER PRIMARY KEY,
    error_pattern TEXT NOT NULL,
    command_pattern TEXT,
    hint TEXT NOT NULL,
    occurrence_count INTEGER DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
);

CREATE INDEX idx_error_pattern ON error_hints(error_pattern);
CREATE INDEX idx_command_pattern ON error_hints(command_pattern);
```

### Hint Extraction

Instead of LLM judge (Hermes uses), use pattern matching:

```rust
fn extract_hint(error: &str, command: &str) -> Option<String> {
    // Map error patterns to helpful hints
    let hints = [
        ("No such file or directory", "Check if the path exists first with: ls -la <parent_path>"),
        ("Permission denied", "Check file permissions with: ls -la <path>"),
        ("command not found", "Is the command installed? Try: which <command>"),
        ("syntax error", "Check the command syntax - may need escaping or quotes"),
        ("not a directory", "Expected a directory but found a file. Remove the trailing / or use dirname"),
        ("Is a directory", "Target is a directory, not a file. Did you mean ls <path>/?"),
        ("Operation not permitted", "May need sudo or elevated permissions"),
        ("No space left on device", "Check disk space with: df -h"),
        ("Argument list too long", "Use glob pattern or xargs to batch process"),
    ];
    
    for (pattern, hint) in hints {
        if error.contains(pattern) {
            return Some(hint);
        }
    }
    None
}
```

### Storage

```rust
impl ErrorHintStore {
    async fn record_failure(&self, command: &str, error: &str) -> Result<()> {
        let hint = extract_hint(error, command);
        if hint.is_none() {
            return Ok(());  // No useful hint to record
        }
        
        // Upsert: increment occurrence if exists
        sqlx::query("""
            INSERT INTO error_hints (error_pattern, command_pattern, hint)
            VALUES (?, ?, ?)
            ON CONFLICT(error_pattern, command_pattern) 
            DO UPDATE SET occurrence_count = occurrence_count + 1
        """)
        .bind(&error)
        .bind(&command)
        .bind(&hint)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    
    async fn get_relevant_hints(&self, partial_command: &str) -> Result<Vec<String>> {
        // Find hints for similar command patterns
        sqlx::query_as::<_, ErrorHint>("""
            SELECT * FROM error_hints 
            WHERE command_pattern LIKE ?
            ORDER BY occurrence_count DESC
            LIMIT 5
        """)
        .bind(partial_command)
        .fetch_all(&self.pool)
        .await
    }
}
```

### Integration with Shell Tool

In the shell tool execution flow:

```rust
async fn execute_shell(&self, command: &str) -> Result<Output> {
    let output = self.inner.execute(command).await;
    
    // If command failed, record the failure
    if !output.success {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        self.error_hints.record_failure(command, &error_msg).await;
    }
    
    // Before executing potentially risky commands, inject hints
    let hints = self.error_hints.get_relevant_hints(command).await?;
    if !hints.is_empty() && self.is_risky_command(command) {
        // Add hints to the shell output for context
        return Err(Error::with_hints(output, hints));
    }
    
    Ok(output)
}
```

### Risky Command Detection

```rust
fn is_risky_command(cmd: &str) -> bool {
    let risky = ["rm", "dd", "mkfs", ">/dev/sd", "chmod 777", "chown -R"];
    risky.iter().any(|p| cmd.contains(p))
}
```

### Context Injection

When the model attempts a shell command that previously failed:

```
Model: ls /nonexistent/dir
System: "Before running: Check if the path exists first with: ls -la /"
         "(Previous failure: ls: /nonexistent/dir: No such file or directory)"
```

## Verification

- Error hints recorded on shell failures
- Similar commands retrieve relevant hints
- Hints displayed before executing risky commands

## Dependencies

- SQLite (from task 149)
- Shell tool integration

## Notes

- This is a simplified version of Hermes Agent's OPD
- Hermes uses token-level RL training; this uses rule-based hints
- For more advanced version, could add LLM-based hint extraction for novel errors
- The key insight: capture the failure + hint pairing for reuse

## Hermes Agent's Full OPD Reference

Hermes does token-level learning:
1. Runs agent on coding tasks
2. Extracts (assistant response, next_state) pairs
3. Uses LLM judge to extract hints from tool results
4. Scores student tokens under enhanced (hint + context) distribution
5. Computes advantage: A_t = teacher_logprob - student_logprob
6. On-policy distillation training signal

This task is the CLI-friendly simplified version.