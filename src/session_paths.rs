//! @efficiency-role: data-model
//!
//! Session - Paths and Basic Setup

use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct SessionPaths {
    pub(crate) root: PathBuf,
    pub(crate) shell_dir: PathBuf,
    pub(crate) artifacts_dir: PathBuf,
    pub(crate) snapshots_dir: PathBuf,
    pub(crate) plans_dir: PathBuf,
    pub(crate) decisions_dir: PathBuf,
    pub(crate) tune_dir: PathBuf,
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
    let shell_dir = root.join("shell");
    let artifacts_dir = root.join("artifacts");
    let snapshots_dir = root.join("snapshots");
    let plans_dir = root.join("plans");
    let decisions_dir = root.join("decisions");
    let tune_dir = root.join("tune");

    std::fs::create_dir_all(&shell_dir)
        .with_context(|| format!("mkdir {}", shell_dir.display()))?;
    std::fs::create_dir_all(&artifacts_dir)
        .with_context(|| format!("mkdir {}", artifacts_dir.display()))?;
    std::fs::create_dir_all(&snapshots_dir)
        .with_context(|| format!("mkdir {}", snapshots_dir.display()))?;
    std::fs::create_dir_all(&plans_dir)
        .with_context(|| format!("mkdir {}", plans_dir.display()))?;
    std::fs::create_dir_all(&decisions_dir)
        .with_context(|| format!("mkdir {}", decisions_dir.display()))?;
    std::fs::create_dir_all(&tune_dir).with_context(|| format!("mkdir {}", tune_dir.display()))?;

    let master = plans_dir.join("_master.md");
    if !master.exists() {
        std::fs::write(
            &master,
            "# Master Plan\n\n- [ ] (Add high-level plan items here)\n",
        )
        .with_context(|| format!("write {}", master.display()))?;
    }

    Ok(SessionPaths {
        root,
        shell_dir,
        artifacts_dir,
        snapshots_dir,
        plans_dir,
        decisions_dir,
        tune_dir,
    })
}
