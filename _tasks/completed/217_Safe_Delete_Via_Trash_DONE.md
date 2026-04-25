# 217: Safe Delete via `trash`

## Status
`pending`

## Crate
`trash` — Move files to OS trash instead of deleting.

## Rationale
Elma executes file operations through LLM-generated shell commands, and has destructive command detection/preflight (Tasks 116-119). A safety net: when Elma deletes files during skill execution or session cleanup, moving them to the OS trash instead of `rm` gives users a recovery path. `trash` provides cross-platform (Linux/macOS/Windows) trash integration.

## Implementation Boundary
- Add `trash = "3.1"` to `Cargo.toml`.
- Create `src/trash.rs` with a safe delete helper:

  ```rust
  use trash::{remove, Error};

  pub fn trash_file(path: &Path) -> Result<(), Error> {
      remove(path)
  }
  ```

- Identify file deletion paths in Elma:
  - Session cleanup on `clear` command
  - Skill temp file removal after processing
  - Any `std::fs::remove_file` / `tokio::fs::remove_file` calls on user files
- Replace irreversible `remove_file`/`remove_dir_all` calls on user-facing files with `trash()`.
- Keep `remove_file` for temp/ephemeral files (Task 212 temp dir cleanup) — those should truly disappear.
- Provide a `trash_batch(paths: &[PathBuf])` for bulk operations.
- Add an `RUST_BACKTRACE` check: if the OS trash integration fails (e.g., no trash dir found), fall back to permanent deletion with a warning log.
- Do NOT change the destructive command detection/preflight system — this is an independent safety net.

## Verification
- `cargo build` passes.
- Deleted files appear in macOS Finder Trash / Linux trashcan.
- Graceful fallback when trash is unavailable.
- Session cleanup still works end-to-end.