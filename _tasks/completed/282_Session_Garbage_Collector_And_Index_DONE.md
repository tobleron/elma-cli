# Task 282: Session Garbage Collector And Index

**Status**: Pending  
**Priority**: High  
**Depends on**: T281 (transcript persistence in display/ and sessions/)  
**Elma Philosophy**: Local-first, efficiency-first, preserve all evidence

## Goal

Implement a robust session garbage-collector and session index system to:
1. Keep sessions folder efficient without losing functionality
2. Compute exact space savings by session for retention queries
3. Support safe deletion and recovery workflows
4. Provide fast queries via lightweight JSON index

## Requirements

### Part 1: Session Index (`sessions/index.json`)

Create a lightweight JSON index that records per-session metadata:

```json
{
  "sessions": [
    {
      "id": "s_1777223805_63539000",
      "created_at_unix": 1777223805,
      "transcript_path": "s_1777223805_63539000/display/terminal_transcript.txt",
      "size_bytes": 245678,
      "artifact_count": 18,
      "last_modified_unix": 1777223900,
      "status": "completed" // "active", "completed", "error", "crashed"
    }
  ],
  "index_version": 1,
  "last_updated_unix": 1777224000,
  "total_sessions": 35,
  "total_size_bytes": 51335442
}
```

**Implementation**:
- After session closes (or on `write_session_status`), append entry to index.json
- Prune index entries for deleted sessions
- Keep index under 1MB by summarizing old sessions if needed

### Part 2: Garbage Collector CLI (`elma-cli session-gc`)

Add a new subcommand under `Commands`:

```bash
elma-cli session-gc --older-than-days 14 --dry-run

# Output:
Sessions eligible for deletion (older than 14 days):
- s_1776800000_123456 (5.2 MB)
- s_1776850000_234567 (3.1 MB)
Total savings: 8.3 MB (files: 48)

Use --confirm to delete, or --compress to create archive.
```

**Features**:
- `--older-than-days <N>`: Compute savings for sessions older than N days
- `--dry-run`: Show what would be deleted without changing anything (default)
- `--confirm`: Actually delete old sessions
- `--compress`: Create `.tar.gz` archive of old sessions before deletion (for safe recovery)
- `--archive-dir <path>`: Place archives under given directory (default: `sessions/.archive/`)
- Output: savings in bytes, file count, and preview of session IDs

**Implementation approach**:
- Use session index to avoid stat() calls on every file
- Filter by `last_modified_unix` from index
- Use Rust `fs` APIs (not shell commands) for atomic, reliable operations
- Log all operations to `sessions/gc.log` with timestamps

### Part 3: Space Savings Query (Wire to `sessions_savings()`)

Update `src/session_cleanup.rs`:
- Modify `sessions_savings()` to use the index instead of shell commands
- Use Rust fs API (fast, reliable)
- Return structured response with breakdown by session

### Part 4: Integration with Session Steward

Update `src/session_error.rs` and `write_session_status()`:
- On session completion, update index entry with final status and size
- On session error, mark as "error" in index
- On panic, mark as "crashed" in index

## Non-Requirements (Out of Scope)

- Encryption of archived sessions (use OS-level encryption if needed)
- Cloud backup
- Differential compression
- GUI for session management

## Testing

- [x] Verify index.json is created after session closes
- [x] Test `session-gc --dry-run` shows correct savings
- [x] Test `session-gc --compress` creates archive
- [x] Test `session-gc --confirm` deletes sessions safely
- [ ] Stress test with 100+ sessions
- [ ] Verify no data loss during concurrent CLI runs

## Acceptance Criteria

1. `sessions/index.json` exists after first session and updates automatically
2. `elma-cli session-gc --older-than-days 14 --dry-run` shows correct space savings (matches Python calculation)
3. Deleted sessions cannot be recovered except via `--archive` backups
4. All operations are logged to `sessions/gc.log`
5. Index stays under 1MB even with 1000+ sessions

## Notes

- This task preserves Elma's local-first philosophy
- No external tools or shell pipelines (use Rust fs APIs)
- All computation happens locally
- Small model-friendly (index lookup is O(1) instead of O(n) find commands)
