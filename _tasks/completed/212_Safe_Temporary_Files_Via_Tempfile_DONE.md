# 212: Safe Temporary Files via `tempfile`

## Status
`pending`

## Crate
`tempfile` — Safe temporary files and directories.

## Rationale
Elma processes skill documents, ebook extractions, and skill output that may need temporary staging before rendering or persistence. Using raw `std::fs::File` in `/tmp` is fragile (race conditions, leaks on panic). `tempfile` handles creation, uniqueness, and cleanup automatically, including on crash.

## Implementation Boundary
- Add `tempfile = "3.12"` to `Cargo.toml`.
- Audit places where Elma writes to `/tmp` or uses `std::env::temp_dir()` manually.
- Create `src/temp.rs` with a scoped temp directory helper:

  ```rust
  use tempfile::TempDir;

  pub struct ScopedTemp {
      dir: TempDir,
  }

  impl ScopedTemp {
      pub fn new(prefix: &str) -> anyhow::Result<Self> {
          let dir = tempfile::Builder::new()
              .prefix(prefix)
              .tempdir()?;
          Ok(Self { dir })
      }
      pub fn path(&self) -> &Path { self.dir.path() }
  }
  ```

- Replace at least one manual temp file/dir creation in the document processing pipeline (e.g., ebook extraction in `pdf-extract`/ epub handling, or skill temp staging).
- `ScopedTemp` drops and cleans up automatically on scope exit or panic.
- Do NOT replace in-memory buffers where temp files are unnecessary.
- Do NOT use temp files for session data that must survive process restart.

## Verification
- `cargo build` passes.
- Temp files are cleaned up after use (test by running a skill that processes a doc and checking `/tmp`).
- No temp file conflicts when multiple Elma processes run simultaneously.