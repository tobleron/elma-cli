//! @efficiency-role: service-orchestrator
//!
//! Retry Orchestration Module
//!
//! Handles retry logic with temperature escalation and meta-review synthesis.
//! Task 010: Integrated strategy chains for fallback-based retries.

use crate::app::LoadedProfiles;
use crate::*;

/// Retry orchestration with strategy chains and temperature escalation.
/// Returns the best program from all attempts, or a meta-review synthesized program.
///
/// Task 010: Now uses strategy fallback chains instead of just temperature escalation.
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

    // Task 010: Create strategy chain based on task characteristics
    let mut strategy_chain = select_strategy_chain(
        messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.as_str())
            .unwrap_or(""),
        complexity,
        route_decision,
    );

    trace(
        args,
        &format!(
            "strategy_chain_selected primary={:?} fallbacks={}",
            strategy_chain.primary,
            strategy_chain.fallbacks.len()
        ),
    );

    let mut best_outcome: Option<AutonomousLoopOutcome> = None;
    let mut attempt_history: Vec<(u32, Program, String)> = Vec::new(); // (attempt, program, error)

    for attempt in 0..max_retries {
        // Task 010: Get next strategy from chain (or use temperature escalation as before)
        let strategy = if let Some(s) = strategy_chain.next_strategy() {
            s
        } else {
            // Strategy chain exhausted, fall back to temperature escalation
            ExecutionStrategy::Direct
        };

        // Respect the already-built initial program on the first attempt ONLY IF strategy matches.
        // If the selected strategy is different from Direct, we should rebuild the program.
        let use_initial_program = attempt == 0 && strategy == ExecutionStrategy::Direct;

        // Calculate temperature for this attempt (adjusted by strategy)
        let base_temperature = profiles.orchestrator_cfg.temperature;
        let temp_adjustment = match strategy {
            ExecutionStrategy::Direct => 0.0,
            ExecutionStrategy::InspectFirst => -0.1,
            ExecutionStrategy::PlanThenExecute => 0.1,
            ExecutionStrategy::SafeMode => -0.2,
            ExecutionStrategy::Incremental => 0.0,
        };
        let temperature =
            (base_temperature + (attempt as f64 * temp_step) + temp_adjustment).min(max_temp);

        // Show retry intel summary with strategy
        show_intel_summary(
            args.show_process,
            &format!(
                "Retry {}/{} (temp={:.1}, strategy={:?})",
                attempt + 1,
                max_retries,
                temperature,
                strategy
            ),
        );

        trace(
            args,
            &format!(
                "orchestration_retry_attempt id={} strategy={:?} temp={:.2} use_initial={}",
                attempt + 1,
                strategy,
                temperature,
                use_initial_program
            ),
        );

        let retry_program = if use_initial_program {
            initial_program.clone()
        } else {
            // Build program with strategy-aware prompt
            build_program_with_strategy(
                client,
                chat_url,
                &profiles.orchestrator_cfg,
                strategy,
                temperature,
                messages,
                ws,
                ws_brief,
                route_decision,
                complexity,
                scope,
                formula,
                &attempt_history,
            )
            .await?
        };

        // Check for stale program generation
        let is_stale =
            !use_initial_program && attempt_history.iter().any(|(_, p, _)| p == &retry_program);
        if is_stale {
            trace(
                args,
                "orchestrator_stale_program=true reason=Generated program is identical to a previously failed attempt",
            );
            show_intel_summary(
                args.show_process,
                &format!("Attempt {} generated a stale program (identical to a previous failure). Switching strategy.", attempt + 1),
            );
            attempt_history.push((
                attempt,
                retry_program.clone(),
                "STALE_PROGRAM: Strategy generated an identical program to a previously failed attempt".to_string(),
            ));
            continue;
        }

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
            &profiles.status_message_cfg,
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
            && outcome
                .step_results
                .iter()
                .all(|r| r.ok || r.kind.eq_ignore_ascii_case("reply"));

        if is_successful {
            show_intel_summary(
                args.show_process,
                &format!("Retry {} succeeded", attempt + 1),
            );
            return Ok(outcome);
        }

        // Record failure for meta-review
        let error_summary = outcome
            .step_results
            .iter()
            .filter(|r| !r.ok)
            .map(|r| {
                format!(
                    "{}: {}",
                    r.id,
                    r.outcome_reason.as_deref().unwrap_or("failed")
                )
            })
            .collect::<Vec<_>>()
            .join("; ");

        attempt_history.push((attempt, retry_program.clone(), error_summary));
        best_outcome = Some(outcome);
    }

    // All attempts failed - trigger meta-review
    show_intel_summary(
        args.show_process,
        &format!(
            "All {} retries failed - triggering meta-review",
            max_retries
        ),
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
        &profiles.status_message_cfg,
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
    // Get tool registry for this workspace
    let workspace_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tool_registry = crate::tools::ToolRegistry::new(&workspace_path);

    // Select optimal formula for retry (slightly higher efficiency priority on retries)
    let formula_selection = crate::formulas::select_optimal_formula(
        &complexity.complexity,
        &complexity.risk,
        &route_decision.route,
        0.6, // Slightly more efficiency-focused on retry
    );

    // Build enhanced prompt with failure history
    let mut prompt = build_orchestrator_user_content(
        &messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default(),
        route_decision,
        workflow_plan,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &tool_registry,
        &formula_selection,
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
        grammar: None,
    };

    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);
    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str =
        crate::routing::extract_first_json_object(&response_text).unwrap_or(&response_text);
    parse_json_loose(json_str)
}

