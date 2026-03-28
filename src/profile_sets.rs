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

pub(crate) fn write_profile_specs_to_dir(dir: &Path, specs: &[(&str, Profile)]) -> Result<()> {
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
    Ok(dir)
}

pub(crate) fn copy_profile_set(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dst_dir).with_context(|| format!("mkdir {}", dst_dir.display()))?;
    for filename in managed_profile_file_names() {
        let src = src_dir.join(filename);
        if !src.exists() {
            continue;
        }
        let dst = dst_dir.join(filename);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

pub(crate) fn snapshot_active_profile_set(model_cfg_dir: &Path, snapshot_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(snapshot_dir)
        .with_context(|| format!("mkdir {}", snapshot_dir.display()))?;
    for filename in managed_profile_file_names() {
        let src = model_cfg_dir.join(filename);
        if !src.exists() {
            continue;
        }
        let dst = snapshot_dir.join(filename);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

pub(crate) fn sync_profile_dir_base_url_and_model(dir: &Path, base_url: &str, model: &str) -> Result<()> {
    for filename in managed_profile_file_names() {
        let path = dir.join(filename);
        if !path.exists() {
            continue;
        }
        let mut profile = load_agent_config(&path)?;
        profile.base_url = base_url.to_string();
        profile.model = model.to_string();
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
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths.reverse();
    let mut out = Vec::new();
    for path in paths.into_iter().take(limit) {
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let s = String::from_utf8(bytes).context("formula memory is not valid UTF-8")?;
        if let Ok(record) = serde_json::from_str::<FormulaMemoryRecord>(&s) {
            out.push(record);
        }
    }
    Ok(out)
}

pub(crate) fn save_formula_memory(model_cfg_dir: &Path, record: &FormulaMemoryRecord) -> Result<PathBuf> {
    let dir = model_formula_memory_dir(model_cfg_dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let path = dir.join(format!("{}.json", record.id));
    let body = serde_json::to_string_pretty(record).context("serialize formula memory")?;
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn ensure_model_config_folder(
    config_root: &PathBuf,
    base_url: &str,
    model_id: &str,
) -> Result<PathBuf> {
    let folder = sanitize_model_folder_name(model_id);
    let dir = config_root.join(folder);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;

    let elma_path = dir.join("_elma.config");
    if !elma_path.exists() {
        save_agent_config(&elma_path, &default_elma_config(base_url, model_id))?;
    }
    let intention_path = dir.join("intention.toml");
    if !intention_path.exists() {
        save_agent_config(
            &intention_path,
            &default_intention_config(base_url, model_id),
        )?;
    }
    let gate_path = dir.join("gate.toml");
    if !gate_path.exists() {
        save_agent_config(&gate_path, &default_gate_config(base_url, model_id))?;
    }
    let gate_why_path = dir.join("gate_why.toml");
    if !gate_why_path.exists() {
        save_agent_config(&gate_why_path, &default_gate_why_config(base_url, model_id))?;
    }
    let tooler_path = dir.join("tooler.toml");
    if !tooler_path.exists() {
        save_agent_config(&tooler_path, &default_tooler_config(base_url, model_id))?;
    }
    let planner_master_path = dir.join("planner_master.toml");
    if !planner_master_path.exists() {
        save_agent_config(
            &planner_master_path,
            &default_planner_master_config(base_url, model_id),
        )?;
    }
    let planner_path = dir.join("planner.toml");
    if !planner_path.exists() {
        save_agent_config(&planner_path, &default_planner_config(base_url, model_id))?;
    }
    let decider_path = dir.join("decider.toml");
    if !decider_path.exists() {
        save_agent_config(&decider_path, &default_decider_config(base_url, model_id))?;
    }
    let tune_path = dir.join("intention_tune.toml");
    if !tune_path.exists() {
        save_agent_config(
            &tune_path,
            &default_intention_tune_config(base_url, model_id),
        )?;
    }
    let action_type_path = dir.join("action_type.toml");
    if !action_type_path.exists() {
        save_agent_config(
            &action_type_path,
            &default_action_type_config(base_url, model_id),
        )?;
    }
    let router_path = dir.join("router.toml");
    if !router_path.exists() {
        save_agent_config(&router_path, &default_router_config(base_url, model_id))?;
    }
    let mode_router_path = dir.join("mode_router.toml");
    if !mode_router_path.exists() {
        save_agent_config(
            &mode_router_path,
            &default_mode_router_config(base_url, model_id),
        )?;
    }
    let speech_act_path = dir.join("speech_act.toml");
    if !speech_act_path.exists() {
        save_agent_config(
            &speech_act_path,
            &default_speech_act_config(base_url, model_id),
        )?;
    }
    let summarizer_path = dir.join("summarizer.toml");
    if !summarizer_path.exists() {
        save_agent_config(
            &summarizer_path,
            &default_summarizer_config(base_url, model_id),
        )?;
    }
    let formatter_path = dir.join("formatter.toml");
    if !formatter_path.exists() {
        save_agent_config(
            &formatter_path,
            &default_formatter_config(base_url, model_id),
        )?;
    }
    let calibration_judge_path = dir.join("calibration_judge.toml");
    if !calibration_judge_path.exists() {
        save_agent_config(
            &calibration_judge_path,
            &default_calibration_judge_config(base_url, model_id),
        )?;
    }
    let complexity_assessor_path = dir.join("complexity_assessor.toml");
    if !complexity_assessor_path.exists() {
        save_agent_config(
            &complexity_assessor_path,
            &default_complexity_assessor_config(base_url, model_id),
        )?;
    }
    let formula_selector_path = dir.join("formula_selector.toml");
    if !formula_selector_path.exists() {
        save_agent_config(
            &formula_selector_path,
            &default_formula_selector_config(base_url, model_id),
        )?;
    }
    let command_repair_path = dir.join("command_repair.toml");
    if !command_repair_path.exists() {
        save_agent_config(
            &command_repair_path,
            &default_command_repair_config(base_url, model_id),
        )?;
    }
    let scope_builder_path = dir.join("scope_builder.toml");
    if !scope_builder_path.exists() {
        save_agent_config(
            &scope_builder_path,
            &default_scope_builder_config(base_url, model_id),
        )?;
    }
    let evidence_compactor_path = dir.join("evidence_compactor.toml");
    if !evidence_compactor_path.exists() {
        save_agent_config(
            &evidence_compactor_path,
            &default_evidence_compactor_config(base_url, model_id),
        )?;
    }
    let artifact_classifier_path = dir.join("artifact_classifier.toml");
    if !artifact_classifier_path.exists() {
        save_agent_config(
            &artifact_classifier_path,
            &default_artifact_classifier_config(base_url, model_id),
        )?;
    }
    let result_presenter_path = dir.join("result_presenter.toml");
    if !result_presenter_path.exists() {
        save_agent_config(
            &result_presenter_path,
            &default_result_presenter_config(base_url, model_id),
        )?;
    }
    let claim_checker_path = dir.join("claim_checker.toml");
    if !claim_checker_path.exists() {
        save_agent_config(
            &claim_checker_path,
            &default_claim_checker_config(base_url, model_id),
        )?;
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
    let _ = ensure_baseline_profile_set(&dir, base_url, model_id)?;

    Ok(dir)
}

pub(crate) fn maybe_upgrade_system_prompt(profile: &mut Profile, expected_name: &str, patch: &str) -> bool {
    if profile.name != expected_name {
        return false;
    }
    if profile.system_prompt.contains(patch) {
        return false;
    }
    // Non-destructive upgrade: append a small block that corrects known failures
    // without overwriting user customizations.
    profile.system_prompt.push_str("\n\n");
    profile.system_prompt.push_str(patch);
    true
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
    if profile.system_prompt.contains(must_contain) {
        return false;
    }
    profile.system_prompt = replacement;
    true
}
