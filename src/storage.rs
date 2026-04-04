//! @efficiency-role: infra-adapter

use crate::*;

fn write_bytes_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    let tmp_name = format!(
        ".{}.tmp-{}-{}",
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("config"),
        std::process::id(),
        now.as_nanos()
    );
    let tmp_path = path.with_file_name(tmp_name);
    std::fs::write(&tmp_path, bytes)
        .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
    std::fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "Failed to atomically replace {} with {}",
            path.display(),
            tmp_path.display()
        )
    })?;
    Ok(())
}

pub(crate) fn load_agent_config(path: &PathBuf) -> Result<Profile> {
    let bytes = std::fs::read(&path)
        .with_context(|| format!("Failed to read config file at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("config file is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_global_config(path: &PathBuf, cfg: &GlobalConfig) -> Result<()> {
    let s = toml::to_string_pretty(cfg).context("Failed to serialize global config toml")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}

pub(crate) fn load_global_config(path: &PathBuf) -> Result<GlobalConfig> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read global config at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("global config is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_agent_config(path: &PathBuf, p: &Profile) -> Result<()> {
    let s = toml::to_string_pretty(p).context("Failed to serialize config toml")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}

pub(crate) fn save_router_calibration(path: &PathBuf, c: &RouterCalibration) -> Result<()> {
    let s = toml::to_string_pretty(c).context("Failed to serialize router calibration toml")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}

pub(crate) fn load_router_calibration(path: &PathBuf) -> Result<RouterCalibration> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read router calibration at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("router calibration is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_model_behavior_profile(
    path: &PathBuf,
    profile: &ModelBehaviorProfile,
) -> Result<()> {
    let s = toml::to_string_pretty(profile).context("Failed to serialize model behavior toml")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}

pub(crate) fn load_model_behavior_profile(path: &PathBuf) -> Result<ModelBehaviorProfile> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read model behavior at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("model behavior is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_active_manifest(path: &PathBuf, m: &ActiveManifest) -> Result<()> {
    let s = toml::to_string_pretty(m).context("Failed to serialize active manifest toml")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}

pub(crate) fn load_active_manifest(path: &PathBuf) -> Result<ActiveManifest> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read active manifest at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("active manifest is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_tune_run_manifest(path: &PathBuf, m: &TuneRunManifest) -> Result<()> {
    let s = toml::to_string_pretty(m).context("Failed to serialize tune run manifest toml")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}

pub(crate) fn save_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let s = serde_json::to_string_pretty(value).context("Failed to serialize json")?;
    write_bytes_atomically(path, s.as_bytes())?;
    Ok(())
}
