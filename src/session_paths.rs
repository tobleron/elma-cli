//! @efficiency-role: data-model
//!
//! Session - Paths and Basic Setup
//!
//! Session layout (no backward compatibility with old structure):
//!   session.md        — chronological user-visible transcript, no thinking bodies
//!   session.json      — metadata, status, workspace brief, goal/runtime task state
//!   thinking.jsonl    — streamed thinking/reasoning records (turn id + timestamp)
//!   artifacts/        — raw tool outputs, shell scripts/output, snapshots, large docs

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

        // Verify no old-style directories are created
        let shell_dir = session.root.join("shell");
        let display_dir = session.root.join("display");
        let snapshots_dir = session.root.join("snapshots");
        let plans_dir = session.root.join("plans");
        let decisions_dir = session.root.join("decisions");
        let tune_dir = session.root.join("tune");

        assert!(!shell_dir.exists(), "old shell/ dir should not exist");
        assert!(!display_dir.exists(), "old display/ dir should not exist");
        assert!(
            !snapshots_dir.exists(),
            "old snapshots/ dir should not exist"
        );
        assert!(!plans_dir.exists(), "old plans/ dir should not exist");
        assert!(
            !decisions_dir.exists(),
            "old decisions/ dir should not exist"
        );
        assert!(!tune_dir.exists(), "old tune/ dir should not exist");

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
