use crate::*;

pub(crate) fn load_agent_config(path: &PathBuf) -> Result<Profile> {
    let bytes = std::fs::read(&path)
        .with_context(|| format!("Failed to read config file at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("config file is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_global_config(path: &PathBuf, cfg: &GlobalConfig) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(cfg).context("Failed to serialize global config toml")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn load_global_config(path: &PathBuf) -> Result<GlobalConfig> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read global config at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("global config is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_agent_config(path: &PathBuf, p: &Profile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(p).context("Failed to serialize config toml")?;
    std::fs::write(&path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn save_router_calibration(path: &PathBuf, c: &RouterCalibration) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(c).context("Failed to serialize router calibration toml")?;
    std::fs::write(&path, s).with_context(|| format!("Failed to write {}", path.display()))?;
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
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(profile).context("Failed to serialize model behavior toml")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn load_model_behavior_profile(path: &PathBuf) -> Result<ModelBehaviorProfile> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read model behavior at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("model behavior is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_active_manifest(path: &PathBuf, m: &ActiveManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(m).context("Failed to serialize active manifest toml")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn load_active_manifest(path: &PathBuf) -> Result<ActiveManifest> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read active manifest at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("active manifest is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

pub(crate) fn save_tune_run_manifest(path: &PathBuf, m: &TuneRunManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(m).context("Failed to serialize tune run manifest toml")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub(crate) fn save_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = serde_json::to_string_pretty(value).context("Failed to serialize json")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}
