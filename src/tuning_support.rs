use crate::*;
use crate::formulas::FormulaPattern;

/// Calculate variance from a slice of scores
pub(crate) fn calculate_variance(scores: &[f64]) -> f64 {
    if scores.is_empty() {
        return 0.0;
    }
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let sum_sq_diff = scores.iter().map(|s| (s - mean).powi(2)).sum::<f64>();
    sum_sq_diff / scores.len() as f64
}

/// Calculate standard deviation from variance
pub(crate) fn calculate_std_dev(variance: f64) -> f64 {
    variance.sqrt()
}

/// Calculate adjusted score with variance penalty
pub(crate) fn calculate_adjusted_score(mean_score: f64, std_dev: f64, penalty_multiplier: f64) -> f64 {
    // Penalize high-variance candidates
    (mean_score - (std_dev * penalty_multiplier)).max(0.0)
}

/// Default variance penalty multiplier (can be tuned)
pub(crate) const VARIANCE_PENALTY_MULTIPLIER: f64 = 0.5;

/// Explicit allowlist of profile fields that tuning variants may change.
/// Any mutation outside this list is a tuning boundary violation.
#[allow(dead_code)]
const TUNABLE_FIELDS: &[&str] = &[
    "temperature",
    "top_p",
    "repeat_penalty",
    "max_tokens",
];

/// Fields that tuning must NEVER change.
#[allow(dead_code)]
const IMMUTABLE_FIELDS: &[&str] = &[
    "system_prompt",
    "reasoning_format",
    "name",
    "version",
];

/// Validate that a tuning variant only changed allowed fields between
/// an original profile and a mutated profile. Returns an error with
/// the field name if an immutable field was mutated.
pub(crate) fn validate_tuning_mutation(original: &Profile, mutated: &Profile) -> Result<()> {
    if original.system_prompt != mutated.system_prompt {
        anyhow::bail!("tuning boundary violation: system_prompt was mutated by a tuning variant");
    }
    if original.reasoning_format != mutated.reasoning_format {
        anyhow::bail!("tuning boundary violation: reasoning_format was mutated by a tuning variant");
    }
    if original.name != mutated.name {
        anyhow::bail!("tuning boundary violation: name was mutated by a tuning variant");
    }
    Ok(())
}

/// Validate an entire profile directory after a tuning variant was applied.
/// Compares each managed profile file against the originals from src_dir.
pub(crate) fn validate_tuning_mutations(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    for filename in managed_profile_file_names() {
        let src_path = src_dir.join(&filename);
        let dst_path = dst_dir.join(&filename);
        if !src_path.exists() || !dst_path.exists() {
            continue;
        }
        let original = load_agent_config(&src_path)?;
        let mutated = load_agent_config(&dst_path)?;
        validate_tuning_mutation(&original, &mutated)
            .with_context(|| format!("in profile file {filename}"))?;
    }
    Ok(())
}

/// The minimum improvement margin required over the runtime-default baseline
/// for a tuned candidate to be activated. Below this threshold, the baseline
/// is preferred for stability.
pub(crate) const ACTIVATION_MARGIN: f64 = 0.02;

/// Determine the activation reason when comparing a candidate against a baseline.
pub(crate) fn activation_reason(
    candidate_score: f64,
    baseline_score: f64,
    candidate_certified: bool,
) -> (bool, String) {
    let margin = candidate_score - baseline_score;
    if margin < ACTIVATION_MARGIN {
        (
            false,
            format!(
                "baseline_preferred: candidate {:.4} vs baseline {:.4} (margin {:.4} < threshold {:.4})",
                candidate_score, baseline_score, margin, ACTIVATION_MARGIN
            ),
        )
    } else if candidate_certified {
        (
            true,
            format!(
                "higher_score_and_certified: candidate {:.4} vs baseline {:.4} (margin {:.4})",
                candidate_score, baseline_score, margin
            ),
        )
    } else {
        (
            true,
            format!(
                "higher_score: candidate {:.4} vs baseline {:.4} (margin {:.4})",
                candidate_score, baseline_score, margin
            ),
        )
    }
}

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
        "none" => {}
        other => anyhow::bail!(
            "Prompt-bundle tuning is disabled by reliability policy; unsupported bundle: {other}"
        ),
    }
    Ok(())
}

