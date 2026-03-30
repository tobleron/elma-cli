//! @efficiency-role: util-pure
//!
//! Session - Sequence Helpers

use crate::*;

pub(crate) fn next_shell_seq(shell_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(shell_dir).with_context(|| format!("read_dir {}", shell_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        let digits = name
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        if digits.len() >= 3 {
            if let Ok(n) = digits[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn next_artifact_seq(artifacts_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in std::fs::read_dir(artifacts_dir)
        .with_context(|| format!("read_dir {}", artifacts_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        let digits = name
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();
        if digits.len() >= 3 {
            if let Ok(n) = digits[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn reserve_artifact_path(
    artifacts_dir: &PathBuf,
    kind: &str,
    extension: &str,
) -> Result<(String, PathBuf)> {
    let n = next_artifact_seq(artifacts_dir)?;
    let artifact_id = format!("a_{n:03}");
    let safe_kind = kind
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let ext = extension.trim_start_matches('.');
    let path = artifacts_dir.join(format!("{n:03}_{safe_kind}.{ext}"));
    Ok((artifact_id, path))
}

pub(crate) fn next_plan_seq(plans_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(plans_dir).with_context(|| format!("read_dir {}", plans_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("plan_") {
            if rest.len() >= 7 && rest.as_bytes()[3] == b'.' {
                let digits = &rest[..3];
                if let Ok(n) = digits.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn next_decision_seq(decisions_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in std::fs::read_dir(decisions_dir)
        .with_context(|| format!("read_dir {}", decisions_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if name.len() >= 7 && name.ends_with(".txt") {
            if let Ok(n) = name[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn next_gate_why_seq(tune_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(tune_dir).with_context(|| format!("read_dir {}", tune_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("gate_why_") {
            if rest.len() >= 7 && rest.ends_with(".txt") {
                if let Ok(n) = rest[..3].parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

pub(crate) fn append_artifact_manifest_record(
    artifacts_dir: &PathBuf,
    record: &ArtifactRecord,
) -> Result<PathBuf> {
    let path = artifacts_dir.join("manifest.jsonl");
    let line = serde_json::to_string(record).context("serialize artifact record")?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open {}", path.display()))?
        .write_all(format!("{line}\n").as_bytes())
        .with_context(|| format!("append {}", path.display()))?;
    Ok(path)
}

pub(crate) fn append_thinking_to_manifest(
    artifacts_dir: &PathBuf,
    step_id: &str,
    thinking_path: &PathBuf,
    bytes_written: u64,
) -> Result<PathBuf> {
    let manifest_path = artifacts_dir.join("thinking_manifest.jsonl");
    let record = serde_json::json!({
        "step_id": step_id,
        "thinking_path": thinking_path.display().to_string(),
        "bytes": bytes_written,
        "created_unix_s": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
    let line = serde_json::to_string(&record).context("serialize thinking record")?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&manifest_path)
        .with_context(|| format!("open thinking manifest {}", manifest_path.display()))?
        .write_all(format!("{line}\n").as_bytes())
        .with_context(|| format!("append thinking manifest {}", manifest_path.display()))?;
    Ok(manifest_path)
}