/// Build a program with strategy-aware prompt and temperature.
/// Task 010: Uses strategy-specific prompts from config/defaults/
async fn build_program_with_strategy(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    strategy: ExecutionStrategy,
    temperature: f64,
    messages: &[ChatMessage],
    ws: &str,
    ws_brief: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    attempt_history: &[(u32, Program, String)],
) -> Result<Program> {
    // Get tool registry for this workspace
    let workspace_path = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let tool_registry = crate::tools::ToolRegistry::new(&workspace_path);

    // Select optimal formula for this strategy
    let formula_selection = crate::formulas::select_optimal_formula(
        &complexity.complexity,
        &complexity.risk,
        &route_decision.route,
        0.6, // Slightly more efficiency-focused
    );

    // Build enhanced prompt with strategy context
    let mut prompt = build_orchestrator_user_content(
        &messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default(),
        route_decision,
        None, // workflow_plan
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
        &tool_registry,
        &formula_selection,
    );

    // Add strategy context
    prompt.push_str(&format!(
        "\n\n=== EXECUTION STRATEGY ===\nStrategy: {:?}\nGuidance: {}\n",
        strategy,
        strategy.hint()
    ));

    // Add failure history with strategy context
    if !attempt_history.is_empty() {
        prompt.push_str("\n\n=== PREVIOUS FAILED ATTEMPTS ===\n");
        for (attempt, _program, error) in attempt_history {
            prompt.push_str(&format!("\nAttempt {}: {}\n", attempt + 1, error));
        }
        prompt.push_str("\nTry a DIFFERENT approach based on the current strategy.\n");
    }

    // Make request with strategy-adjusted temperature
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
        grammar: None,
    };

    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);

    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str =
        crate::routing::extract_first_json_object(&response_text).unwrap_or(&response_text);
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
    prompt.push_str(&format!(
        "User request: {}\n\n",
        messages
            .last()
            .map(|m| m.content.clone())
            .unwrap_or_default()
    ));

    prompt.push_str("=== FAILED ATTEMPTS ===\n");
    for (attempt, program, error) in attempt_history {
        prompt.push_str(&format!(
            "\nAttempt {}:\n- Error: {}\n- Steps: {}\n",
            attempt + 1,
            error,
            program
                .steps
                .iter()
                .map(|s| format!("{} ({})", s.id(), s.kind()))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    prompt.push_str("\n=== WORKSPACE CONTEXT (TRUNCATED) ===\n");
    prompt.push_str(&ws.chars().take(4000).collect::<String>());
    prompt.push_str("\n\n");
    prompt.push_str(&ws_brief.chars().take(1000).collect::<String>());

    prompt.push_str("\n\n=== ROUTE PRIOR ===\n");
    prompt.push_str(&format!("Suggested route: {}\n", route_decision.route));

    prompt.push_str("\n\n=== INSTRUCTION ===\n");
    prompt.push_str("Analyze all failed attempts and synthesize a NEW approach that:\n");
    prompt.push_str("1. Avoids the mistakes from previous attempts\n");
    prompt.push_str("2. Uses a different strategy than those that failed\n");
    prompt.push_str("3. Is grounded in the workspace context provided\n");
    prompt.push_str("4. Has clear, achievable steps\n\n");
    prompt.push_str("Output ONLY valid Program JSON.\n");

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
        grammar: None,
    };

    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);
    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str =
        crate::routing::extract_first_json_object(&response_text).unwrap_or(&response_text);
    parse_json_loose(json_str)
}
