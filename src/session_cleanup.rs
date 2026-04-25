//! Session cleanup utilities for querying and calculating space savings
//!
//! Provides functions to calculate disk space savings from deleting
//! sessions older than a specified time period without requiring
//! complex chained shell commands.

use crate::*;
use std::path::PathBuf;
use std::os::unix::process::ExitStatusExt;

/// Calculate space savings from deleting sessions older than `days` days.
///
/// This internally runs shell commands but presents them as a single
/// analytical operation to avoid budget issues.
pub(crate) fn sessions_savings(days: u64, workdir: &PathBuf) -> String {
    let total_size = run_shell_one_liner_sync(&format!("du -sh ./sessions"), workdir, None);
    let old_count = run_shell_one_liner_sync(&format!("find ./sessions -type f -mtime +{} | wc -l", days), workdir, None);
    let old_size = run_shell_one_liner_sync(&format!("find ./sessions -type f -mtime +{} -exec du -ch {{}} + | tail -1", days), workdir, None);

    format!(
        "=== Session Cleanup Analysis ({} days) ===\n\
         Total sessions directory size: {}\n\
         Files older than {} days: {}\n\
         Space in old files: {}\n\
         === End Analysis ===",
        days, total_size.trim(), days, old_count.trim(), old_size.trim()
    )
}

/// Quick check: get total sessions size
pub(crate) fn sessions_total_size(workdir: &PathBuf) -> String {
    run_shell_one_liner_sync("du -sh ./sessions", workdir, None).trim().to_string()
}

/// Quick check: count old files
pub(crate) fn sessions_old_count(days: u64, workdir: &PathBuf) -> String {
    run_shell_one_liner_sync(&format!("find ./sessions -type f -mtime +{} | wc -l", days), workdir, None).trim().to_string()
}

fn run_shell_one_liner_sync(cmd: &str, workdir: &PathBuf, timeout: Option<u64>) -> String {
    use std::process::Command;
    
    let output = match timeout {
        Some(t) => Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(workdir)
            .output()
            .unwrap_or_else(|_| std::process::Output {
                status: std::process::ExitStatus::from_raw(1),
                stdout: Vec::new(),
                stderr: format!("timeout: {}ms", t).into_bytes(),
            }),
        None => Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(workdir)
            .output()
            .unwrap_or_else(|_| std::process::Output {
                status: std::process::ExitStatus::from_raw(1),
                stdout: Vec::new(),
                stderr: b"command failed".to_vec(),
            }),
    };

    String::from_utf8_lossy(&output.stdout).to_string()
}