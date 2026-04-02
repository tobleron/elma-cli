use crate::*;

fn neutral_route_decision() -> RouteDecision {
    let base = ProbabilityDecision {
        choice: String::new(),
        source: "compat_wrapper".to_string(),
        distribution: Vec::new(),
        margin: 0.0,
        entropy: 1.0,
    };
    RouteDecision {
        route: String::new(),
        source: "compat_wrapper".to_string(),
        distribution: Vec::new(),
        margin: 0.0,
        entropy: 1.0,
        speech_act: base.clone(),
        workflow: base.clone(),
        mode: base,
    }
}

fn base_intel_context(
    client: &reqwest::Client,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> IntelContext {
    IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        workspace_facts.to_string(),
        workspace_brief.to_string(),
        messages.to_vec(),
        client.clone(),
    )
}

pub(crate) async fn generate_status_message_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    current_action: &str,
    step_type: &str,
    step_purpose: &str,
) -> Result<String> {
    let unit = StatusMessageUnit::new(cfg.clone());
    let context = IntelContext::new(
        current_action.to_string(),
        neutral_route_decision(),
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("current_action", current_action)?
    .with_extra("step_type", step_type)?
    .with_extra("step_purpose", step_purpose)?;
    let output = unit.execute_with_fallback(&context).await?;
    Ok(output
        .get_str("status")
        .unwrap_or(current_action)
        .to_string())
}

pub(crate) async fn assess_complexity_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ComplexityAssessment> {
    let unit = ComplexityAssessmentUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        workspace_facts,
        workspace_brief,
        messages,
    );
    let output = unit.execute_with_fallback(&context).await?;
    Ok(ComplexityOutput::from_intel_output(&output)?.assessment)
}

pub(crate) async fn assess_evidence_needs_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<(bool, bool)> {
    let unit = EvidenceNeedsUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        workspace_facts,
        workspace_brief,
        messages,
    );
    let output = unit.execute_with_fallback(&context).await?;
    let parsed = EvidenceNeedsOutput::from_intel_output(&output)?;
    Ok((parsed.needs_evidence, parsed.needs_tools))
}

pub(crate) async fn assess_action_needs_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<(bool, bool)> {
    let unit = ActionNeedsUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        workspace_facts,
        workspace_brief,
        messages,
    );
    let output = unit.execute_with_fallback(&context).await?;
    let parsed = ActionNeedsOutput::from_intel_output(&output)?;
    Ok((parsed.needs_decision, parsed.needs_plan))
}

pub(crate) async fn suggest_pattern_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    messages: &[ChatMessage],
) -> Result<String> {
    let unit = PatternSuggestionUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        "",
        "",
        messages,
    );
    let output = unit.execute_with_fallback(&context).await?;
    Ok(PatternSuggestionOutput::from_intel_output(&output)?.suggested_pattern)
}

pub(crate) async fn build_scope_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ScopePlan> {
    let unit = ScopeBuilderUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        workspace_facts,
        workspace_brief,
        messages,
    )
    .with_complexity(complexity.clone());
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data).map_err(|e| anyhow::anyhow!("Failed to parse scope plan: {}", e))
}

pub(crate) async fn select_formula_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> Result<FormulaSelection> {
    let memory_candidates = memories
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id, "title": m.title, "route": m.route, "complexity": m.complexity,
                "formula": m.formula, "objective": m.objective, "example_user_message": m.user_message,
                "program_signature": m.program_signature, "success_count": m.success_count,
                "failure_count": m.failure_count, "last_success_unix_s": m.last_success_unix_s,
                "artifact_mode_capable": m.artifact_mode_capable, "active_run_id": m.active_run_id,
            })
        })
        .collect::<Vec<_>>();
    let unit = FormulaSelectorUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        "",
        "",
        messages,
    )
    .with_complexity(complexity.clone())
    .with_extra("scope", scope)?
    .with_extra("memory_candidates", memory_candidates)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse formula selection: {}", e))
}

pub(crate) async fn plan_workflow_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    _memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> Result<WorkflowPlannerOutput> {
    let unit = WorkflowPlannerUnit::new(cfg.clone());
    let context = base_intel_context(
        client,
        user_message,
        route_decision,
        workspace_facts,
        workspace_brief,
        messages,
    );
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse workflow planner output: {}", e))
}

