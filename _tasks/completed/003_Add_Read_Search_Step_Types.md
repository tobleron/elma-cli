# Task 047: Add Explicit READ and SEARCH Step Types

## Priority
**P1 - HIGH** (Major accuracy gain, clean separation of concerns)

## Problem
Current `Step::Shell` conflates:
- Reading files (`cat file.txt`)
- Executing commands (`git status`)
- Searching (`grep -r pattern`)

This makes verification hard - a read step shouldn't have side effects, but Shell doesn't guarantee this.

## Objective
Add explicit `Step::Read` and `Step::Search` types for clear operation semantics.

## Implementation

### New Step Types (`src/types_core.rs`)

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub(crate) enum Step {
    // NEW: Explicit read-only file access
    #[serde(rename = "read")]
    Read {
        id: String,
        path: String,           // File to read
        purpose: String,        // Why we're reading
        common: StepCommon,
    },
    
    // NEW: Explicit search with query
    #[serde(rename = "search")]
    Search {
        id: String,
        query: String,          // Search pattern
        paths: Vec<String>,     // Where to search (empty = everywhere)
        purpose: String,
        common: StepCommon,
    },
    
    // EXISTING: Keep Shell for command execution
    #[serde(rename = "shell")]
    Shell { id, cmd, common },
    
    // ...existing types
}
```

### Execution Handlers (`src/execution_steps.rs`)

```rust
pub(crate) async fn handle_read_step(
    session: &SessionPaths,
    path: &str,
) -> Result<StepResult> {
    // Validate file exists
    // Validate file is readable (not binary huge)
    // Read content
    // Return content as step output
}

pub(crate) async fn handle_search_step(
    session: &SessionPaths,
    workdir: &Path,
    query: &str,
    paths: &[String],
) -> Result<StepResult> {
    // Use ripgrep/find for search
    // Limit output size
    // Return matches as step output
}
```

### Execution Integration (`src/execution.rs`)

```rust
for step in program.steps {
    match step {
        Step::Read { path, .. } => {
            handle_read_step(&session, &path).await?;
        }
        Step::Search { query, paths, .. } => {
            handle_search_step(&session, &workdir, &query, &paths).await?;
        }
        Step::Shell { cmd, .. } => {
            handle_shell_step(...).await?;
        }
        // ...existing
    }
}
```

## Acceptance Criteria
- [ ] `Step::Read` type added with path semantics
- [ ] `Step::Search` type added with query semantics
- [ ] Execution handlers implemented
- [ ] Orchestrator can generate Read/Search steps
- [ ] Backward compatibility: Shell still works for read/search
- [ ] Unit tests for new step types

## Expected Impact
- **+25% search accuracy** (explicit search vs Shell+grep)
- **+20% read safety** (validation before read)
- **Better verification** (know which steps are read-only)
- **Token reduction** (search results structured, not raw output)

## Dependencies
- None (additive change)

## Verification
- `cargo build`
- `cargo test`
- Test scenarios:
  - "read Cargo.toml" → Step::Read
  - "find where X is defined" → Step::Search
  - "run tests" → Step::Shell

## Architecture Alignment
- ✅ Articulate terminology (Read/Search clearly defined)
- ✅ Separation of concerns (read vs execute)
- ✅ Better verification (read-only steps can't have side effects)
