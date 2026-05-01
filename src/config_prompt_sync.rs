//! @efficiency-role: infra-config
//!
//! Sync profile `system_prompt` fields in a config tree to the canonical prompts in code.
//!
//! This is an offline operation intended for:
//! - CI/certification gates
//! - developer maintenance (eliminating drift in checked-in config snapshots)

use crate::*;
use std::path::{Path, PathBuf};

pub(crate) struct PromptSyncReport {
    pub(crate) scanned: usize,
    pub(crate) updated: usize,
}

pub(crate) fn sync_canonical_prompts_in_tree(
    root: &Path,
    dry_run: bool,
) -> Result<PromptSyncReport> {
    if !root.exists() {
        anyhow::bail!("config root does not exist: {}", root.display());
    }

    let mut files = Vec::new();
    collect_toml_files(&root.join("defaults"), &mut files)?;

    if let Ok(rd) = std::fs::read_dir(root) {
        for entry in rd.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.ends_with(".DISABLED") {
                continue;
            }
            if name.ends_with(".gguf") {
                collect_toml_files(&path, &mut files)?;
            }
        }
    }

    let mut scanned = 0usize;
    let mut updated = 0usize;
    let defaults_dir = root.join("defaults");
    for path in files {
        let p = path.to_string_lossy();
        if p.contains("/tune/") || p.contains("\\tune\\") {
            continue;
        }
        let Ok(mut profile) = load_agent_config(&path) else {
            // Not a profile TOML (e.g., router_calibration.toml). Skip.
            continue;
        };
        scanned += 1;
        let mut dirty = apply_canonical_system_prompt(&mut profile);

        // For non-managed profiles, keep the semantic contract aligned to `config/defaults/`
        // so model-specific snapshots cannot drift back to JSON-output contracts.
        if !dirty {
            if let Some(file_name) = path.file_name() {
                let default_path = defaults_dir.join(file_name);
                if default_path.exists() {
                    if let Ok(default_profile) = load_agent_config(&default_path) {
                        if default_profile.system_prompt != profile.system_prompt {
                            profile.system_prompt = default_profile.system_prompt;
                            dirty = true;
                        }
                    }
                }
            }
        }

        if dirty {
            updated += 1;
            if !dry_run {
                save_agent_config(&path, &profile)?;
            }
        }
    }

    Ok(PromptSyncReport { scanned, updated })
}

fn collect_toml_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_toml_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("toml") {
            out.push(path);
        }
    }
    Ok(())
}
