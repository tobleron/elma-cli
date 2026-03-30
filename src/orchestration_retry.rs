//! @efficiency-role: service-orchestrator
//!
//! Retry Orchestration Module
//!
//! Handles retry logic with temperature escalation and meta-review synthesis.

use crate::*;
use crate::app::LoadedProfiles;

/// Retry orchestration with temperature escalation and prompt variants.
/// Returns the best program from all attempts, or a meta-review synthesized program.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn orchestrate_with_retries(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    initial_program: Program,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
    profiles: &LoadedProfiles,
    max_retries: u32,
    temp_step: f64,
    max_temp: f64,
) -> Result<AutonomousLoopOutcome> {
    use crate::defaults::get_retry_prompt_variant;

    let mut best_outcome: Option<AutonomousLoopOutcome> = None;
    let mut attempt_history: Vec<(u32, Program, String)> = Vec::new(); // (attempt, program, error)

    for attempt in 0..max_retries {
        // Calculate temperature for this attempt
        let temperature = (profiles.orchestrator_cfg.temperature + (attempt as f64 * temp_step)).min(max_temp);

        // Show retry intel summary
        show_intel_summary(
            args.show_process,
            &format!(
                "Retry {}/{} (temp={:.1}, strategy={})",
                attempt + 1,
                max_retries,
                temperature,
                match attempt {
                    0 => "standard",
                    1 => "step-by-step",
                    2 => "challenge",
                    _ => "simplify"
                }
            ),
        );

        // Build program with retry-specific prompt variant
        let retry_program = build_program_with_retry(
            client,
            chat_url,
            &profiles.orchestrator_cfg,
            temperature,
            get_retry_prompt_variant(attempt),
            messages,
            ws,
            ws_brief,
            route_decision,
            workflow_plan,
            complexity,
            scope,
            formula,
            &attempt_history,
        )
        .await?;

        // Execute the program
        let outcome = run_autonomous_loop(
            args,
            client,
            chat_url,
            session,
            workdir,
            retry_program.clone(),
            route_decision,
            workflow_plan,
            complexity,
            scope,
            formula,
            ws,
            ws_brief,
            messages,
            &profiles.orchestrator_cfg,
            &profiles.planner_cfg,
            &profiles.planner_master_cfg,
            &profiles.decider_cfg,
            &profiles.selector_cfg,
            &profiles.summarizer_cfg,
            &profiles.command_repair_cfg,
            &profiles.command_preflight_cfg,
            &profiles.task_semantics_guard_cfg,
            &profiles.evidence_compactor_cfg,
            &profiles.artifact_classifier_cfg,
            &profiles.outcome_verifier_cfg,
            &profiles.execution_sufficiency_cfg,
            &profiles.critic_cfg,
            &profiles.logical_reviewer_cfg,
            &profiles.efficiency_reviewer_cfg,
            &profiles.risk_reviewer_cfg,
            &profiles.refinement_cfg,
        )
        .await?;

        // Check if outcome is successful (has final reply and no critical failures)
        let is_successful = outcome.final_reply.is_some()
            && outcome.step_results.iter().all(|r| r.ok || r.kind.eq_ignore_ascii_case("reply"));

        if is_successful {
            show_intel_summary(
                args.show_process,
                &format!("Retry {} succeeded", attempt + 1),
            );
            return Ok(outcome);
        }

        // Record failure for meta-review
        let error_summary = outcome.step_results
            .iter()
            .filter(|r| !r.ok)
            .map(|r| format!("{}: {}", r.id, r.outcome_reason.as_deref().unwrap_or("failed")))
            .collect::<Vec<_>>()
            .join("; ");

        attempt_history.push((attempt, retry_program.clone(), error_summary));
        best_outcome = Some(outcome);
    }

    // All attempts failed - trigger meta-review
    show_intel_summary(
        args.show_process,
        &format!("All {} retries failed - triggering meta-review", max_retries),
    );

    let meta_program = synthesize_meta_review(
        client,
        chat_url,
        &profiles.meta_review_cfg,
        messages,
        ws,
        ws_brief,
        route_decision,
        &attempt_history,
    )
    .await?;

    // Execute the meta-review program
    run_autonomous_loop(
        args,
        client,
        chat_url,
        session,
        workdir,
        meta_program,
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &profiles.orchestrator_cfg,
        &profiles.planner_cfg,
        &profiles.planner_master_cfg,
        &profiles.decider_cfg,
        &profiles.selector_cfg,
        &profiles.summarizer_cfg,
        &profiles.command_repair_cfg,
        &profiles.command_preflight_cfg,
        &profiles.task_semantics_guard_cfg,
        &profiles.evidence_compactor_cfg,
        &profiles.artifact_classifier_cfg,
        &profiles.outcome_verifier_cfg,
        &profiles.execution_sufficiency_cfg,
        &profiles.critic_cfg,
        &profiles.logical_reviewer_cfg,
        &profiles.efficiency_reviewer_cfg,
        &profiles.risk_reviewer_cfg,
        &profiles.refinement_cfg,
    )
    .await
}

