//! @efficiency-role: orchestrator
//!
//! App Bootstrap - Profile Loading and Synchronization
//!
//! Loading order:
//! 1. Model-specific config (e.g., config/llama_3.2_3b_instruct_q6_k_l.gguf/angel_helper.toml)
//! 2. Global defaults (config/defaults/angel_helper.toml)
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
    let default_path = defaults_dir.join(path.file_name().unwrap());
    
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
        planner_master_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("planner_master.toml"))?,
        planner_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("planner.toml"))?,
        decider_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("decider.toml"))?,
        selector_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("selector.toml"))?,
        summarizer_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("summarizer.toml"))?,
        formatter_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("formatter.toml"))?,
        json_outputter_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("json_outputter.toml"))?,
        final_answer_extractor_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("final_answer_extractor.toml"),
        )?,
        complexity_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("complexity_assessor.toml"))?,
        formula_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("formula_selector.toml"))?,
        workflow_planner_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("workflow_planner.toml"))?,
        evidence_mode_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("evidence_mode.toml"))?,
        command_repair_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("command_repair.toml"))?,
        task_semantics_guard_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("task_semantics_guard.toml"),
        )?,
        execution_sufficiency_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("execution_sufficiency.toml"),
        )?,
        outcome_verifier_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("outcome_verifier.toml"))?,
        memory_gate_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("memory_gate.toml"))?,
        command_preflight_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("command_preflight.toml"))?,
        scope_builder_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("scope_builder.toml"))?,
        evidence_compactor_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("evidence_compactor.toml"))?,
        artifact_classifier_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("artifact_classifier.toml"))?,
        result_presenter_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("result_presenter.toml"))?,
        claim_checker_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("claim_checker.toml"))?,
        orchestrator_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("orchestrator.toml"))?,
        critic_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("critic.toml"))?,
        logical_reviewer_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("logical_reviewer.toml"))?,
        efficiency_reviewer_cfg: load_agent_config_with_fallback(
            &model_cfg_dir.join("efficiency_reviewer.toml"),
        )?,
        risk_reviewer_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("risk_reviewer.toml"))?,
        refinement_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("refinement.toml"))?,
        reflection_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("reflection.toml"))?,
        meta_review_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("meta_review.toml"))?,
        router_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("router.toml"))?,
        mode_router_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("mode_router.toml"))?,
        speech_act_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("speech_act.toml"))?,
        intent_helper_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("intent_helper.toml"))?,
        angel_helper_cfg: load_agent_config_with_fallback(&model_cfg_dir.join("angel_helper.toml"))?,
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
    let json_outputter_cfg_path = model_cfg_dir.join("json_outputter.toml");
    profiles.json_outputter_cfg.base_url = base_url.to_string();
    profiles.json_outputter_cfg.model = model_id.to_string();
    save_agent_config(&json_outputter_cfg_path, &profiles.json_outputter_cfg)?;
    let final_answer_extractor_cfg_path = model_cfg_dir.join("final_answer_extractor.toml");
    profiles.final_answer_extractor_cfg.base_url = base_url.to_string();
    profiles.final_answer_extractor_cfg.model = model_id.to_string();
    save_agent_config(
        &final_answer_extractor_cfg_path,
        &profiles.final_answer_extractor_cfg,
    )?;

    let router_cfg_path = model_cfg_dir.join("router.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.router_cfg,
        "router",
        "2 = WORKFLOW",
        default_router_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=router.system_prompt");
        save_agent_config(&router_cfg_path, &profiles.router_cfg)?;
    }

    let mode_router_cfg_path = model_cfg_dir.join("mode_router.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.mode_router_cfg,
        "mode_router",
        "1 = INSPECT",
        default_mode_router_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=mode_router.system_prompt");
        save_agent_config(&mode_router_cfg_path, &profiles.mode_router_cfg)?;
    }

    let speech_act_cfg_path = model_cfg_dir.join("speech_act.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.speech_act_cfg,
        "speech_act",
        "1 = CAPABILITY_CHECK",
        default_speech_act_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=speech_act.system_prompt");
        save_agent_config(&speech_act_cfg_path, &profiles.speech_act_cfg)?;
    }

    let orchestrator_cfg_path = model_cfg_dir.join("orchestrator.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "EVIDENCE-FIRST RULES",
        default_orchestrator_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=orchestrator.system_prompt");
        save_agent_config(&orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }

    let critic_cfg_path = model_cfg_dir.join("critic.toml");
    if replace_system_prompt_if_missing(
        &mut profiles.critic_cfg,
        "critic",
        "there is no workspace evidence in the step results",
        default_critic_config(base_url, model_id).system_prompt,
    ) {
        trace(args, "upgraded=critic.system_prompt");
        save_agent_config(&critic_cfg_path, &profiles.critic_cfg)?;
    }

    apply_prompt_upgrades(
        args,
        &elma_cfg_path,
        &router_cfg_path,
        &mode_router_cfg_path,
        &speech_act_cfg_path,
        &orchestrator_cfg_path,
        &critic_cfg_path,
        profiles,
    )
}

