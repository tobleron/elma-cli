use crate::*;

pub(crate) fn score_calibration_report(report: &CalibrationReport) -> f64 {
    let s = &report.summary;
    (0.10 * s.speech_act.accuracy)
        + (0.10 * s.workflow.accuracy)
        + (0.05 * s.mode.accuracy)
        + (0.10 * s.route.accuracy)
        + (0.05 * s.program_parse.accuracy)
        + (0.10 * s.program_shape.accuracy)
        + (0.10 * s.program_policy.accuracy)
        + (0.07 * s.program_consistency.accuracy)
        + (0.08 * s.execution.accuracy)
        + (0.05 * s.critic.accuracy)
        + (0.08 * s.response.accuracy)
        + (0.04 * s.scope.accuracy)
        + (0.03 * s.compaction.accuracy)
        + (0.02 * s.classification.accuracy)
        + (0.02 * s.claim_check.accuracy)
        + (0.01 * s.presentation.accuracy)
}

pub(crate) fn hard_rejects_calibration_report(report: &CalibrationReport) -> bool {
    report.summary.program_parse.accuracy < 0.95 || report.summary.program_policy.accuracy < 0.95
}

pub(crate) fn score_efficiency_report(report: &EfficiencyReport) -> f64 {
    report.summary.overall_efficiency
}

pub(crate) fn prompt_patch_routing() -> &'static str {
    "ADDITIONAL EXAMPLES:\n- \"Which files in this project are safe to clean up?\" should prefer WORKFLOW, not CHAT.\n- \"Can you help me decide which files to clean up?\" should not jump directly to DECIDE without evidence.\n- Safety, cleanup, inspection, and comparison questions about the workspace usually require workflow mode."
}

pub(crate) fn prompt_patch_mode_router() -> &'static str {
    "ADDITIONAL EXAMPLES:\n- \"Which files in this project are safe to clean up?\" is usually INSPECT first, not DECIDE.\n- If the user asks to compare workspace candidates or identify safe cleanup targets, prefer INSPECT so evidence is gathered before any decision.\n- Use DECIDE only when the task is truly label-like and does not need fresh workspace evidence."
}

pub(crate) fn prompt_patch_orchestrator_cleanup() -> &'static str {
    "CLEANUP AND SAFETY RULES:\n- For cleanup, safety review, or \"what is safe to remove\" requests, default to inspect_decide_reply.\n- Gather workspace evidence first: inspect directory names, build output dirs, generated artifacts, ignore rules, and obvious system clutter.\n- Do not search repo file contents for English phrases like \"safe to delete\", \"generated\", or \"temporary\". Cleanup evidence should come from filesystem structure and known artifact types, not prose matches.\n- Distinguish safe generated artifacts, maybe-safe regenerable files, and files that should normally stay.\n- Never answer cleanup safety questions from general knowledge alone when workspace evidence is available.\n- If a shell command fails with regex, glob, quoting, or parser errors, inspect stderr and retry once with a corrected command instead of proceeding as if the evidence was valid.\n- Good cleanup evidence usually includes commands like ls, find, rg on .gitignore or config, and short targeted inspection of target, sessions, config, and repo-root clutter."
}

pub(crate) fn prompt_patch_critic_cleanup() -> &'static str {
    "CLEANUP VALIDATION:\n- If the user asked what is safe to clean up and there is no inspected workspace evidence, choose retry.\n- If a cleanup answer classifies files without evidence or after a failed shell step, choose retry.\n- If the program used DECIDE without first inspecting relevant workspace files for a cleanup task, choose retry."
}

pub(crate) fn prompt_patch_elma_grounding() -> &'static str {
    "GROUNDING RULES:\n- Base answers on the provided step results.\n- If a shell step failed or evidence is incomplete, say so plainly.\n- Do not silently replace failed evidence with generic advice unless you clearly mark it as general guidance."
}

