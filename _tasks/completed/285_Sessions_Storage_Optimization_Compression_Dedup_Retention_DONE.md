# Task 285: Sessions Storage Optimization (Compression, Dedup, Retention)

**Status**: Pending  
**Priority**: Medium  
**Depends on**: T282 (session index), T283 (transcript flushing)  
**Elma Philosophy**: Small-model-friendly, local-first, efficiency without losing functionality

## Goal

Implement storage optimization strategies to keep the `sessions/` directory efficient:
1. Compress old session artifacts (non-transcript)
2. Detect and deduplicate redundant tool outputs
3. Implement automatic retention policies
4. Keep index lightweight

This reduces disk footprint for users with constrained storage (e.g., laptops, edge devices) while preserving full recovery capability.

## Requirements

### Part 1: Artifact Compression

In `src/session_cleanup.rs` (or new `src/session_storage.rs`):

**Strategy**:
- Sessions older than 7 days: compress `artifacts/` to `artifacts.tar.gz` (one file per session)
- Keep `display/terminal_transcript.txt` uncompressed (high-value for debugging)
- Keep `session_status.json`, `error.json`, etc. uncompressed (needed for index queries)

**Implementation**:
```rust
pub(crate) fn compress_old_artifacts(session_root: &PathBuf, days_old: u64) -> Result<usize> {
    // Check if session is older than N days
    // If yes and artifacts/ is uncompressed:
    //   - Create artifacts.tar.gz with flate2
    //   - Remove artifacts/
    //   - Update session index entry
    // Return bytes saved
}
```

**Testing**:
- [ ] Compress a session with 50 small files, verify tar.gz is created
- [ ] Decompress and verify content matches
- [ ] Verify transcript still readable after compression

### Part 2: Deduplication Detector

Add a utility to detect redundant tool outputs:

```rust
pub(crate) fn find_duplicate_tool_outputs(sessions_root: &PathBuf) -> Result<Vec<(String, usize)>> {
    // Scan all tool_*.txt files across sessions
    // Group by content hash (SHA256)
    // Return duplicates: (hash, count)
    // Optionally: hardlink identical files (on supported filesystems)
}
```

**Use case**: If multiple sessions ran the same `ls -la` command, outputs are likely identical.

**Testing**:
- [ ] Run two sessions with identical commands, verify duplicates detected
- [ ] Verify hardlink approach saves space on supported filesystems

### Part 3: Automatic Retention Policy

In `src/session_cleanup.rs`, add a retention policy:

```rust
pub(crate) enum RetentionPolicy {
    Keep(u64), // Keep all sessions
    KeepDays(u64), // Keep sessions from last N days
    KeepCount(usize), // Keep last N sessions (FIFO)
    Hybrid { keep_days: u64, min_count: usize }, // Keep N days OR last M sessions
}

pub(crate) fn apply_retention_policy(
    sessions_root: &PathBuf,
    policy: RetentionPolicy,
    dry_run: bool,
) -> Result<RetentionStats> {
    // Apply policy, return stats on deletions
}
```

**Integrate with `session-gc` (task 282)**:
- `elma-cli session-gc --policy keep-days=14` uses Hybrid(14, 5)
- `elma-cli session-gc --policy keep-count=10` keeps last 10 sessions

### Part 4: Index Compaction

Prevent `sessions/index.json` from growing unbounded:

```rust
pub(crate) fn compact_session_index(sessions_root: &PathBuf) -> Result<()> {
    // If index > 500KB:
    //   - Archive old entries (pre-computed summaries)
    //   - Prune deleted sessions
    // Keep current entries (last 100 sessions) in main index
}
```

### Part 5: Storage Report

Add `elma-cli session-stats`:

```bash
elma-cli session-stats

# Output:
Sessions Storage Report:
- Total sessions: 45
- Total size: 51.8 MB
  - Uncompressed artifacts: 23.4 MB (45%)
  - Transcripts: 2.1 MB (4%)
  - Other: 26.3 MB (51%)
- Largest session: s_1777223805 (2.3 MB)
- Oldest session: s_1776000000 (20 days old)
- Compression potential: 18.2 MB (35%)
- Dedup potential: 3.1 MB (6%)

Recommendations:
- Run: elma-cli session-gc --older-than-days 14
- Run: elma-cli session-gc --compress
```

## Non-Requirements (Out of Scope)

- Network compression (e.g., rsync, zstd with remote)
- Encryption before compression
- Cloud sync

## Testing

- [ ] Compress 5 old sessions, verify space saved
- [ ] Run dedup detector on 10 sessions with overlapping commands
- [ ] Apply retention policy and verify only correct sessions remain
- [ ] Verify index compaction keeps recent entries
- [ ] Run `session-stats` on real sessions folder

## Acceptance Criteria

1. Artifact compression saves ≥30% space for typical sessions
2. Dedup detector runs in <5 seconds on 50 sessions
3. Retention policy applies correctly without data loss (except deleted sessions)
4. Index stays under 500KB even with 500+ sessions
5. `session-stats` output is accurate and actionable

## Notes

- Compression is optional (user opt-in via `session-gc --compress`)
- Dedup is informational, not automatic (prevent data surprises)
- Retention policy is configurable per deployment
- All operations logged to `sessions/gc.log`
- Complements tasks 282 (GC CLI) and 283 (transcript flushing)