fn apply_prompt_upgrades(
    args: &Args,
    elma_cfg_path: &PathBuf,
    router_cfg_path: &PathBuf,
    mode_router_cfg_path: &PathBuf,
    speech_act_cfg_path: &PathBuf,
    orchestrator_cfg_path: &PathBuf,
    critic_cfg_path: &PathBuf,
    profiles: &mut LoadedProfiles,
) -> Result<()> {
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "ROUTER PRIOR RULES:\n- You will receive a probabilistic route prior over CHAT, SHELL, PLAN, MASTERPLAN, and DECIDE.\n- Treat the route prior as evidence, not a hard rule.\n- If the route prior is uncertain or the user request is genuinely ambiguous, you may output a Program with a single reply step that asks one concise clarifying question.",
    ) {
        trace(args, "upgraded=orchestrator.router_prior");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "- A shell step is for real workspace inspection or execution only. Never use shell steps to print prose, plan lines, or explanations.\n- If the user asks for a plan, prefer a plan or masterplan step plus an optional reply step. Do not emit plan text through shell commands.",
    ) {
        trace(args, "upgraded=orchestrator.shell_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "- If the user asks for one concrete step-by-step plan, use a plan step.\n- If the user asks for a higher-level overall plan across phases, use a masterplan step.",
    ) {
        trace(args, "upgraded=orchestrator.plan_distinction");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "STRUCTURE RULES:\n- Every step must include purpose and success_condition.\n- Use depends_on to reference earlier step ids when a later step consumes prior results.\n- For summarize steps that summarize earlier outputs, leave text empty and set depends_on.\n- Keep programs minimal. Remove any step that does not directly advance the objective.",
    ) {
        trace(args, "upgraded=orchestrator.structure_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "PLAN EXAMPLE:\nUser: Create a step-by-step plan to add a new config file to this Rust project.\nOutput:\n{\"objective\":\"create a concrete plan for adding a config file\",\"steps\":[{\"id\":\"p1\",\"type\":\"plan\",\"goal\":\"Add a new config file to this Rust project.\",\"purpose\":\"plan\",\"depends_on\":[],\"success_condition\":\"a concrete step-by-step plan is saved\"},{\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"Tell the user a step-by-step plan was created and summarize it briefly in plain text.\",\"purpose\":\"answer\",\"depends_on\":[\"p1\"],\"success_condition\":\"the user receives a concise plain-text summary of the saved plan\"}]}",
    ) {
        trace(args, "upgraded=orchestrator.plan_example");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "MINIMALITY RULES:\n- For a step-by-step plan request, default to one plan step plus an optional reply step.\n- Do not inspect src/main.rs, config files, or prompt files just because examples mention them.\n- Only add shell inspection to a plan request when the plan truly depends on current workspace evidence.",
    ) {
        trace(args, "upgraded=orchestrator.minimality_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.router_cfg,
        "router",
        "Important distinctions:\n- Greetings or general knowledge questions are usually 1.\n- Questions about the current project, files, code, or tasks that need planning or decisions are usually 2.",
    ) {
        trace(args, "upgraded=router.examples");
        save_agent_config(router_cfg_path, &profiles.router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.router_cfg,
        "router",
        "- Output must be exactly one digit from 1 to 2.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents whether Elma should enter workflow mode.",
    ) {
        trace(args, "upgraded=router.workflow_rules");
        save_agent_config(router_cfg_path, &profiles.router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.mode_router_cfg,
        "mode_router",
        "Important distinctions:\n- \"What is my current project about?\", \"read Cargo.toml and summarize it\", and \"find where fetch_ctx_max is defined\" are usually 1.\n- \"list files\", \"run tests\", and \"build the project\" are usually 2.\n- \"Create a step-by-step plan\" is 3, not 4.\n- Only choose 4 when the user truly wants an overall master plan.",
    ) {
        trace(args, "upgraded=mode_router.examples");
        save_agent_config(mode_router_cfg_path, &profiles.mode_router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.speech_act_cfg,
        "speech_act",
        "Important distinctions:\n- \"Are you able to list files here?\" is usually 1.\n- \"What is my current project about?\" is usually 2.\n- \"Can you list files?\" and \"Could you run the tests?\" are usually 3 in normal English, because they are indirect requests.",
    ) {
        trace(args, "upgraded=speech_act.examples");
        save_agent_config(speech_act_cfg_path, &profiles.speech_act_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "- If a shell step only prints prose or plan text instead of inspecting or executing something real in the workspace, choose retry.",
    ) {
        trace(args, "upgraded=critic.shell_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "- If the user asked for a step-by-step plan and there is no plan step result, choose retry and provide a corrected Program that uses type \"plan\".\n- If the user asked for an overall or master plan and there is no masterplan step result, choose retry and provide a corrected Program that uses type \"masterplan\".",
    ) {
        trace(args, "upgraded=critic.plan_distinction");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "EVALUATION RULES:\n- Judge whether each step's purpose and success_condition actually advanced the objective.\n- If a step has depends_on, verify the dependent outputs were meaningfully used.\n- For planning requests, reject shell steps unless they gather clearly necessary workspace evidence.\n- Prefer the simplest valid program that can satisfy the request.",
    ) {
        trace(args, "upgraded=critic.evaluation_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "PLAN VALIDATION HINTS:\n- If any successful step_result has type \"plan\", the step-by-step plan requirement is satisfied.\n- If any successful step_result has type \"masterplan\", the master plan requirement is satisfied.\n- For a step-by-step plan request, reject unnecessary shell inspection and prefer a corrected program with only a plan step and an optional reply step.",
    ) {
        trace(args, "upgraded=critic.plan_validation_hints");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "SPEECH-ACT RULES:\n- You will receive a probabilistic speech-act prior over CAPABILITY_CHECK, INFO_REQUEST, and ACTION_REQUEST.\n- If CAPABILITY_CHECK dominates, prefer a reply step that answers whether Elma can do it. Do not execute commands unless the user also asked for action now.\n- INFO_REQUEST may still require workspace inspection before answering.\n- ACTION_REQUEST may use shell, plan, masterplan, or decide steps as needed.",
    ) {
        trace(args, "upgraded=orchestrator.speech_act_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.orchestrator_cfg,
        "orchestrator",
        "COMPLEXITY AND FORMULA PRIORS:\n- You may receive a complexity prior and a formula prior.\n- Treat them as guidance, not hard rules.\n- For cleanup, safety review, or comparison requests about the workspace, prefer inspect_decide_reply.\n- If a shell command fails because of regex, glob, quoting, or parser issues, repair it once and continue if safe.",
    ) {
        trace(args, "upgraded=orchestrator.complexity_formula_rules");
        save_agent_config(orchestrator_cfg_path, &profiles.orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "CLEANUP VALIDATION:\n- If the user asked what is safe to clean up and there is no inspected workspace evidence, choose retry.\n- If a cleanup answer classifies files after a failed shell step, choose retry.\n- If a cleanup task used DECIDE without prior inspection, choose retry.",
    ) {
        trace(args, "upgraded=critic.cleanup_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.elma_cfg,
        "_elma",
        prompt_patch_elma_grounding(),
    ) {
        trace(args, "upgraded=elma.grounding_rules");
        save_agent_config(elma_cfg_path, &profiles.elma_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut profiles.critic_cfg,
        "critic",
        "SPEECH-ACT VALIDATION:\n- If speech_act is CAPABILITY_CHECK and the program executed shell or planning actions without explicit user intent to do so now, choose retry and replace it with a reply-only program.\n- If speech_act is ACTION_REQUEST, reject answers that only talk about capability without attempting the task when it is allowed.",
    ) {
        trace(args, "upgraded=critic.speech_act_rules");
        save_agent_config(critic_cfg_path, &profiles.critic_cfg)?;
    }
    Ok(())
}
