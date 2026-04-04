//! @efficiency-role: util-pure

use crate::*;

pub(crate) fn new_tune_run_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    Ok(format!("run_{:010}_{}", now.as_secs(), now.subsec_nanos()))
}

pub(crate) fn now_unix_s() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?
        .as_secs())
}

pub(crate) fn model_baseline_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("baseline")
}

pub(crate) fn model_fallback_last_active_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("fallback").join("last_active")
}

pub(crate) fn model_tune_runs_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("tune").join("runs")
}

pub(crate) fn model_active_manifest_path(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("tune").join("active_manifest.toml")
}

pub(crate) fn model_formula_memory_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("formula_memory")
}

pub(crate) fn write_profile_specs_to_dir(dir: &Path, specs: &[(String, Profile)]) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    for (filename, profile) in specs {
        save_agent_config(&dir.join(filename), profile)?;
    }
    Ok(())
}

pub(crate) fn ensure_baseline_profile_set(
    model_cfg_dir: &Path,
    base_url: &str,
    model: &str,
) -> Result<PathBuf> {
    let dir = model_baseline_dir(model_cfg_dir);
    if !dir.exists() {
        let specs = managed_profile_specs(base_url, model);
        write_profile_specs_to_dir(&dir, &specs)?;
    }
    sync_profile_dir_base_url_and_model(&dir, base_url, model)?;
    Ok(dir)
}