pub(crate) fn apply_prompt_bundle(dir: &Path, bundle: &str) -> Result<()> {
    match bundle {
        "routing_bundle" => {
            let mut router = load_agent_config(&dir.join("router.toml"))?;
            let mut mode_router = load_agent_config(&dir.join("mode_router.toml"))?;
            let mut speech_act = load_agent_config(&dir.join("speech_act.toml"))?;
            let _ = maybe_upgrade_system_prompt(&mut router, "router", prompt_patch_routing());
            let _ = maybe_upgrade_system_prompt(
                &mut mode_router,
                "mode_router",
                prompt_patch_mode_router(),
            );
            let _ = maybe_upgrade_system_prompt(&mut speech_act, "speech_act", "ADDITIONAL EXAMPLES:\n- \"Can you help me decide which files to clean up?\" is usually ACTION_REQUEST because the user is asking Elma to help now.\n- \"Which files in this project are safe to clean up?\" is usually INFO_REQUEST, but it still may require workflow inspection.");
            save_agent_config(&dir.join("router.toml"), &router)?;
            save_agent_config(&dir.join("mode_router.toml"), &mode_router)?;
            save_agent_config(&dir.join("speech_act.toml"), &speech_act)?;
        }
        "workflow_bundle" => {
            let mut orch = load_agent_config(&dir.join("orchestrator.toml"))?;
            let mut critic = load_agent_config(&dir.join("critic.toml"))?;
            let _ = maybe_upgrade_system_prompt(
                &mut orch,
                "orchestrator",
                prompt_patch_orchestrator_cleanup(),
            );
            let _ =
                maybe_upgrade_system_prompt(&mut critic, "critic", prompt_patch_critic_cleanup());
            save_agent_config(&dir.join("orchestrator.toml"), &orch)?;
            save_agent_config(&dir.join("critic.toml"), &critic)?;
        }
        "response_bundle" => {
            let mut elma = load_agent_config(&dir.join("_elma.config"))?;
            let mut critic = load_agent_config(&dir.join("critic.toml"))?;
            let _ = maybe_upgrade_system_prompt(&mut elma, "_elma", prompt_patch_elma_grounding());
            let _ = maybe_upgrade_system_prompt(&mut critic, "critic", "RESPONSE VALIDATION:\n- If the final answer ignores a failed shell step, choose retry.\n- If the answer gives generic advice where evidence was expected, choose retry unless the uncertainty is explicit.");
            save_agent_config(&dir.join("_elma.config"), &elma)?;
            save_agent_config(&dir.join("critic.toml"), &critic)?;
        }
        "comprehensive_bundle" => {
            apply_prompt_bundle(dir, "routing_bundle")?;
            apply_prompt_bundle(dir, "workflow_bundle")?;
            apply_prompt_bundle(dir, "response_bundle")?;
        }
        "none" => {}
        other => anyhow::bail!("Unknown prompt bundle: {other}"),
    }
    Ok(())
}

pub(crate) fn apply_router_param_variant(dir: &Path, variant: &str) -> Result<()> {
    let settings = match variant {
        "router_strict" => (0.0, 1.0, 1u32),
        "router_soft" => (0.1, 1.0, 2u32),
        other => anyhow::bail!("Unknown router variant: {other}"),
    };
    for name in ["router.toml", "mode_router.toml", "speech_act.toml"] {
        let path = dir.join(name);
        let mut profile = load_agent_config(&path)?;
        profile.temperature = settings.0;
        profile.top_p = settings.1;
        profile.max_tokens = settings.2;
        save_agent_config(&path, &profile)?;
    }
    Ok(())
}