pub(crate) async fn select_items_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    instructions: &str,
    evidence: &str,
) -> Result<SelectionOutput> {
    let unit = SelectorUnit::new(cfg.clone());
    let context = IntelContext::new(
        objective.to_string(),
        neutral_route_decision(),
        evidence.to_string(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("purpose", purpose)?
    .with_extra("instructions", instructions)?
    .with_extra("evidence", evidence)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse selector output: {}", e))
}

pub(crate) async fn decide_evidence_mode_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
) -> Result<EvidenceModeDecision> {
    // Detect if user explicitly asked for command execution
    let has_command_request = user_message
        .to_lowercase()
        .split_whitespace()
        .any(|w| ["run", "execute", "show", "display", "print"].contains(&w));

    // Check if any step actually executed a command
    let has_command_execution = step_results
        .iter()
        .any(|s| s.command.as_ref().is_some_and(|c| !c.is_empty()));

    // Check if step results have artifact_path (indicates output was captured to file)
    let has_artifact = step_results
        .iter()
        .any(|s| s.artifact_path.as_ref().is_some_and(|p| !p.is_empty()));

    // Deterministic override for command execution requests
    // This ensures RAW output is shown when user explicitly asks to run/see commands
    if has_command_request || has_command_execution {
        // Estimate output size from step results
        let output_is_short = step_results
            .iter()
            .filter_map(|s| s.raw_output.as_ref())
            .all(|out| out.lines().count() < 100);

        // Force RAW or RAW_PLUS_COMPACT for command execution
        let mode = if has_artifact {
            "RAW_PLUS_COMPACT".to_string() // Has file artifact, show both
        } else if output_is_short {
            "RAW".to_string() // Short output, show raw
        } else {
            "RAW_PLUS_COMPACT".to_string() // Long output, show raw + compact summary
        };

        return Ok(EvidenceModeDecision {
            mode,
            reason: "Command execution detected - showing raw output".to_string(),
        });
    }

    let narrative = crate::intel_narrative::build_evidence_mode_narrative(
        user_message,
        route_decision,
        reply_instructions,
        step_results,
        has_command_request,
        has_command_execution,
        has_artifact,
    );

    let unit = EvidenceModeUnit::new(cfg.clone());
    let context = base_intel_context(client, user_message, route_decision, "", "", &[])
        .with_extra("narrative", narrative)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse evidence mode decision: {}", e))
}

pub(crate) async fn compact_evidence_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    scope: &ScopePlan,
    cmd: &str,
    output: &str,
) -> Result<EvidenceCompact> {
    let unit = EvidenceCompactorUnit::new(cfg.clone());
    let context = IntelContext::new(
        objective.to_string(),
        neutral_route_decision(),
        output.to_string(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("objective", objective)?
    .with_extra("purpose", purpose)?
    .with_extra("scope", scope)?
    .with_extra("cmd", cmd)?
    .with_extra("output", output)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse evidence compact output: {}", e))
}

pub(crate) async fn classify_artifacts_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    scope: &ScopePlan,
    evidence: &str,
) -> Result<ArtifactClassification> {
    let unit = ArtifactClassifierUnit::new(cfg.clone());
    let context = IntelContext::new(
        objective.to_string(),
        neutral_route_decision(),
        evidence.to_string(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("objective", objective)?
    .with_extra("scope", scope)?
    .with_extra("evidence", evidence)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse artifact classification: {}", e))
}

pub(crate) async fn present_result_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<String> {
    let unit = ResultPresenterUnit::new(cfg.clone());
    let context = base_intel_context(client, user_message, route_decision, "", "", &[])
        .with_extra("evidence_mode", evidence_mode)?
        .with_extra(
            "step_results",
            step_results.iter().map(step_result_json).collect::<Vec<_>>(),
        )?
        .with_extra("reply_instructions", reply_instructions)?;
    let output = unit.execute_with_fallback(&context).await?;
    Ok(output.get_str("final_text").unwrap_or_default().to_string())
}

pub(crate) async fn repair_command_once(
    client: &reqwest::Client,
    _chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    failed_cmd: &str,
    output: &str,
) -> Result<CommandRepair> {
    let unit = CommandRepairUnit::new(cfg.clone());
    let context = IntelContext::new(
        failed_cmd.to_string(),
        neutral_route_decision(),
        summarize_shell_output(output),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("objective", objective)?
    .with_extra("purpose", purpose)?
    .with_extra("output", output)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse command repair output: {}", e))
}
