//! @efficiency-role: infra-adapter
//!
//! Session - Write Helpers

use crate::intel_units::TurnSummaryOutput;
use crate::*;
use std::collections::HashSet;

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

// ============================================================================
// Turn Summary Persistence (Task 310)
// ============================================================================

/// Write turn summary to session/summaries/turn_N_summary.json
pub(crate) fn save_turn_summary(
    session_root: &Path,
    turn_number: usize,
    summary: &TurnSummaryOutput,
) -> Result<()> {
    let summaries_dir = session_root.join("summaries");
    std::fs::create_dir_all(&summaries_dir)?;
    let path = summaries_dir.join(format!("turn_{}_summary.json", turn_number));
    let json = serde_json::to_string_pretty(summary)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Load the most recent pending (unapplied) turn summary
pub(crate) fn load_pending_turn_summary(
    session_root: &Path,
) -> Result<Option<(usize, TurnSummaryOutput)>> {
    let summaries_dir = session_root.join("summaries");
    if !summaries_dir.exists() {
        return Ok(None);
    }
    let applied = load_applied_summaries(session_root)?;
    let mut entries: Vec<_> = std::fs::read_dir(&summaries_dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            let stem = path.file_stem()?.to_string_lossy().to_string();
            let ext = path.extension()?.to_string_lossy().to_string();
            if ext != "json" {
                return None;
            }
            let turn_num = stem
                .strip_prefix("turn_")?
                .strip_suffix("_summary")?
                .parse::<usize>()
                .ok()?;
            Some((turn_num, path))
        })
        .collect();
    entries.sort_by_key(|(n, _)| *n);
    for (turn_num, path) in entries.into_iter().rev() {
        if !applied.contains(&turn_num) {
            let content = std::fs::read_to_string(&path)?;
            let summary: TurnSummaryOutput = serde_json::from_str(&content)?;
            return Ok(Some((turn_num, summary)));
        }
    }
    Ok(None)
}

/// Mark a turn summary as applied so it won't be re-injected
pub(crate) fn mark_summary_applied(session_root: &Path, turn_number: usize) -> Result<()> {
    let mut applied = load_applied_summaries(session_root)?;
    applied.insert(turn_number);
    let path = session_root.join("summaries").join("applied.json");
    let json = serde_json::to_string(&applied)?;
    std::fs::write(&path, json)?;
    Ok(())
}

fn load_applied_summaries(session_root: &Path) -> Result<HashSet<usize>> {
    let path = session_root.join("summaries").join("applied.json");
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        Ok(serde_json::from_str(&content)?)
    } else {
        Ok(HashSet::new())
    }
}
