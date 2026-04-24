# 207: Explicit `directories` Dependency for Cross-Platform Path Resolution

## Status
`pending`

## Crate
`directories` — Cross-platform config/cache/data directory paths.

## Rationale
Elma currently relies on Ratatui pulling `directories` in transitively. This makes the paths fragile — if Ratatui ever drops the dep or changes versions, Elma breaks. An explicit dep gives Elma full control over its own config dir (`elma.toml`), cache dir (model responses, embeddings), and data dir (sessions, skills).

## Implementation Boundary
- Add `directories = "5.0"` (or current) to `[dependencies]` in `Cargo.toml` — do NOT remove Ratatui's usage of it.
- Create `src/dirs.rs` (or `src/paths.rs`) that wraps `directories::ProjectDirs`:

  ```rust
  use directories::ProjectDirs;

  pub struct ElmaPaths {
      config_dir: PathBuf,
      cache_dir: PathBuf,
      data_dir: PathBuf,
  }

  impl ElmaPaths {
      pub fn new() -> Option<Self> {
          let proj = ProjectDirs::from("rs", "elma", "elma-cli")?;
          Some(Self {
              config_dir: proj.config_dir().to_path_buf(),
              cache_dir: proj.cache_dir().to_path_buf(),
              data_dir: proj.data_dir().to_path_buf(),
          })
      }
  }
  ```

- Expose `config_dir()`, `cache_dir()`, `data_dir()` getters.
- Add `ensure_dirs()` that creates all three on first run, returning `anyhow::Result<()>`.
- Audit existing code that constructs paths manually (e.g., `~/.config/elma/`, `.elmarc`, session dirs) and migrate to `ElmaPaths`.
- Verify no runtime panics if XDG/system dirs are unavailable — fall back gracefully.
- Do NOT change where existing config files live unless the old path is clearly wrong.

## Verification
- `cargo build` passes.
- `ElmaPaths::new()` returns `Some` on macOS/Linux.
- Existing session/config loading still works.