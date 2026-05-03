//! @efficiency-role: data-model
//! Atomic write helper for safe file persistence.
//!
//! Writes content to a temp file, fsyncs, then renames atomically.
//! Prevents partial/corrupt writes from leaving inconsistent state.

use std::io::Write;
use std::path::Path;

/// Write content to a file atomically: temp file → fsync → rename.
pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp = path.with_extension("tmp");
    let mut file = std::fs::File::create(&tmp).map_err(|e| format!("create {}: {}", tmp.display(), e))?;
    file.write_all(content.as_bytes()).map_err(|e| format!("write {}: {}", tmp.display(), e))?;
    file.sync_all().map_err(|e| format!("fsync {}: {}", tmp.display(), e))?;
    drop(file);
    std::fs::rename(&tmp, path).map_err(|e| format!("rename {} -> {}: {}", tmp.display(), path.display(), e))?;
    Ok(())
}

/// Sync a file after writing (best-effort durability).
pub fn sync_file(path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(path).map_err(|e| format!("open {}: {}", path.display(), e))?;
    file.sync_all().map_err(|e| format!("sync {}: {}", path.display(), e))?;
    Ok(())
}
