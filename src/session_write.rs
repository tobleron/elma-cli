//! @efficiency-role: infra-adapter
//!
//! Session - Write Helpers
//!
//! All session artifacts (shell actions, plans, decisions, gate-why logs)
//! are written to artifacts/ under the session root.  Thinking logs are
//! appended to thinking.jsonl as streaming JSON Lines records.

use crate::intel_units::TurnSummaryOutput;
use crate::*;
use std::collections::HashSet;

// ── session.json centralized read/write ────────────────────────────────

/// Load the full session.json as a generic Value.
/// Returns `json!({})` if the file doesn't exist yet.
pub(crate) fn load_session_doc(session_root: &Path) -> serde_json::Value {
    let path = session_root.join("session.json");
    if !path.exists() {
        return serde_json::json!({});
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| {
            tracing::warn!("corrupt session.json at {}, starting fresh", path.display());
            serde_json::json!({})
        })
}

/// Read session.json, apply a mutation via `f`, and atomically write back.
/// `f` receives a `&mut Value` that always starts fresh from disk, so concurrent
/// callers don't clobber each other's keys.
pub(crate) fn mutate_session_doc(
    session_root: &Path,
    f: impl FnOnce(&mut serde_json::Value),
) -> Result<PathBuf> {
    let path = session_root.join("session.json");
    let mut doc = load_session_doc(session_root);
    f(&mut doc);

    // Ensure schema_version is set
    if !doc.get("schema_version").is_some() {
        doc["schema_version"] = serde_json::json!(2);
    }

    let json = serde_json::to_string_pretty(&doc)
        .with_context(|| format!("serialize {}", path.display()))?;

    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, &json).with_context(|| format!("write {}", tmp.display()))?;
    std::fs::rename(&tmp, &path).with_context(|| format!("rename to {}", path.display()))?;
    Ok(path)
}

// ── shell actions & output ────────────────────────────────────────────

