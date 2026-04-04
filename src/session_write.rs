//! @efficiency-role: infra-adapter
//!
//! Session - Write Helpers

use crate::*;

pub(crate) fn write_shell_action(shell_dir: &PathBuf, cmd_line: &str) -> Result<PathBuf> {
    let n = next_shell_seq(shell_dir)?;
    let path = shell_dir.join(format!("{n:03}.sh"));
    std::fs::write(&path, format!("{cmd_line}\n"))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_shell_output(
    shell_dir: &PathBuf,
    seq_path: &PathBuf,
    output: &str,
) -> Result<PathBuf> {
    let stem = seq_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "000".to_string());
    let path = shell_dir.join(format!("{stem}.out"));
    std::fs::write(&path, output).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_plan_file(plans_dir: &PathBuf, content: &str) -> Result<PathBuf> {
    let n = next_plan_seq(plans_dir)?;
    let path = plans_dir.join(format!("plan_{n:03}.md"));
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn append_master_link(
    plans_dir: &PathBuf,
    plan_path: &PathBuf,
    title: &str,
) -> Result<()> {
    let master = plans_dir.join("_master.md");
    let rel = plan_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plan_???".to_string());
    let line = format!("- [ ] {title} ({rel})\n");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&master)
        .with_context(|| format!("open {}", master.display()))?
        .write_all(line.as_bytes())
        .with_context(|| format!("append {}", master.display()))?;
    Ok(())
}

pub(crate) fn write_decision(decisions_dir: &PathBuf, word: &str) -> Result<PathBuf> {
    let n = next_decision_seq(decisions_dir)?;
    let path = decisions_dir.join(format!("{n:03}.txt"));
    std::fs::write(&path, format!("{}\n", word.trim()))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_gate_why(tune_dir: &PathBuf, text: &str) -> Result<PathBuf> {
    let n = next_gate_why_seq(tune_dir)?;
    let path = tune_dir.join(format!("gate_why_{n:03}.txt"));
    std::fs::write(&path, text.trim().to_string() + "\n")
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_thinking_log(
    shell_dir: &PathBuf,
    seq: u32,
    thinking_content: &str,
) -> Result<PathBuf> {
    let path = shell_dir.join(format!("{seq:03}.thinking.log"));
    std::fs::write(&path, thinking_content)
        .with_context(|| format!("write thinking log {}", path.display()))?;
    Ok(path)
}

/// Save goal state to session file (Task 014)
pub(crate) fn save_goal_state(session_root: &PathBuf, goal_state: &GoalState) -> Result<PathBuf> {
    let path = session_root.join("goal_state.json");
    let json = serde_json::to_string_pretty(goal_state).context("serialize goal state")?;
    std::fs::write(&path, json).with_context(|| format!("write goal state {}", path.display()))?;
    Ok(path)
}

/// Load goal state from session file (Task 014)
pub(crate) fn load_goal_state(session_root: &PathBuf) -> Option<GoalState> {
    let path = session_root.join("goal_state.json");
    if !path.exists() {
        return None;
    }
    let json = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}
