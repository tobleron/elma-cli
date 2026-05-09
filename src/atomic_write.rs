//! @efficiency-role: data-model
//! Atomic write helper for safe file persistence.
//!
//! Writes content to a temp file, fsyncs, then renames atomically.
//! Prevents partial/corrupt writes from leaving inconsistent state.

use anyhow::{Context, Result};
use std::io::Write;
use std::path::Path;

/// Write content to a file atomically: temp file → fsync → rename.
pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let tmp = path.with_extension("tmp");
    let mut file =
        std::fs::File::create(&tmp).with_context(|| format!("create {}", tmp.display()))?;
    file.write_all(content.as_bytes())
        .with_context(|| format!("write {}", tmp.display()))?;
    file.sync_all()
        .with_context(|| format!("fsync {}", tmp.display()))?;
    drop(file);
    std::fs::rename(&tmp, path)
        .with_context(|| format!("rename {} -> {}", tmp.display(), path.display()))?;
    Ok(())
}

/// Sync a file after writing (best-effort durability).
pub fn sync_file(path: &Path) -> Result<()> {
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("sync {}", path.display()))?;
    Ok(())
}