pub(crate) fn apply_orchestrator_param_variant(dir: &Path, variant: &str) -> Result<()> {
    let (orch_temp, orch_top_p, orch_max_tokens, planner_temp, planner_top_p, planner_tokens, verifier_temp, verifier_top_p, verifier_tokens) = match variant {
        "orch_conservative" => (0.0, 0.90, 1024, 0.0, 0.90, 1024, 0.0, 1.0, 1024),
        "orch_balanced" => (0.1, 0.95, 2048, 0.1, 0.95, 1536, 0.0, 1.0, 1024),
        "orch_creative" => (0.2, 1.0, 2048, 0.2, 0.98, 2048, 0.1, 1.0, 1024),
        other => anyhow::bail!("Unknown orchestrator variant: {other}"),
    };
    for name in [
        "orchestrator.toml",
        "workflow_planner.toml",
        "formula_selector.toml",
        "selector.toml",
    ] {
        let path = dir.join(name);
        let mut profile = load_agent_config(&path)?;
        profile.temperature = planner_temp;
        profile.top_p = planner_top_p;
        profile.max_tokens = planner_tokens;
        if name == "orchestrator.toml" {
            profile.temperature = orch_temp;
            profile.top_p = orch_top_p;
            profile.max_tokens = orch_max_tokens;
        }
        save_agent_config(&path, &profile)?;
    }
    for name in [
        "command_preflight.toml",
        "command_repair.toml",
        "task_semantics_guard.toml",
        "execution_sufficiency.toml",
        "outcome_verifier.toml",
        "critic.toml",
        "logical_reviewer.toml",
        "efficiency_reviewer.toml",
        "risk_reviewer.toml",
        "json_outputter.toml",
    ] {
        let path = dir.join(name);
        let mut profile = load_agent_config(&path)?;
        profile.temperature = verifier_temp;
        profile.top_p = verifier_top_p;
        profile.max_tokens = verifier_tokens;
        save_agent_config(&path, &profile)?;
    }
    Ok(())
}

pub(crate) fn apply_response_param_variant(dir: &Path, variant: &str) -> Result<()> {
    let (elma_temp, elma_top_p, sum_temp, plan_temp, presenter_temp, presenter_top_p, max_tokens) = match variant {
        "response_stable" => (0.3, 0.90, 0.0, 0.4, 0.1, 0.90, 2048),
        "response_balanced" => (0.5, 0.95, 0.2, 0.6, 0.2, 0.95, 4096),
        "response_creative" => (0.7, 1.0, 0.3, 0.8, 0.3, 1.0, 4096),
        other => anyhow::bail!("Unknown response variant: {other}"),
    };
    let mut elma = load_agent_config(&dir.join("_elma.config"))?;
    elma.temperature = elma_temp;
    elma.top_p = elma_top_p;
    elma.max_tokens = max_tokens;
    save_agent_config(&dir.join("_elma.config"), &elma)?;

    let mut summarizer = load_agent_config(&dir.join("summarizer.toml"))?;
    summarizer.temperature = sum_temp;
    save_agent_config(&dir.join("summarizer.toml"), &summarizer)?;

    for name in ["planner.toml", "planner_master.toml"] {
        let path = dir.join(name);
        let mut planner = load_agent_config(&path)?;
        planner.temperature = plan_temp;
        planner.top_p = 0.95;
        planner.max_tokens = max_tokens;
        save_agent_config(&path, &planner)?;
    }

    for name in ["result_presenter.toml", "formatter.toml"] {
        let path = dir.join(name);
        let mut profile = load_agent_config(&path)?;
        profile.temperature = presenter_temp;
        profile.top_p = presenter_top_p;
        profile.max_tokens = max_tokens;
        save_agent_config(&path, &profile)?;
    }

    let mut claim_checker = load_agent_config(&dir.join("claim_checker.toml"))?;
    claim_checker.temperature = 0.0;
    claim_checker.top_p = 1.0;
    claim_checker.max_tokens = 1024;
    save_agent_config(&dir.join("claim_checker.toml"), &claim_checker)?;
    Ok(())
}