pub(crate) fn write_shell_action(artifacts_dir: &PathBuf, cmd_line: &str) -> Result<PathBuf> {
    let n = next_shell_seq(artifacts_dir)?;
    let path = artifacts_dir.join(format!("{n:03}.sh"));
    std::fs::write(&path, format!("{cmd_line}\n"))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn write_shell_output(
    artifacts_dir: &PathBuf,
    seq_path: &PathBuf,
    output: &str,
) -> Result<PathBuf> {
    let stem = seq_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "000".to_string());
    let path = artifacts_dir.join(format!("{stem}.out"));
    std::fs::write(&path, output).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

// ── plans ─────────────────────────────────────────────────────────────

pub(crate) fn write_plan_file(artifacts_dir: &PathBuf, content: &str) -> Result<PathBuf> {
    let n = next_plan_seq(artifacts_dir)?;
    let path = artifacts_dir.join(format!("plan_{n:03}.md"));
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn append_master_link(
    artifacts_dir: &PathBuf,
    plan_path: &PathBuf,
    title: &str,
) -> Result<()> {
    let master = artifacts_dir.join("_master.md");
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

// ── decisions ─────────────────────────────────────────────────────────

pub(crate) fn write_decision(artifacts_dir: &PathBuf, word: &str) -> Result<PathBuf> {
    let n = next_decision_seq(artifacts_dir)?;
    let path = artifacts_dir.join(format!("{n:03}.txt"));
    std::fs::write(&path, format!("{}\n", word.trim()))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

// ── gate-why (tune) ───────────────────────────────────────────────────

pub(crate) fn write_gate_why(artifacts_dir: &PathBuf, text: &str) -> Result<PathBuf> {
    let n = next_gate_why_seq(artifacts_dir)?;
    let path = artifacts_dir.join(format!("gate_why_{n:03}.txt"));
    std::fs::write(&path, text.trim().to_string() + "\n")
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

// ── thinking logs → thinking.jsonl ────────────────────────────────────

/// Append a streaming thinking/reasoning record to thinking.jsonl.
/// Each record: {"turn": N, "timestamp_unix_s": T, "content": "..."}
pub(crate) fn write_thinking_log(
    session_root: &Path,
    seq: u32,
    thinking_content: &str,
) -> Result<PathBuf> {
    let path = session_root.join("thinking.jsonl");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let record = serde_json::json!({
        "turn": seq,
        "timestamp_unix_s": now,
        "content": thinking_content,
    });
    let line = serde_json::to_string(&record).context("serialize thinking record")?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?
        .write_all(format!("{}\n", line).as_bytes())
        .with_context(|| format!("append {}", path.display()))?;
    Ok(path)
}

// ── goal state → session.json ─────────────────────────────────────────

/// Write goal state into the session metadata file (session.json).
pub(crate) fn save_goal_state(session_root: &PathBuf, goal_state: &GoalState) -> Result<PathBuf> {
    mutate_session_doc(session_root, |doc| {
        doc["goal_state"] = serde_json::to_value(goal_state)
            .expect("serialize goal state");
    })
}

/// Load goal state from session.json.
pub(crate) fn load_goal_state(session_root: &PathBuf) -> Option<GoalState> {
    let doc = load_session_doc(session_root);
    doc.get("goal_state")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
}

// ── turn summaries → summaries/*.md ───────────────────────────────────

/// Write a turn summary as a markdown artifact in the session's summaries/ directory.
pub(crate) fn write_summary_markdown(
    session_root: &Path,
    turn_number: usize,
    timestamp: &str,
    model: &str,
    session_id: &str,
    narrative: &str,
    status: &str,
    tools_used: &[String],
    errors: &[String],
) {
    let summaries_dir = session_root.join("summaries");
    if let Err(e) = std::fs::create_dir_all(&summaries_dir) {
        tracing::warn!("Failed to create summaries dir '{}': {}", summaries_dir.display(), e);
        return;
    }

    // Sanitize timestamp for filename
    let file_ts = timestamp.replace(':', "-").replace('T', "_");
    let filename = format!("{}_summary_{}.md", file_ts, turn_number);
    let filepath = summaries_dir.join(&filename);

    let mut content = String::new();
    content.push_str("---\n");
    content.push_str(&format!("timestamp: {}\n", timestamp));
    content.push_str(&format!("session: {}\n", session_id));
    content.push_str(&format!("model: {}\n", model));
    content.push_str(&format!("turn: {}\n", turn_number));
    content.push_str(&format!("status: {}\n", status));
    if !tools_used.is_empty() {
        content.push_str(&format!("tools: [{}]\n", tools_used.join(", ")));
    }
    if !errors.is_empty() {
        content.push_str(&format!("errors: [{}]\n", errors.join(", ")));
    }
    content.push_str("---\n\n");
    content.push_str(narrative);
    content.push('\n');

    match std::fs::write(&filepath, &content) {
        Ok(_) => tracing::debug!("Wrote summary markdown: {}", filepath.display()),
        Err(e) => tracing::warn!("Failed to write summary '{}': {}", filepath.display(), e),
    }
}

// ── turn summaries → session.json ─────────────────────────────────────

/// Write turn summary into session.json under "turn_summaries".
pub(crate) fn save_turn_summary(
    session_root: &Path,
    turn_number: usize,
    summary: &TurnSummaryOutput,
) -> Result<()> {
    mutate_session_doc(session_root, |doc| {
        if doc.get("turn_summaries").is_none() {
            doc["turn_summaries"] = serde_json::json!({});
        }
        doc["turn_summaries"][format!("turn_{}", turn_number)] =
            serde_json::to_value(summary).expect("serialize turn summary");
    })?;
    Ok(())
}

/// Load the most recent pending (unapplied) turn summary from session.json.
pub(crate) fn load_pending_turn_summary(
    session_root: &Path,
) -> Result<Option<(usize, TurnSummaryOutput)>> {
    let session_data = load_session_doc(session_root);
    let summaries = match session_data.get("turn_summaries") {
        Some(s) => s,
        None => return Ok(None),
    };
    let applied = match session_data.get("applied_summaries") {
        Some(s) => serde_json::from_value::<Vec<usize>>(s.clone()).unwrap_or_default(),
        None => Vec::new(),
    };
    let applied_set: HashSet<usize> = applied.into_iter().collect();

    let mut entries: Vec<(usize, TurnSummaryOutput)> = Vec::new();
    if let Some(obj) = summaries.as_object() {
        for (key, val) in obj {
            if let Some(turn_num) = key
                .strip_prefix("turn_")
                .and_then(|s| s.parse::<usize>().ok())
            {
                if let Ok(summary) = serde_json::from_value::<TurnSummaryOutput>(val.clone()) {
                    entries.push((turn_num, summary));
                }
            }
        }
    }
    entries.sort_by_key(|(n, _)| *n);
    for (turn_num, summary) in entries.into_iter().rev() {
        if !applied_set.contains(&turn_num) {
            return Ok(Some((turn_num, summary)));
        }
    }
    Ok(None)
}

/// Mark a turn summary as applied.
pub(crate) fn mark_summary_applied(session_root: &Path, turn_number: usize) -> Result<()> {
    mutate_session_doc(session_root, |doc| {
        let mut applied: Vec<usize> = doc
            .get("applied_summaries")
            .and_then(|s| serde_json::from_value(s.clone()).ok())
            .unwrap_or_default();
        if !applied.contains(&turn_number) {
            applied.push(turn_number);
            applied.sort_unstable();
            doc["applied_summaries"] = serde_json::to_value(&applied).expect("serialize applied");
        }
    })?;
    Ok(())
}

// ── session.md append ─────────────────────────────────────────────────

/// Entry kinds for the session markdown transcript.
pub(crate) enum MdEntry {
    User { content: String },
    Assistant { content: String },
    Thinking { content: String },
    ToolStart { name: String, input: String },
    ToolProgress { name: String, message: String },
    ToolResult { name: String, success: bool, output: String, duration_ms: Option<u64> },
    Meta { label: String, detail: String },
}

/// Append a formatted entry to `session.md` under the session root.
/// Creates the file if it does not exist.
pub(crate) fn append_session_markdown(session_root: &Path, entry: &MdEntry) {
    let path = session_root.join("session.md");
    let mut line = String::new();
    let ts = chrono::Local::now().format("%H:%M:%S").to_string();
    match entry {
        MdEntry::User { content } => {
            line = format!("**[{}] USER:** {}\n\n", ts, content);
        }
        MdEntry::Assistant { content } => {
            line = format!("**[{}] ELMA:** {}\n\n", ts, content);
        }
        MdEntry::Thinking { content } => {
            // Collapse thinking into one line in markdown
            let preview: String = content.chars().take(200).collect();
            let suffix = if content.len() > 200 { "…" } else { "" };
            line = format!("> {} _thinking…_{}\n\n", preview, suffix);
        }
        MdEntry::ToolStart { name, input } => {
            let input_preview: String = input.chars().take(120).collect();
            let suffix = if input.len() > 120 { "…" } else { "" };
            line = format!("> `{name}` {input_preview}{suffix}\n");
        }
        MdEntry::ToolProgress { name, message } => {
            line = format!("> `{name}` {message}\n");
        }
        MdEntry::ToolResult { name, success, output, duration_ms } => {
            let status = if *success { "✓" } else { "✗" };
            let preview: String = output.chars().take(200).collect();
            let suffix = if output.len() > 200 { "…" } else { "" };
            let dur = match duration_ms {
                Some(ms) if *ms > 0 => format!(" ({:.1}s)", *ms as f64 / 1000.0),
                _ => String::new(),
            };
            line = format!("> `{name}` {status}{dur}: `{preview}{suffix}`\n\n");
        }
        MdEntry::Meta { label, detail } => {
            line = format!("> **{label}:** {detail}\n");
        }
    };
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, line.as_bytes()))
    {
        tracing::warn!("Failed to append to session.md: {}", e);
    }
}

// ── sequence helpers ──────────────────────────────────────────────────

fn next_shell_seq(artifacts_dir: &Path) -> Result<u32> {
    next_seq_in_dir(artifacts_dir, ".sh")
}

fn next_plan_seq(artifacts_dir: &Path) -> Result<u32> {
    next_seq_in_dir(artifacts_dir, "plan_")
}

fn next_decision_seq(artifacts_dir: &Path) -> Result<u32> {
    next_seq_in_dir(artifacts_dir, ".txt")
}

fn next_gate_why_seq(artifacts_dir: &Path) -> Result<u32> {
    next_seq_in_dir(artifacts_dir, "gate_why_")
}

fn next_seq_in_dir(dir: &Path, prefix: &str) -> Result<u32> {
    let mut max: u32 = 0;
    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with(prefix) || name.ends_with(prefix) {
                // Extract numeric portion
                let digits: String = name
                    .chars()
                    .skip_while(|c| !c.is_ascii_digit())
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if let Ok(n) = digits.parse::<u32>() {
                    max = max.max(n);
                }
            }
        }
    }
    Ok(max + 1)
}
