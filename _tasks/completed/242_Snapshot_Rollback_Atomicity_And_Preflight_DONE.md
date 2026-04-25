# 242: Snapshot Rollback Atomicity And Pre-Check Gate

## Status
`pending`

## Priority
Medium — Reliability: rollback deletes files before verifying restore success.

## Source
Code review finding M-12. `rollback_workspace_snapshot` in `snapshot.rs` copies files back into the workspace, then removes files not in the manifest. If a `copy` fails mid-loop (disk full, permissions error), the function returns `Err` — but files already removed from the workspace are gone. No transactional guarantee exists.

## Objective
Add a pre-flight verification gate before the destructive phase of rollback. Verify all snapshot source files are readable before touching the workspace.

## Scope

### `src/snapshot.rs` — `rollback_workspace_snapshot`

**1. Add pre-flight existence check before any workspace mutation:**
```rust
// Pre-check: verify all snapshot source files are readable before touching workspace
for rel_str in &manifest.files {
    let src = files_dir.join(PathBuf::from(rel_str));
    if !src.exists() {
        anyhow::bail!(
            "Snapshot source file missing: {}. Rollback aborted to protect workspace.",
            src.display()
        );
    }
    // Verify it's actually readable
    std::fs::File::open(&src)
        .with_context(|| format!("Cannot read snapshot file: {}", src.display()))?;
}
```

**2. Separate the restore phase from the deletion phase with a clear boundary comment.**

**3. Consider atomic rename on restore (stretch goal):**
- Copy snapshot files to `<dest>.elma_restore_tmp` first.
- Only after all copies succeed, rename each to its final path.
- On any failure, clean up `.elma_restore_tmp` files without touching originals.

**4. Add test coverage for the failure scenario:**
```rust
#[test]
fn rollback_aborts_when_snapshot_file_missing() {
    // Create a snapshot, manually delete a file from it, attempt rollback
    // Verify workspace is untouched and Err is returned
}
```

### `src/snapshot.rs` — `rollback_workspace_snapshot` deletion phase
- Move the file deletion loop (lines 97–110) to a separate clearly-named function `remove_extra_files_after_restore`.
- Add a trace log entry before each deletion: `trace(args, &format!("rollback_removing path={}", path.display()))`.

## Verification
- `cargo build` passes.
- `cargo test snapshot` passes.
- New failure-scenario test passes.
- Manual: corrupt a snapshot source file and attempt rollback — verify `Err` with clear message, workspace unchanged.

## References
- `src/snapshot.rs:60–119` (rollback_workspace_snapshot)
- `src/snapshot.rs:79–95` (restore loop)
- `src/snapshot.rs:97–110` (deletion loop)