pub(crate) fn conversation_excerpt(messages: &[ChatMessage], max_items: usize) -> String {
    messages
        .iter()
        .skip(1)
        .rev()
        .take(max_items)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content.replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn build_orchestrator_user_content(
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> String {
    format!(
        "User message:\n{line}\n\nSpeech-act prior:\n- chosen: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nWorkflow prior:\n- chosen: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nMode prior:\n- chosen: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nCombined route prior:\n- chosen route: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nWorkflow planner prior:\n{}\n\nComplexity prior:\n{}\n\nScope prior:\n{}\n\nFormula prior:\n{}\n\nWorkspace facts:\n{}\n\nWorkspace brief:\n{}\n\nConversation so far (most recent last):\n{}",
        route_decision.speech_act.choice,
        route_decision.speech_act.source,
        format_route_distribution(&route_decision.speech_act.distribution),
        route_decision.speech_act.margin,
        route_decision.speech_act.entropy,
        route_decision.workflow.choice,
        route_decision.workflow.source,
        format_route_distribution(&route_decision.workflow.distribution),
        route_decision.workflow.margin,
        route_decision.workflow.entropy,
        route_decision.mode.choice,
        route_decision.mode.source,
        format_route_distribution(&route_decision.mode.distribution),
        route_decision.mode.margin,
        route_decision.mode.entropy,
        route_decision.route,
        route_decision.source,
        format_route_distribution(&route_decision.distribution),
        route_decision.margin,
        route_decision.entropy,
        workflow_plan
            .map(|plan| serde_json::to_string_pretty(plan).unwrap_or_else(|_| "{}".to_string()))
            .unwrap_or_else(|| "null".to_string()),
        serde_json::to_string_pretty(complexity).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string_pretty(scope).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string_pretty(formula).unwrap_or_else(|_| "{}".to_string()),
        ws.trim(),
        ws_brief.trim(),
        conversation_excerpt(messages, 12)
    )
}

pub(crate) fn build_recovery_user_content(
    line: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    failure_reason: &str,
    current_program: Option<&Program>,
    step_results: &[StepResult],
) -> String {
    serde_json::json!({
        "failure_reason": failure_reason,
        "user_message": line,
        "speech_act_prior": {
            "choice": route_decision.speech_act.choice,
            "distribution": route_decision.speech_act.distribution.iter().map(|(label, p)| {
                serde_json::json!({"label": label, "p": p})
            }).collect::<Vec<_>>(),
        },
        "workflow_prior": {
            "choice": route_decision.workflow.choice,
            "distribution": route_decision.workflow.distribution.iter().map(|(label, p)| {
                serde_json::json!({"label": label, "p": p})
            }).collect::<Vec<_>>(),
        },
        "mode_prior": {
            "choice": route_decision.mode.choice,
            "distribution": route_decision.mode.distribution.iter().map(|(label, p)| {
                serde_json::json!({"label": label, "p": p})
            }).collect::<Vec<_>>(),
        },
        "route_prior": {
            "route": route_decision.route,
            "distribution": route_decision.distribution.iter().map(|(label, p)| {
                serde_json::json!({"label": label, "p": p})
            }).collect::<Vec<_>>(),
        },
        "workflow_planner": workflow_plan,
        "complexity": complexity,
        "scope": scope,
        "formula": formula,
        "workspace_facts": ws.trim(),
        "workspace_brief": ws_brief.trim(),
        "conversation": conversation_excerpt(messages, 12),
        "current_program_steps": current_program.map(|program| {
            program.steps.iter().map(program_step_json).collect::<Vec<_>>()
        }),
        "observed_step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
        "recovery_rules": [
            "Return the smallest valid Program JSON that can still satisfy the request.",
            "For non-CHAT routes, do not fall back to reply-only unless a concise clarifying question is the only safe next step.",
            "If the user asks to choose, rank, prioritize, or select workspace items, inspect evidence first, then decide or summarize, then reply.",
            "If the user asks to show file contents, inspect the chosen files before replying.",
            "If a select step exists, later shell steps that use the selection should normally reference it directly with a placeholder such as {{sel1|shell_words}}.",
            "Do not claim completion when the observed step results did not satisfy the request.",
            "Prefer 2-4 steps unless more are clearly necessary."
        ]
    })
    .to_string()
}
