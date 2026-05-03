# 567 — Audit and Harden Evidence Ledger Consistency

- **Priority**: Medium
- **Category**: Validation
- **Depends on**: 554 (session-scoped state)
- **Blocks**: None

## Problem Statement

The evidence ledger (`evidence_ledger.rs`, 1012 lines) provides evidence tracking and grounding enforcement but has several consistency concerns:

1. **Global static with `RwLock<Option<>>`**: The `SESSION_LEDGER` is a global that's cleared at end of turn (`clear_session_ledger()`). Race conditions could occur if multiple operations access it concurrently.

2. **Non-transactional persistence**: `persist()` writes to `session.json` and a `ledger.json` file sequentially — if one write succeeds and the other fails, the ledger is inconsistent.

3. **ANSI stripping may fail**: `strip_ansi_escapes::strip()` is fallible (accepts `&[u8]`, returns `Result<Vec<u8>>`). On failure, falls back to raw output — which may still contain ANSI codes.

4. **Evidence staleness is path-based only**: `check_file_is_stale()` only checks mtime. If a file is edited and reverted to original content within the same second, mtime doesn't change but content did.

5. **Hash function is non-cryptographic**: Uses `DefaultHasher` (SipHash) — fine for dedup, but file_hash won't detect intentional collisions. Not a security issue but worth documenting.

6. **Claims system is underutilized**: `add_claim()` exists but is only called in tests. The enforcement gate creates claim verdicts but doesn't persist them back to the ledger's claims list.

## Why This Matters for Small Local LLMs

Small models rely heavily on evidence grounding to avoid hallucination. If the evidence ledger has consistency issues:
- A model might trust stale evidence (file changed but staleness not detected)
- Evidence from a previous turn might leak into current turn (ledger not properly cleared)
- Claims might be verified but the verification result lost

## Current Behavior

```rust
// evidence_ledger.rs
static SESSION_LEDGER: OnceLock<RwLock<Option<EvidenceLedger>>> = OnceLock::new();

pub(crate) fn clear_session_ledger() {
    if let Ok(mut lock) = session_ledger().write() {
        *lock = None;  // just drops the Option
    }
}
```

## Recommended Target Behavior

1. Move ledger to `SessionState` (per Task 554)
2. Make persistence transactional (write to temp file, then rename)
3. Add content-hash-based staleness detection alongside mtime
4. Persist claim verdicts back to the ledger
5. Add integrity checks on ledger load

## Source Files That Need Modification

- `src/evidence_ledger.rs` — Move from global to SessionState, add transactional persistence
- `src/evidence_summary.rs` — Audit summarization for edge cases
- `src/session_write.rs` — Add transactional write helpers
- `src/tool_loop.rs` — Update ledger access to use SessionState

## Step-by-Step Implementation Plan

1. Move `EvidenceLedger` ownership to `SessionState` (after Task 554)
2. Add `persist_atomic()` using write-to-temp-then-rename pattern
3. Add content hash tracking for staleness:
   ```rust
   fn compute_content_hash(path: &Path) -> Option<u64> {
       let content = std::fs::read(path).ok()?;
       // Use SHA-1 (already a dependency) for content hashing
       let mut hasher = sha1::Sha1::new();
       hasher.update(&content);
       let hash = hasher.digest().to_string();
       // Truncate to u64 for compact storage
       // ...
   }
   ```
4. Persist claim verdicts in `enforce_evidence_grounding_with_intel()`
5. Add ledger integrity check on load (verify entries reference valid paths)
6. Add unit tests for transactional failure recovery

## Recommended Crates

- `sha1` — already a dependency for content hashing
- None new required

## Testing Plan

1. Test transactional persistence (simulate write failure mid-way)
2. Test content-hash staleness detection
3. Test that claim verdicts are persisted
4. Test ledger load with corrupted file → should degrade gracefully
5. Test concurrent access safety (if still using RwLock)

## Acceptance Criteria

- Evidence ledger is owned by SessionState (not global)
- Persistence is transactional (no partial writes)
- Staleness detection uses both mtime and content hash
- Claim verdicts are persisted to ledger claims
- All existing evidence tests pass

## Risks and Migration Notes

- Content hashing on every evidence entry may add latency for large files. Hash only small files (<1MB) or compute lazily.
- Moving from global to SessionState requires updating all access sites — pair with Task 554.