pub(crate) fn apply_runtime_generation_defaults(
    dir: &Path,
    defaults: &RuntimeGenerationDefaults,
) -> Result<()> {
    for filename in managed_profile_file_names() {
        let path = dir.join(&filename);
        if !path.exists() {
            continue;
        }
        let original = load_agent_config(&path)?;
        let mut profile = original.clone();
        if let Some(v) = defaults.temperature {
            profile.temperature = v;
        }
        if let Some(v) = defaults.top_p {
            profile.top_p = v;
        }
        if let Some(v) = defaults.repeat_penalty {
            profile.repeat_penalty = v;
        }
        if let Some(v) = defaults.max_tokens {
            profile.max_tokens = profile.max_tokens.min(v);
        }
        validate_tuning_mutation(&original, &profile)
            .with_context(|| format!("runtime-default mapping in {filename}"))?;
        save_agent_config(&path, &profile)?;
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
        let original = load_agent_config(&path)?;
        let mut profile = original.clone();
        profile.temperature = settings.0;
        profile.top_p = settings.1;
        profile.max_tokens = settings.2;
        validate_tuning_mutation(&original, &profile)
            .with_context(|| format!("router variant '{variant}' in {name}"))?;
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
        let original = load_agent_config(&path)?;
        let mut profile = original.clone();
        profile.temperature = planner_temp;
        profile.top_p = planner_top_p;
        profile.max_tokens = planner_tokens;
        if name == "orchestrator.toml" {
            profile.temperature = orch_temp;
            profile.top_p = orch_top_p;
            profile.max_tokens = orch_max_tokens;
        }
        validate_tuning_mutation(&original, &profile)
            .with_context(|| format!("orchestrator variant '{variant}' in {name}"))?;
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
        let original = load_agent_config(&path)?;
        let mut profile = original.clone();
        profile.temperature = verifier_temp;
        profile.top_p = verifier_top_p;
        profile.max_tokens = verifier_tokens;
        validate_tuning_mutation(&original, &profile)
            .with_context(|| format!("orchestrator variant '{variant}' in {name}"))?;
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
    let elma_original = load_agent_config(&dir.join("_elma.config"))?;
    let mut elma = elma_original.clone();
    elma.temperature = elma_temp;
    elma.top_p = elma_top_p;
    elma.max_tokens = max_tokens;
    validate_tuning_mutation(&elma_original, &elma)
        .with_context(|| format!("response variant '{variant}' in _elma.config"))?;
    save_agent_config(&dir.join("_elma.config"), &elma)?;

    let sum_original = load_agent_config(&dir.join("summarizer.toml"))?;
    let mut summarizer = sum_original.clone();
    summarizer.temperature = sum_temp;
    validate_tuning_mutation(&sum_original, &summarizer)
        .with_context(|| format!("response variant '{variant}' in summarizer.toml"))?;
    save_agent_config(&dir.join("summarizer.toml"), &summarizer)?;

    for name in ["planner.toml", "planner_master.toml"] {
        let path = dir.join(name);
        let original = load_agent_config(&path)?;
        let mut planner = original.clone();
        planner.temperature = plan_temp;
        planner.top_p = 0.95;
        planner.max_tokens = max_tokens;
        validate_tuning_mutation(&original, &planner)
            .with_context(|| format!("response variant '{variant}' in {name}"))?;
        save_agent_config(&path, &planner)?;
    }

    for name in ["result_presenter.toml", "formatter.toml"] {
        let path = dir.join(name);
        let original = load_agent_config(&path)?;
        let mut profile = original.clone();
        profile.temperature = presenter_temp;
        profile.top_p = presenter_top_p;
        profile.max_tokens = max_tokens;
        validate_tuning_mutation(&original, &profile)
            .with_context(|| format!("response variant '{variant}' in {name}"))?;
        save_agent_config(&path, &profile)?;
    }

    let cc_original = load_agent_config(&dir.join("claim_checker.toml"))?;
    let mut claim_checker = cc_original.clone();
    claim_checker.temperature = 0.0;
    claim_checker.top_p = 1.0;
    claim_checker.max_tokens = 1024;
    validate_tuning_mutation(&cc_original, &claim_checker)
        .with_context(|| format!("response variant '{variant}' in claim_checker.toml"))?;
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
    tool_registry: &crate::tools::ToolRegistry,
    formula_selection: &crate::formulas::FormulaSelectionResult,
) -> String {
    let features = ClassificationFeatures::from(route_decision);

    // Determine entropy warning level
    let entropy_warning = if features.entropy < 0.05 {
        "⚠️ EXTREMELY LOW - Classifier is over-confident. Probabilities have been adjusted to encourage alternative reasoning."
    } else if features.entropy < 0.1 {
        "⚠️ LOW - Classifier may be over-confident. Consider alternative interpretations."
    } else if features.entropy < 0.5 {
        "📊 MODERATE - Some uncertainty. Use your judgment."
    } else {
        "🔍 HIGH - Classifier is uncertain. Rely on your reasoning."
    };

    // Build classification features section with autonomy guidance
    let classification_section = format!(
        "## Classification Features (SOFT EVIDENCE - Not Hard Rules)\n\n\
         These probabilities are signals from a classifier, NOT deterministic rules.\n\
         You should reason about the actual user request and override these priors when appropriate.\n\n\
         **Speech Act Probabilities:** {}\n\
         **Workflow Probabilities:** {}\n\
         **Mode Probabilities:** {}\n\
         **Route Probabilities:** {}\n\
         **Classification Entropy:** {:.2} - {}\n\
         **Suggested Route:** {} (treat as a suggestion, not a command)\n\n\
         **AUTONOMY RULE:** If the user's actual request clearly requires a different approach \
         than what the priors suggest, follow the user's request. These priors are here to help, \
         not to constrain your reasoning.\n\n",
        format_route_distribution(&features.speech_act_probs),
        format_route_distribution(&features.workflow_probs),
        format_route_distribution(&features.mode_probs),
        format_route_distribution(&features.route_probs),
        features.entropy,
        entropy_warning,
        features.suggested_route
    );

    // Build available tools section
    let tools_section = tool_registry.format_tools_for_prompt();

    // Build formula section with scores
    let formula_section = format!(
        "## Formula Pattern\n\n\
         **Selected Formula:** {}\n\
         **Intent:** {}\n\
         **Expected Steps:** {:?}\n\
         **Cost Score:** {} (1-10, lower = cheaper)\n\
         **Value Score:** {} (1-10, higher = more thorough)\n\
         **Efficiency Ratio:** {:.2} (value / cost)\n\
         **Selection Reason:** {}\n\n\
         **INSTRUCTION:** Generate program steps that match this formula pattern.\n\
         Use available tools from the tool registry to implement each step.\n\
         For example, if formula is 'inspect_reply', use read/search/workspace_tree tools for inspection,\n\
         then use reply tool to present findings.\n\n",
        formula_selection.formula,
        FormulaPattern::by_name(&formula_selection.formula)
            .map(|f| f.intent)
            .unwrap_or("Execute task"),
        formula_selection.scores.expected_steps,
        formula_selection.scores.cost_score,
        formula_selection.scores.value_score,
        formula_selection.scores.efficiency_ratio,
        formula_selection.reason
    );

    format!(
        "User message:\n{line}\n\n{}\
         {}\n\n\
         {}\n\n\
         Workflow planner prior:\n{}\n\n\
         Complexity prior:\n{}\n\n\
         Scope prior:\n{}\n\n\
         Formula prior:\n{}\n\n\
         Workspace facts:\n{}\n\n\
         Workspace brief:\n{}\n\n\
         Conversation so far (most recent last):\n{}",
        classification_section,
        tools_section,
        formula_section,
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
