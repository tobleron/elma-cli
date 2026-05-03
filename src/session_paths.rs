//! @efficiency-role: data-model
//!
//! Session - Paths and Basic Setup
//!
//! ## Canonical Session Store Policy
//!
//! Two stores are canonical. All other session data files are legacy duplicates
//! that exist only for backward-compatible fallback reads — they must not be
//! written in new sessions.
//!
//! ### Canonical stores
//!   session.md        — chronological user-visible transcript, no thinking bodies
//!   session.json      — metadata, status, workspace brief, goal/runtime task state,
//!                       hierarchy, evidence, turn summaries
//!   thinking.jsonl    — streamed thinking/reasoning records (turn id + timestamp)
//!   artifacts/        — raw tool outputs, shell scripts/output, snapshots, large docs
//!
//! ### Legacy stores (duplicates — writers should be phased out)
//!   terminal_transcript.txt    — duplicate of session.md
//!   error.json                 — duplicate of session.json.status.error
//!   session_status.json        — duplicate of session.json.status
//!   hierarchy/*.json           — duplicate of session.json.hierarchy
//!   runtime_tasks/*.json       — duplicate of session.json.runtime_task
//!   workspace.txt              — duplicate of session.json.runtime.workspace
//!   workspace_brief.txt        — same as above
//!   project_guidance.txt       — duplicate of session.json.runtime.guidance_snapshot
//!   evidence/{id}/ledger.json  — duplicate of session.json.evidence
//!   summaries/*.md             — duplicate of session.json.turn_summaries
//!
//! ### Dead code
//!   session_store.rs (SQLite)  — never instantiated in production; ~510 lines dead
//!
//! Readers may fall back to legacy files for sessions created before this policy
//! took effect. Writers must never create legacy files from this point forward.

use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct SessionPaths {
    pub(crate) root: PathBuf,
    pub(crate) artifacts_dir: PathBuf,
}

pub(crate) fn new_session_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    Ok(format!("s_{:010}_{}", now.as_secs(), now.subsec_nanos()))
}

pub(crate) fn ensure_session_layout(sessions_root: &PathBuf) -> Result<SessionPaths> {
    std::fs::create_dir_all(sessions_root)
        .with_context(|| format!("mkdir {}", sessions_root.display()))?;

    let sid = new_session_id()?;
    let root = sessions_root.join(&sid);
    let artifacts_dir = root.join("artifacts");

    std::fs::create_dir_all(&artifacts_dir)
        .with_context(|| format!("mkdir {}", artifacts_dir.display()))?;

    Ok(SessionPaths {
        root,
        artifacts_dir,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_ensure_session_layout_creates_expected_structure() {
        let tmp = tempfile::tempdir().unwrap();
        let sessions_root = tmp.path().to_path_buf();
        let session = ensure_session_layout(&sessions_root).unwrap();

        assert!(session.root.exists(), "session root should exist");
        assert!(session.artifacts_dir.exists(), "artifacts/ should exist");
        assert!(
            session.artifacts_dir.is_dir(),
            "artifacts/ should be a directory"
        );

        // Verify no old-style directories or root files are created
        let old_paths = [
            ("shell", false),
            ("display", false),
            ("snapshots", false),
            ("plans", false),
            ("decisions", false),
            ("tune", false),
            ("workspace.txt", false),
            ("workspace_brief.txt", false),
            ("session_status.json", false),
            ("error.json", false),
            ("hierarchy", false),
            ("runtime_tasks", false),
            ("tool-results", false),
            ("evidence", false),
        ];
        for (name, _should_exist) in &old_paths {
            let p = session.root.join(name);
            assert!(!p.exists(), "legacy path {} should not exist", name);
        }

        // Verify only artifacts/ exists as a subdirectory
        let entries: Vec<_> = fs::read_dir(&session.root)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .collect();
        assert_eq!(entries.len(), 1, "only one subdirectory should exist");
        assert_eq!(entries[0].file_name().to_string_lossy(), "artifacts");

        // Clean up
        fs::remove_dir_all(&sessions_root).unwrap();
    }

    #[test]
    fn test_new_session_id_is_unique() {
        let id1 = new_session_id().unwrap();
        let id2 = new_session_id().unwrap();
        assert_ne!(id1, id2, "session IDs should be unique");
        assert!(id1.starts_with("s_"), "session ID should start with s_");
    }
}
