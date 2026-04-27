//! @efficiency-role: service
//!
//! Session Garbage Collector CLI (Task 282)
//!
//! Provides `elma-cli session-gc` for safe, indexed session cleanup.

use crate::*;
use chrono::Local;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::path::PathBuf;
use tar::Builder;

/// Arguments for session-gc command
#[derive(Debug, Clone)]
pub(crate) struct SessionGcArgs {
    pub(crate) older_than_days: u64,
    pub(crate) dry_run: bool,
    pub(crate) confirm: bool,
    pub(crate) compress: bool,
    pub(crate) archive_dir: Option<PathBuf>,
}

/// Automatically clean up sessions older than specified days (no archive, no confirmation)
///
/// Returns the number of sessions deleted and space reclaimed.
pub(crate) fn auto_cleanup_sessions(
    sessions_root: &PathBuf,
    older_than_days: u64,
) -> Result<(usize, u64)> {
    let mut index = match crate::session_index::SessionIndex::load(sessions_root) {
        Ok(idx) => idx,
        Err(_) => crate::session_index::build_index(sessions_root)?,
    };

    let cutoff = current_unix() - (older_than_days * 86400);
    let to_delete: Vec<String> = index
        .sessions
        .iter()
        .filter(|s| s.last_modified_unix < cutoff)
        .map(|s| s.id.clone())
        .collect();

    if to_delete.is_empty() {
        return Ok((0, 0));
    }

    let mut deleted_count = 0;
    let mut deleted_size = 0u64;
    for id in &to_delete {
        let session_dir = sessions_root.join(id);
        if session_dir.exists() {
            let size = index
                .sessions
                .iter()
                .find(|e| e.id == *id)
                .map(|e| e.size_bytes)
                .unwrap_or(0);
            if fs::remove_dir_all(&session_dir).is_ok() {
                deleted_count += 1;
                deleted_size += size;
                log_gc(&format!(
                    "Auto-deleted session {} ({})",
                    id,
                    human_size(size)
                ));
            }
        }
    }

    // Update index
    index.sessions.retain(|e| !to_delete.contains(&e.id));
    index.recompute_totals();
    let _ = index.save(sessions_root);

    Ok((deleted_count, deleted_size))
}

