# Task 158: Session-based File Tracker

## Backlog Reconciliation (2026-05-02)

Superseded by Task 456 file context tracking and Task 469 session-state ownership.


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

Track which files are read during each session to provide context awareness and history.

## Motivation

- Help agents know what files they've already read
- Avoid redundant reads within a session
- Provide "recently accessed" functionality
- Support file change detection between reads

## Source

Crush's filetracker package at `_stress_testing/_crush/internal/filetracker/service.go`

## Implementation

### Service Interface

```go
type Service interface {
    // RecordRead records when a file was read
    RecordRead(ctx context.Context, session_id: String, path: String)

    // LastReadTime returns when a file was last read
    // Returns zero time if never read
    LastReadTime(ctx context.Context, session_id: String, path: String) -> time.Time

    // ListReadFiles returns the paths of all files read in a session
    ListReadFiles(ctx context.Context, session_id: String) -> Vec<String>
}
```

### Storage

- SQLite table for file reads per session
- Columns: session_id, path, read_at (timestamp)
- Queries: RecordFileRead, GetFileRead, ListSessionReadFiles

### Integration

- Read tool calls RecordRead after successful reads
- Check LastReadTime before re-reading (for cache invalidation)
- UI can display "recent files" from ListReadFiles

### Usage

```rust
// Check if file changed since last read
let last_read = tracker.last_read_time(&session_id, &path)?;
if last_read.elapsed() > Duration::from_secs(300) {
    // Re-read file (5+ minutes old)
}
// Otherwise use cached content
```

## Verification

- File reads recorded in DB
- LastReadTime returns correct timestamps
- ListReadFiles returns all files from session

## Dependencies

- SQLite (from task 149)
- Session management (existing)

## Notes

- This is for read tracking, not write tracking
- Consider path normalization for comparison
- Track relative paths from workspace root