/// Build a program with retry-specific temperature and prompt variant.
async fn build_program_with_retry(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    temperature: f64,
    retry_prompt: &str,
    messages: &[ChatMessage],
    ws: &str,
    ws_brief: &str,
    route_decision: &RouteDecision,
    workflow_plan: Option<&WorkflowPlannerOutput>,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    attempt_history: &[(u32, Program, String)],
) -> Result<Program> {
    // Build enhanced prompt with failure history
    let mut prompt = build_orchestrator_user_content(
        &messages.last().map(|m| m.content.clone()).unwrap_or_default(),
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
    );

    // Add retry context
    if !attempt_history.is_empty() {
        prompt.push_str("\n\n=== PREVIOUS FAILED ATTEMPTS ===\n");
        for (attempt, _program, error) in attempt_history {
            prompt.push_str(&format!("\nAttempt {}: {}\n", attempt + 1, error));
        }
        prompt.push_str(&format!("\n{}\n", retry_prompt));
    }

    // Make request with adjusted temperature
    let request = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };

    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);
    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str = crate::routing::extract_first_json_object(&response_text)
        .unwrap_or(&response_text);
    parse_json_loose(json_str)
}

/// Synthesize a new program from meta-review of all failed attempts.
async fn synthesize_meta_review(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    messages: &[ChatMessage],
    ws: &str,
    ws_brief: &str,
    route_decision: &RouteDecision,
    attempt_history: &[(u32, Program, String)],
) -> Result<Program> {
    let mut prompt = String::new();
    prompt.push_str("=== TASK ===\n");
    prompt.push_str(&format!("User request: {}\n\n", messages.last().map(|m| m.content.clone()).unwrap_or_default()));

    prompt.push_str("=== FAILED ATTEMPTS ===\n");
    for (attempt, program, error) in attempt_history {
        prompt.push_str(&format!(
            "\nAttempt {} (strategy: {}):\n- Error: {}\n- Steps: {}\n",
            attempt + 1,
            match attempt {
                0 => "standard",
                1 => "step-by-step",
                2 => "challenge",
                _ => "simplify"
            },
            error,
            program.steps.iter().map(|s| format!("{} ({})", s.id(), s.kind())).collect::<Vec<_>>().join(", ")
        ));
    }

    prompt.push_str("\n=== WORKSPACE CONTEXT ===\n");
    prompt.push_str(ws);
    prompt.push_str("\n\n");
    prompt.push_str(ws_brief);

    prompt.push_str("\n\n=== ROUTE PRIOR ===\n");
    prompt.push_str(&format!("Suggested route: {}\n", route_decision.route));

    let request = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };

    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);
    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str = crate::routing::extract_first_json_object(&response_text)
        .unwrap_or(&response_text);
    parse_json_loose(json_str)
}
