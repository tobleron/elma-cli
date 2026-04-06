//! @efficiency-role: util-pure

use crate::*;

pub(crate) fn read_expected_line(s: &str) -> Option<String> {
    for line in s.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("expected:") {
            let t = rest.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

pub(crate) fn parse_three_tags(s: &str) -> [String; 3] {
    let mut out: Vec<String> = Vec::new();
    for line in s.lines() {
        let w = line.trim().split_whitespace().next().unwrap_or("").trim();
        if w.is_empty() {
            continue;
        }
        // keep only letters
        let cleaned: String = w.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if cleaned.is_empty() {
            continue;
        }
        out.push(cleaned);
        if out.len() == 3 {
            break;
        }
    }
    while out.len() < 3 {
        out.push("Unknown".to_string());
    }
    [out[0].clone(), out[1].clone(), out[2].clone()]
}

pub(crate) fn load_intention_mapping(
    model_cfg_dir: &PathBuf,
) -> Option<Vec<(String, [String; 3])>> {
    let path = model_cfg_dir.join("intention_mapping.txt");
    let txt = std::fs::read_to_string(path).ok()?;
    let mut out = Vec::new();
    for line in txt.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let Some((expected, tags)) = l.split_once(':') else {
            continue;
        };
        let expected = expected.trim().to_string();
        let parts: Vec<String> = tags
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 3 {
            out.push((
                expected,
                [parts[0].clone(), parts[1].clone(), parts[2].clone()],
            ));
        }
    }
    Some(out)
}

pub(crate) fn scenario_helper(
    intent_word: &str,
    mapping: &[(String, [String; 3])],
) -> (Option<String>, f64) {
    let w = intent_word.trim();
    if w.is_empty() {
        return (None, 0.0);
    }
    let wl = w.to_lowercase();
    let mut best: Option<(String, f64)> = None;
    for (expected, tags) in mapping {
        let mut score: f64 = 0.0;
        for t in tags {
            let tl = t.to_lowercase();
            if tl == wl {
                score = score.max(0.9);
            }
            // soft match for variants (Listing vs List)
            if tl.starts_with(&wl) || wl.starts_with(&tl) {
                score = score.max(0.75);
            }
        }
        if score == 0.0 {
            // weak sentence keyword match
            if expected.to_lowercase().contains(&wl) {
                score = 0.6;
            }
        }
        if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
            best = Some((expected.clone(), score));
        }
    }
    if let Some((e, s)) = best {
        (Some(e), s)
    } else {
        (None, 0.0)
    }
}

pub(crate) fn list_intention_scenario_paths() -> Result<Vec<PathBuf>> {
    let dir = repo_root()?.join("scenarios").join("intention");
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("scenario_") && s.ends_with(".md"))
                .unwrap_or(false)
        })
        .collect();
    out.sort();
    Ok(out)
}

pub(crate) fn load_calibration_manifest() -> Result<CalibrationManifest> {
    let root = repo_root()?.join("scenarios");
    let mut scenarios = Vec::new();
    for suite in ["intention", "stress"] {
        let path = root.join(suite).join("manifest.toml");
        if !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path).with_context(|| {
            format!("Failed to read calibration manifest at {}", path.display())
        })?;
        let s = String::from_utf8(bytes).context("calibration manifest is not valid UTF-8")?;
        let mut manifest: CalibrationManifest =
            toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))?;
        for scenario in &mut manifest.scenarios {
            if scenario.suite.trim().is_empty() {
                scenario.suite = suite.to_string();
            }
        }
        scenarios.extend(manifest.scenarios);
    }
    Ok(CalibrationManifest {
        version: 1,
        scenarios,
    })
}

fn load_manifest_at(path: &Path, default_suite: &str) -> Result<CalibrationManifest> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read calibration manifest at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("calibration manifest is not valid UTF-8")?;
    let mut manifest: CalibrationManifest =
        toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))?;
    for scenario in &mut manifest.scenarios {
        if scenario.suite.trim().is_empty() {
            scenario.suite = default_suite.to_string();
        }
    }
    Ok(manifest)
}

pub(crate) fn load_tuning_manifest(
    tune_mode: &str,
    runtime_safe_only: bool,
) -> Result<CalibrationManifest> {
    if tune_mode == "quick" {
        let root = repo_root()?.join("scenarios").join("tune");
        let path = root.join("quick_manifest.toml");
        let manifest = load_manifest_at(&path, "tune")?;
        if manifest.scenarios.len() != 5 {
            anyhow::bail!(
                "Quick tuning corpus must contain exactly 5 scenarios, found {} in {}",
                manifest.scenarios.len(),
                path.display()
            );
        }
        return Ok(manifest);
    }

    let mut manifest = load_calibration_manifest()?;
    if runtime_safe_only {
        manifest.scenarios.retain(|scenario| scenario.runtime_safe);
    }
    Ok(manifest)
}

pub(crate) fn calibration_scenario_path(root: &Path, scenario: &CalibrationScenario) -> PathBuf {
    let suite = if scenario.suite.trim().is_empty() {
        "intention"
    } else {
        scenario.suite.as_str()
    };
    root.join("scenarios").join(suite).join(&scenario.file)
}

pub(crate) fn parse_scenario_dialog(s: &str) -> (String, Vec<ChatMessage>) {
    let mut messages = Vec::new();
    for line in s.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("user:") {
            messages.push(ChatMessage::simple("user", &rest.trim().to_string()));
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("elma:") {
            messages.push(ChatMessage::simple("assistant", &rest.trim().to_string()));
        }
    }
    let user_message = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_else(|| s.trim().to_string());
    (user_message, messages)
}