pub(crate) fn copy_profile_set(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dst_dir).with_context(|| format!("mkdir {}", dst_dir.display()))?;
    for filename in managed_profile_file_names() {
        let src = src_dir.join(&filename);
        if !src.exists() {
            continue;
        }
        let dst = dst_dir.join(&filename);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

pub(crate) fn snapshot_active_profile_set(model_cfg_dir: &Path, snapshot_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(snapshot_dir)
        .with_context(|| format!("mkdir {}", snapshot_dir.display()))?;
    for filename in managed_profile_file_names() {
        let src = model_cfg_dir.join(&filename);
        if !src.exists() {
            continue;
        }
        let dst = snapshot_dir.join(&filename);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

pub(crate) fn sync_profile_dir_base_url_and_model(
    dir: &Path,
    base_url: &str,
    model: &str,
) -> Result<()> {
    for filename in managed_profile_file_names() {
        let path = dir.join(filename);
        if !path.exists() {
            continue;
        }
        let mut profile = load_agent_config(&path)?;
        profile.base_url = base_url.to_string();
        profile.model = model.to_string();
        apply_canonical_system_prompt(&mut profile);
        save_agent_config(&path, &profile)?;
    }
    Ok(())
}

pub(crate) fn activate_profile_set(
    model_cfg_dir: &Path,
    src_dir: &Path,
    base_url: &str,
    model: &str,
    active_source: &str,
    active_run_id: Option<String>,
    final_score: f64,
    certified: bool,
    activation_reason: &str,
    baseline_score: f64,
) -> Result<()> {
    let fallback_dir = model_fallback_last_active_dir(model_cfg_dir);
    snapshot_active_profile_set(model_cfg_dir, &fallback_dir)?;
    copy_profile_set(src_dir, model_cfg_dir)?;
    sync_profile_dir_base_url_and_model(model_cfg_dir, base_url, model)?;
    let manifest = ActiveManifest {
        version: 1,
        model: model.to_string(),
        active_source: active_source.to_string(),
        active_run_id,
        activated_unix_s: now_unix_s()?,
        final_score,
        certified,
        restore_last_dir: fallback_dir.display().to_string(),
        restore_base_dir: model_baseline_dir(model_cfg_dir).display().to_string(),
        activation_reason: activation_reason.to_string(),
        baseline_score,
    };
    save_active_manifest(&model_active_manifest_path(model_cfg_dir), &manifest)?;
    Ok(())
}

pub(crate) fn load_recent_formula_memories(
    model_cfg_dir: &Path,
    limit: usize,
) -> Result<Vec<FormulaMemoryRecord>> {
    let dir = model_formula_memory_dir(model_cfg_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for path in std::fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
    {
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let s = String::from_utf8(bytes).context("formula memory is not valid UTF-8")?;
        if let Ok(record) = serde_json::from_str::<FormulaMemoryRecord>(&s) {
            if !record.disabled {
                out.push(record);
            }
        }
    }
    out.sort_by(|a, b| {
        let a_key = a.last_success_unix_s.max(a.created_unix_s);
        let b_key = b.last_success_unix_s.max(b.created_unix_s);
        b_key
            .cmp(&a_key)
            .then_with(|| b.success_count.cmp(&a.success_count))
            .then_with(|| a.failure_count.cmp(&b.failure_count))
    });
    out.truncate(limit);
    Ok(out)
}

pub(crate) fn save_formula_memory(
    model_cfg_dir: &Path,
    record: &FormulaMemoryRecord,
) -> Result<PathBuf> {
    let dir = model_formula_memory_dir(model_cfg_dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let path = dir.join(format!("{}.json", record.id));
    let body = serde_json::to_string_pretty(record).context("serialize formula memory")?;
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn load_formula_memory_by_id(
    model_cfg_dir: &Path,
    memory_id: &str,
) -> Result<Option<FormulaMemoryRecord>> {
    let path = model_formula_memory_dir(model_cfg_dir).join(format!("{memory_id}.json"));
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    let s = String::from_utf8(bytes).context("formula memory is not valid UTF-8")?;
    let record = serde_json::from_str::<FormulaMemoryRecord>(&s)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(record))
}

pub(crate) fn record_formula_memory_reuse(
    model_cfg_dir: &Path,
    memory_id: &str,
    success: bool,
    artifact_mode_capable: bool,
) -> Result<Option<FormulaMemoryRecord>> {
    let Some(mut record) = load_formula_memory_by_id(model_cfg_dir, memory_id)? else {
        return Ok(None);
    };
    let now = now_unix_s()?;
    if success {
        record.success_count = record.success_count.saturating_add(1);
        record.last_success_unix_s = now;
        record.artifact_mode_capable |= artifact_mode_capable;
    } else {
        record.failure_count = record.failure_count.saturating_add(1);
        record.last_failure_unix_s = now;
        if record.failure_count >= 3 {
            record.disabled = true;
        }
    }
    let _ = save_formula_memory(model_cfg_dir, &record)?;
    Ok(Some(record))
}

pub(crate) fn ensure_model_config_folder(
    config_root: &PathBuf,
    base_url: &str,
    model_id: &str,
) -> Result<PathBuf> {
    let folder = sanitize_model_folder_name(model_id);
    let dir = config_root.join(folder);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    for (filename, profile) in managed_profile_specs(base_url, model_id) {
        let path = dir.join(filename);
        if !path.exists() {
            save_agent_config(&path, &profile)?;
        }
    }
    let elma_path = dir.join("_elma.config");
    if !elma_path.exists() {
        save_agent_config(&elma_path, &default_elma_config(base_url, model_id))?;
    }
    let router_cal_path = dir.join("router_calibration.toml");
    if !router_cal_path.exists() {
        // Placeholder; real values written by --tune.
        save_router_calibration(
            &router_cal_path,
            &RouterCalibration {
                version: 1,
                model: model_id.to_string(),
                base_url: base_url.to_string(),
                n_probs: 64,
                supports_logprobs: false,
                routes: vec![
                    "CHAT".to_string(),
                    "WORKFLOW".to_string(),
                    "INSPECT".to_string(),
                    "EXECUTE".to_string(),
                    "PLAN".to_string(),
                    "MASTERPLAN".to_string(),
                    "DECIDE".to_string(),
                    "CAPABILITY_CHECK".to_string(),
                    "INFO_REQUEST".to_string(),
                    "ACTION_REQUEST".to_string(),
                ],
            },
        )?;
    }
    let orch_path = dir.join("orchestrator.toml");
    if !orch_path.exists() {
        save_agent_config(&orch_path, &default_orchestrator_config(base_url, model_id))?;
    }
    let critic_path = dir.join("critic.toml");
    if !critic_path.exists() {
        save_agent_config(&critic_path, &default_critic_config(base_url, model_id))?;
    }
    sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
    let _ = ensure_baseline_profile_set(&dir, base_url, model_id)?;

    Ok(dir)
}

pub(crate) fn maybe_upgrade_system_prompt(
    profile: &mut Profile,
    expected_name: &str,
    patch: &str,
) -> bool {
    if profile.name != expected_name {
        return false;
    }
    if profile.system_prompt.contains(patch) {
        return false;
    }
    let _ = patch;
    apply_canonical_system_prompt(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_formula_root(label: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("elma_formula_test_{label}_{stamp}"))
    }

    #[test]
    fn formula_memory_reuse_updates_success_and_disables_after_failures() -> Result<()> {
        let root = temp_formula_root("reuse");
        std::fs::create_dir_all(&root)?;

        let record = FormulaMemoryRecord {
            id: "fm_test".to_string(),
            created_unix_s: 1,
            model_id: "model".to_string(),
            active_run_id: "run".to_string(),
            user_message: "msg".to_string(),
            route: "SHELL".to_string(),
            complexity: "INVESTIGATE".to_string(),
            formula: "inspect_reply".to_string(),
            objective: "obj".to_string(),
            title: "title".to_string(),
            program_signature: "shell:pwd | reply".to_string(),
            last_success_unix_s: 0,
            last_failure_unix_s: 0,
            success_count: 0,
            failure_count: 0,
            disabled: false,
            artifact_mode_capable: false,
        };
        save_formula_memory(&root, &record)?;

        let updated = record_formula_memory_reuse(&root, "fm_test", true, true)?
            .context("missing updated memory after success")?;
        assert_eq!(updated.success_count, 1);
        assert!(updated.artifact_mode_capable);
        assert!(!updated.disabled);

        let fail1 = record_formula_memory_reuse(&root, "fm_test", false, false)?
            .context("missing updated memory after fail1")?;
        let fail2 = record_formula_memory_reuse(&root, "fm_test", false, false)?
            .context("missing updated memory after fail2")?;
        let fail3 = record_formula_memory_reuse(&root, "fm_test", false, false)?
            .context("missing updated memory after fail3")?;

        assert_eq!(fail1.failure_count, 1);
        assert_eq!(fail2.failure_count, 2);
        assert_eq!(fail3.failure_count, 3);
        assert!(fail3.disabled);

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
}

pub(crate) fn replace_system_prompt_if_missing(
    profile: &mut Profile,
    expected_name: &str,
    must_contain: &str,
    replacement: String,
) -> bool {
    if profile.name != expected_name {
        return false;
    }
    if canonical_system_prompt(expected_name).is_some() {
        return apply_canonical_system_prompt(profile);
    }
    if profile.system_prompt.contains(must_contain) {
        return false;
    }
    profile.system_prompt = replacement;
    true
}
