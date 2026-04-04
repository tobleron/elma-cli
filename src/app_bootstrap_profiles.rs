//! @efficiency-role: service-orchestrator
//!
//! App Bootstrap - Profile Loading and Synchronization
//!
//! Loading order:
//! 1. Model-specific config (e.g., config/llama_3.2_3b_instruct_q6_k_l.gguf/intent_helper.toml)
//! 2. Global defaults (config/defaults/intent_helper.toml)
//! 3. Built-in fallback (minimal, should never be needed)

use crate::app::LoadedProfiles;
use crate::*;

/// Load agent config with fallback to global defaults
fn load_agent_config_with_fallback(path: &PathBuf) -> Result<Profile> {
    // Try model-specific config first
    if path.exists() {
        return load_agent_config(path);
    }

    // Fall back to global defaults
    let defaults_dir = std::path::PathBuf::from("config/defaults");
    let Some(file_name) = path.file_name() else {
        return Err(anyhow::anyhow!(
            "Config path has no file name: {}",
            path.display()
        ));
    };
    let default_path = defaults_dir.join(file_name);

    if default_path.exists() {
        return load_agent_config(&default_path);
    }

    // Final fallback: return error (should never happen if defaults are complete)
    Err(anyhow::anyhow!(
        "Config not found: {} (and no default available)",
        path.display()
    ))
}

pub(crate) fn load_profiles(model_cfg_dir: &PathBuf) -> Result<LoadedProfiles> {
    Ok(LoadedProfiles {
        elma_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("_elma.config"))?,
        expert_responder_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("expert_responder.toml"),
        )?,
        status_message_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("status_message_generator.toml"),
        )?,
        planner_master_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("planner_master.toml"),
        )?,
        planner_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("planner.toml"))?,
        decider_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("decider.toml"))?,
        selector_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("selector.toml"))?,
        summarizer_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("summarizer.toml"))?,
        formatter_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("formatter.toml"))?,
        json_outputter_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("json_outputter.toml"),
        )?,
        final_answer_extractor_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("final_answer_extractor.toml"),
        )?,
        complexity_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("complexity_assessor.toml"),
        )?,
        evidence_need_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("evidence_need_assessor.toml"),
        )?,
        action_need_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("action_need_assessor.toml"),
        )?,
        formula_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("formula_selector.toml"))?,
        workflow_planner_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("workflow_planner.toml"),
        )?,
        evidence_mode_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("evidence_mode.toml"),
        )?,
        command_repair_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("command_repair.toml"),
        )?,
        task_semantics_guard_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("task_semantics_guard.toml"),
        )?,
        execution_sufficiency_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("execution_sufficiency.toml"),
        )?,
        outcome_verifier_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("outcome_verifier.toml"),
        )?,
        memory_gate_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("memory_gate.toml"))?,
        command_preflight_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("command_preflight.toml"),
        )?,
        scope_builder_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("scope_builder.toml"),
        )?,
        evidence_compactor_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("evidence_compactor.toml"),
        )?,
        artifact_classifier_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("artifact_classifier.toml"),
        )?,
        result_presenter_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("result_presenter.toml"),
        )?,
        claim_checker_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("claim_checker.toml"),
        )?,
        orchestrator_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("orchestrator.toml"),
        )?,
        critic_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("critic.toml"))?,
        logical_reviewer_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("logical_reviewer.toml"),
        )?,
        efficiency_reviewer_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("efficiency_reviewer.toml"),
        )?,
        risk_reviewer_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("risk_reviewer.toml"),
        )?,
        refinement_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("refinement.toml"))?,
        reflection_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("reflection.toml"))?,
        meta_review_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("meta_review.toml"))?,
        router_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("router.toml"))?,
        mode_router_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("mode_router.toml"))?,
        speech_act_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("speech_act.toml"))?,
        intent_helper_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("intent_helper.toml"),
        )?,
        router_cal: load_router_calibration(&model_cfg_dir.join("router_calibration.toml"))?,
    })
}

pub(crate) fn sync_and_upgrade_profiles(
    args: &Args,
    model_cfg_dir: &PathBuf,
    base_url: &str,
    model_id: &str,
    profiles: &mut LoadedProfiles,
) -> Result<()> {
    let elma_cfg_path = model_cfg_dir.join("_elma.config");
    profiles.elma_cfg.base_url = base_url.to_string();
    profiles.elma_cfg.model = model_id.to_string();
    save_agent_config(&elma_cfg_path, &profiles.elma_cfg)?;

    sync_managed_profile(
        args,
        &model_cfg_dir.join("json_outputter.toml"),
        &mut profiles.json_outputter_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("final_answer_extractor.toml"),
        &mut profiles.final_answer_extractor_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("router.toml"),
        &mut profiles.router_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("mode_router.toml"),
        &mut profiles.mode_router_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("speech_act.toml"),
        &mut profiles.speech_act_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("expert_responder.toml"),
        &mut profiles.expert_responder_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("result_presenter.toml"),
        &mut profiles.result_presenter_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("claim_checker.toml"),
        &mut profiles.claim_checker_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("formatter.toml"),
        &mut profiles.formatter_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("status_message_generator.toml"),
        &mut profiles.status_message_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("intent_helper.toml"),
        &mut profiles.intent_helper_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("orchestrator.toml"),
        &mut profiles.orchestrator_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("critic.toml"),
        &mut profiles.critic_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("complexity_assessor.toml"),
        &mut profiles.complexity_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("evidence_need_assessor.toml"),
        &mut profiles.evidence_need_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("action_need_assessor.toml"),
        &mut profiles.action_need_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("selector.toml"),
        &mut profiles.selector_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("formula_selector.toml"),
        &mut profiles.formula_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("workflow_planner.toml"),
        &mut profiles.workflow_planner_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("evidence_mode.toml"),
        &mut profiles.evidence_mode_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("command_repair.toml"),
        &mut profiles.command_repair_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("task_semantics_guard.toml"),
        &mut profiles.task_semantics_guard_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("execution_sufficiency.toml"),
        &mut profiles.execution_sufficiency_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("outcome_verifier.toml"),
        &mut profiles.outcome_verifier_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("memory_gate.toml"),
        &mut profiles.memory_gate_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("command_preflight.toml"),
        &mut profiles.command_preflight_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("scope_builder.toml"),
        &mut profiles.scope_builder_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("logical_reviewer.toml"),
        &mut profiles.logical_reviewer_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("efficiency_reviewer.toml"),
        &mut profiles.efficiency_reviewer_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("risk_reviewer.toml"),
        &mut profiles.risk_reviewer_cfg,
        base_url,
        model_id,
    )?;
    sync_managed_profile(
        args,
        &model_cfg_dir.join("reflection.toml"),
        &mut profiles.reflection_cfg,
        base_url,
        model_id,
    )?;

    Ok(())
}

fn sync_managed_profile(
    args: &Args,
    path: &PathBuf,
    profile: &mut Profile,
    base_url: &str,
    model_id: &str,
) -> Result<()> {
    let original_base = profile.base_url.clone();
    let original_model = profile.model.clone();
    let original_prompt = profile.system_prompt.clone();

    profile.base_url = base_url.to_string();
    profile.model = model_id.to_string();
    apply_canonical_system_prompt(profile);

    if profile.base_url != original_base
        || profile.model != original_model
        || profile.system_prompt != original_prompt
    {
        trace(args, &format!("synced_profile={}", profile.name));
        save_agent_config(path, profile)?;
    }
    Ok(())
}
