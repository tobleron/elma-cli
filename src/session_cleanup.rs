//! Session cleanup utilities for querying and calculating space savings
//!
//! Provides functions to calculate disk space savings from deleting
//! sessions older than a specified time period using the session index.

use crate::*;
use std::path::PathBuf;

/// Calculate space savings from deleting sessions older than `days` days.
///
/// Uses the session index (sessions/index.json) for fast lookup.
pub(crate) fn sessions_savings(days: u64, workdir: &PathBuf) -> String {
    let sessions_root = workdir.join("sessions");
    let index = match crate::session_index::SessionIndex::load(&sessions_root) {
        Ok(idx) => idx,
        Err(_) => {
            return "=== Session Cleanup Analysis ===\n\
                    Error: Could not load session index. Run 'elma-cli session-gc' for detailed analysis.\n\
                    === End Analysis ===".to_string();
        }
    };

    let cutoff = current_unix() - (days * 86400);
    let old_sessions: Vec<&crate::session_index::SessionIndexEntry> = index
        .sessions
        .iter()
        .filter(|s| s.last_modified_unix < cutoff)
        .collect();

    let total_size: u64 = old_sessions.iter().map(|s| s.size_bytes).sum();
    let count = old_sessions.len();

    let mut out = String::new();
    out.push_str(&format!(
        "=== Session Cleanup Analysis ({} days) ===\n",
        days
    ));
    out.push_str(&format!(
        "Total sessions directory size: {}\n",
        human_size(index.total_size_bytes)
    ));
    out.push_str(&format!("Sessions older than {} days: {}\n", days, count));
    out.push_str(&format!(
        "Space in old sessions: {}\n",
        human_size(total_size)
    ));
    if count > 0 {
        out.push_str("\nOld sessions:\n");
        for s in &old_sessions {
            out.push_str(&format!(
                "- {} ({}, status: {})\n",
                s.id,
                human_size(s.size_bytes),
                s.status
            ));
        }
    }
    out.push_str("=== End Analysis ===");
    out
}

/// Quick check: get total sessions size
pub(crate) fn sessions_total_size(workdir: &PathBuf) -> String {
    let sessions_root = workdir.join("sessions");
    if let Ok(index) = crate::session_index::SessionIndex::load(&sessions_root) {
        human_size(index.total_size_bytes)
    } else {
        "N/A (no index)".to_string()
    }
}

/// Quick check: count old files
pub(crate) fn sessions_old_count(days: u64, workdir: &PathBuf) -> String {
    let sessions_root = workdir.join("sessions");
    if let Ok(index) = crate::session_index::SessionIndex::load(&sessions_root) {
        let count = index
            .sessions
            .iter()
            .filter(|s| s.last_modified_unix < current_unix() - (days * 86400))
            .count();
        count.to_string()
    } else {
        "0".to_string()
    }
}

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
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
