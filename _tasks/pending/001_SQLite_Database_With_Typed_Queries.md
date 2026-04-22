# 149 SQLite Database With Typed Queries

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
Implement proper SQLite database with typed queries like OpenCode.

## Reference
- OpenCode: `internal/db/db.go`, `internal/db/migrations/`
- Uses sqlc for compile-time checked queries

## Implementation

### 1. Database Schema
File: `src/db/schema.sql`
- sessions: id, title, created_at, updated_at, summary_message_id, total_tokens, total_cost
- messages: id, session_id, role, content_json, created_at
- files: id, session_id, path, content, version, created_at

### 2. Query Layer
File: `src/db/queries.rs` (new)
- Use `rusqlite` with typed Query/Statement
- Prepared statements at startup
- Transaction support

### 3. Repository Pattern
File: `src/db/repository.rs` (new)
- `SessionRepository` - CRUD for sessions
- `MessageRepository` - CRUD for messages  
- `FileVersionRepository` - file versioning

### 4. File Versioning
- Every edit creates new version
- Query by session + path + version
- Enables diff viewing and rollback

## Verification
- [ ] `cargo build` passes
- [ ] Queries compile correctly
- [ ] File versioning works