/// Run the session garbage collector
///
/// Returns a summary string for user display.
pub(crate) fn run_session_gc(sessions_root: &PathBuf, args: &SessionGcArgs) -> Result<String> {
    let index_path = sessions_root.join("index.json");
    // Load index; if missing, build by scanning directory
    let mut index = if index_path.exists() {
        let json = fs::read_to_string(&index_path)
            .with_context(|| format!("read index {}", index_path.display()))?;
        serde_json::from_str::<crate::session_index::SessionIndex>(&json)
            .with_context(|| format!("parse index {}", index_path.display()))?
    } else {
        // No index yet; build by scanning
        crate::session_index::build_index(sessions_root)?
    };

    let cutoff = current_unix() - (args.older_than_days * 86400);

    // Build owned list of eligible sessions (no borrow of index after this)
    let mut candidates: Vec<(String, u64, u32, u64, String)> = index
        .sessions
        .iter()
        .filter(|s| s.last_modified_unix < cutoff)
        .map(|s| {
            (
                s.id.clone(),
                s.size_bytes,
                s.artifact_count,
                s.last_modified_unix,
                s.status.clone(),
            )
        })
        .collect();

    // Sort by last_modified_unix ascending (oldest first)
    candidates.sort_by_key(|c| c.3);

    if candidates.is_empty() {
        return Ok(format!(
            "No sessions older than {} days found.",
            args.older_than_days
        ));
    }

    // Compute totals
    let total_size: u64 = candidates.iter().map(|c| c.1).sum();
    let total_files: u32 = candidates.iter().map(|c| c.2).sum();

    // Dry-run output
    if args.dry_run || (!args.confirm && !args.compress) {
        let mut out = String::new();
        out.push_str(&format!(
            "Sessions eligible for deletion (older than {} days):\n",
            args.older_than_days
        ));
        for (id, size, _art, _lmt, status) in &candidates {
            out.push_str(&format!(
                "- {} ({}, status: {})\n",
                id,
                human_size(*size),
                status
            ));
        }
        out.push_str(&format!(
            "\nTotal savings: {} ({} artifacts)\n",
            human_size(total_size),
            total_files
        ));
        out.push_str("\nUse --confirm to delete, or --compress to create archive.");
        return Ok(out);
    }

    // If compress: create archive first
    if args.compress {
        let archive_base = args
            .archive_dir
            .clone()
            .unwrap_or_else(|| sessions_root.join(".archive"));
        fs::create_dir_all(&archive_base)
            .with_context(|| format!("mkdir archive dir {}", archive_base.display()))?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let archive_name = format!("sessions_{}.tar.gz", timestamp);
        let archive_path = archive_base.join(archive_name);

        // Collect session IDs
        let session_ids: Vec<&str> = candidates.iter().map(|c| c.0.as_str()).collect();

        // Create tarball
        let file = File::create(&archive_path)
            .with_context(|| format!("create archive {}", archive_path.display()))?;
        let mut gz = GzEncoder::new(file, Compression::default());
        let mut tar = Builder::new(gz);

        for sid in &session_ids {
            let session_dir = sessions_root.join(sid);
            if session_dir.exists() {
                tar.append_dir_all(sid, &session_dir)
                    .with_context(|| format!("add {} to archive", sid))?;
            }
        }
        tar.finish()?;

        let archive_size = fs::metadata(&archive_path)
            .ok()
            .map(|m| m.len())
            .unwrap_or(0);

        log_gc(&format!(
            "Archived {} sessions to {} ({})",
            session_ids.len(),
            archive_path.display(),
            human_size(archive_size)
        ));
    }

    // If confirm: delete sessions
    if args.confirm {
        let mut deleted_count = 0;
        let mut deleted_size = 0u64;

        // Build set of IDs to delete for index pruning
        let to_delete_set: HashSet<String> = candidates.iter().map(|c| c.0.clone()).collect();

        for (id, size, _art, _lmt, _status) in &candidates {
            let session_dir = sessions_root.join(id);
            if session_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&session_dir) {
                    log_gc(&format!("Failed to delete {}: {}", id, e));
                } else {
                    deleted_count += 1;
                    deleted_size += size;
                    log_gc(&format!("Deleted session {} ({})", id, human_size(*size)));
                }
            }
        }

        // Prune index entries using the in-memory index
        index.sessions.retain(|e| !to_delete_set.contains(&e.id));
        index.recompute_totals();
        let _ = index.save(sessions_root);

        return Ok(format!(
            "Deleted {} sessions, reclaimed {}.",
            deleted_count,
            human_size(deleted_size)
        ));
    }

    // If compress-only (no confirm), just show archive message
    if args.compress && !args.confirm {
        let archive_base = args
            .archive_dir
            .clone()
            .unwrap_or_else(|| sessions_root.join(".archive"));
        return Ok(format!(
            "Archive mode: sessions would be compressed to {} (without deletion). Use --confirm to delete after archiving.",
            archive_base.display()
        ));
    }

    Ok("Operation complete.".to_string())
}

/// Log a message to sessions/gc.log
fn log_gc(message: &str) {
    let log_path = PathBuf::from("sessions").join("gc.log");
    let timestamp = Local::now().to_rfc3339();
    let line = format!("[{}] {}\n", timestamp, message);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map(|mut f| f.write_all(line.as_bytes()));
}

/// Human-readable file size
fn human_size(bytes: u64) -> String {
    let kb = bytes as f64 / 1024.0;
    let mb = kb / 1024.0;
    let gb = mb / 1024.0;
    if gb >= 1.0 {
        format!("{:.1} GB", gb)
    } else if mb >= 1.0 {
        format!("{:.1} MB", mb)
    } else {
        format!("{:.0} KB", kb)
    }
}

fn current_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_human_size() {
        assert_eq!(human_size(1024), "1 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
        assert_eq!(human_size(1024 * 1024 * 1024), "1.0 GB");
    }
}
