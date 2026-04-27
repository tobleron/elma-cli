//! @efficiency-role: util-pure

use crate::dirs::ElmaPaths;
use crate::*;

pub(crate) fn repo_root() -> Result<PathBuf> {
    std::env::current_dir().context("Failed to get current directory")
}

pub(crate) fn config_root_path(config_root: &str) -> Result<PathBuf> {
    if config_root == "config" {
        if let Some(paths) = ElmaPaths::new() {
            let path = paths.config_dir().to_path_buf();
            let _ = std::fs::create_dir_all(&path);
            return Ok(path);
        }
    }
    Ok(repo_root()?.join(config_root))
}

pub(crate) fn sessions_root_path(sessions_root: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join(sessions_root))
}

pub(crate) fn global_config_path(config_root: &Path) -> PathBuf {
    config_root.join("global.toml")
}

pub(crate) fn discover_saved_base_url(
    config_root: &Path,
    model_hint: Option<&str>,
) -> Result<Option<String>> {
    let global_path = global_config_path(config_root);
    if global_path.exists() {
        let cfg = load_global_config(&global_path)?;
        let url = cfg.base_url.trim();
        if !url.is_empty() {
            return Ok(Some(url.to_string()));
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(model_id) = model_hint {
        let hinted = config_root.join(sanitize_model_folder_name(model_id));
        if hinted.is_dir() {
            candidates.push(hinted);
        }
    }

    if let Ok(rd) = std::fs::read_dir(config_root) {
        let mut dirs: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            if !candidates.contains(&dir) {
                candidates.push(dir);
            }
        }
    }

    for dir in candidates {
        let elma_cfg_path = dir.join("_elma.config");
        if elma_cfg_path.exists() {
            if let Ok(cfg) = load_agent_config(&elma_cfg_path) {
                let url = cfg.base_url.trim();
                if !url.is_empty() {
                    return Ok(Some(url.to_string()));
                }
            }
        }

        let router_cal_path = dir.join("router_calibration.toml");
        if router_cal_path.exists() {
            if let Ok(cal) = load_router_calibration(&router_cal_path) {
                let url = cal.base_url.trim();
                if !url.is_empty() {
                    return Ok(Some(url.to_string()));
                }
            }
        }
    }

    Ok(None)
}

pub(crate) fn resolve_base_url(
    config_root: &Path,
    explicit: Option<&str>,
    model_hint: Option<&str>,
) -> Result<(String, &'static str)> {
    if let Some(url) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok((url.to_string(), "cli_or_env"));
    }
    if let Some(url) = discover_saved_base_url(config_root, model_hint)? {
        return Ok((url, "saved_config"));
    }
    Err(crate::diagnostics::ElmaDiagnostic::MissingBaseUrl.into())
}

pub(crate) fn sanitize_model_folder_name(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}
