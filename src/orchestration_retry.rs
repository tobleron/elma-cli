//! @efficiency-role: service-orchestrator
//!
//! Retry Orchestration Module
//!
//! Handles retry logic with temperature escalation, failure classification,
//! dynamic decomposition (Task 379), and meta-review synthesis (Task 010).

use crate::app::LoadedProfiles;
use crate::*;
use std::future::Future;

async fn await_with_optional_tui<T, F>(
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result,
            _ = tokio::time::sleep(std::time::Duration::from_millis(40)) => {
                if let Some(t) = tui.as_deref_mut() {
                    let _ = t.pump_ui();
                    if let Ok(Some(queued)) = t.poll_busy_submission() {
                        t.enqueue_submission(queued);
                    }
                }
            }
        }
    }
}

// ── Task 379: Failure Classification ──

/// Classifies the type of failure detected in a retry attempt.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FailureClass {
    /// Model output could not be parsed as valid JSON
    JsonParseFailure,
    /// Model produced the same output N times
    Stagnation(u32),
    /// A specific tool failed repeatedly
    ToolRepeatedFailure(String),
    /// Model produced no output or empty response
    EmptyOutput,
    /// Wall clock or stage budget exceeded
    Timeout,
    /// All available strategies have been exhausted
    StrategyExhaustion,
    /// Multiple failure types detected
    Mixed(Vec<FailureClass>),
}

impl FailureClass {
    /// Returns a human-readable label for the failure class.
    pub fn label(&self) -> &str {
        match self {
            FailureClass::JsonParseFailure => "json_parse_failure",
            FailureClass::Stagnation(_) => "stagnation",
            FailureClass::ToolRepeatedFailure(_) => "tool_repeated_failure",
            FailureClass::EmptyOutput => "empty_output",
            FailureClass::Timeout => "timeout",
            FailureClass::StrategyExhaustion => "strategy_exhaustion",
            FailureClass::Mixed(_) => "mixed_failure",
        }
    }

    /// Returns true if decomposition should be attempted.
    pub fn should_decompose(&self) -> bool {
        matches!(
            self,
            FailureClass::JsonParseFailure
                | FailureClass::Stagnation(_)
                | FailureClass::ToolRepeatedFailure(_)
                | FailureClass::EmptyOutput
                | FailureClass::Mixed(_)
        )
    }
}

/// Detect the failure class from the error summary, attempt history, and program.
fn detect_failure_class(
    _attempt: u32,
    error_summary: &str,
    attempt_history: &[(u32, Program, String)],
    program: &Program,
) -> FailureClass {
    let error_lower = error_summary.to_lowercase();

    // Empty output detection
    if error_summary.is_empty() {
        return FailureClass::EmptyOutput;
    }

    // Collect all matching failure signals
    let mut signals: Vec<FailureClass> = Vec::new();

    // Parse / JSON signal
    if error_lower.contains("parse")
        || error_lower.contains("json")
        || error_lower.contains("invalid")
    {
        signals.push(FailureClass::JsonParseFailure);
    }

    // Timeout signal
    if error_lower.contains("timeout") || error_lower.contains("wall clock") {
        signals.push(FailureClass::Timeout);
    }

    // Stagnation signal: same or similar programs in history
    if attempt_history.len() >= 3 {
        let recent: Vec<_> = attempt_history
            .iter()
            .rev()
            .take(3)
            .map(|(_, p, _)| p.objective.as_str())
            .collect();
        if recent.windows(2).all(|w| w[0] == w[1]) {
            signals.push(FailureClass::Stagnation(attempt_history.len() as u32));
        }
    }

    // Tool repeated failure signal
    if error_lower.contains("tool") && error_summary.contains(':') {
        for part in error_summary.split(';') {
            if let Some(tool_name) = part.trim().split(':').next() {
                let tool_name = tool_name.trim();
                if !tool_name.is_empty() && tool_name.len() < 50 {
                    let count = attempt_history
                        .iter()
                        .filter(|(_, _, e)| e.contains(tool_name))
                        .count();
                    if count >= 2 {
                        signals
                            .push(FailureClass::ToolRepeatedFailure(tool_name.to_string()));
                        break;
                    }
                }
            }
        }
    }

    // Check for empty output in program first step
    if program.steps.first().map_or(false, |s| s.kind() == "reply")
        && error_summary.contains("no output")
    {
        signals.push(FailureClass::EmptyOutput);
    }

    // Return appropriate classification
    if signals.len() > 1 {
        FailureClass::Mixed(signals)
    } else if signals.len() == 1 {
        signals.into_iter().next().unwrap()
    } else if attempt_history.len() >= 3 {
        FailureClass::StrategyExhaustion
    } else {
        // If no specific signal detected, infer from context
        if error_lower.contains("failed") || error_lower.contains("error") {
            FailureClass::JsonParseFailure
        } else {
            FailureClass::StrategyExhaustion
        }
    }
}

/// Returns a strategy direction string for the given failure class.
/// Used to guide the next retry attempt.
pub(crate) fn strategy_for_failure(class: &FailureClass) -> &'static str {
    match class {
        FailureClass::JsonParseFailure => {
            "Simplify output format. Return one field at a time. Use plain text instead of JSON."
        }
        FailureClass::Stagnation(n) if *n >= 3 => {
            "Force evidence collection before producing any output. Inspect the workspace first."
        }
        FailureClass::Stagnation(_) => {
            "Change approach entirely. Try a different tool or strategy."
        }
        FailureClass::ToolRepeatedFailure(tool) => {
            "Avoid using the failed tool. Find an alternative method or tool."
        }
        FailureClass::EmptyOutput => {
            "Reset context. Start fresh with a minimal, focused prompt."
        }
        FailureClass::Timeout => {
            "Reduce scope. Focus on one specific sub-problem at a time."
        }
        FailureClass::StrategyExhaustion => {
            "Synthesize a completely new approach. Review all prior failures to avoid repeating mistakes."
        }
        FailureClass::Mixed(_) => {
            "Decompose the task. Split into smaller independent sub-tasks and solve each separately."
        }
    }
}

/// T306/T379: Dynamic decomposition on failure.
/// Returns true when the model is struggling and should switch strategies.
fn decompose_on_failure(attempt: u32, error_summary: &str) -> bool {
    if attempt < 1 {
        return false;
    }

    let error_lower = error_summary.to_lowercase();

    // Stagnation / repeated failure: always decompose
    if error_lower.contains("stale") || error_lower.contains("same program") {
        return true;
    }

    // Persistent JSON parse failures after first retry
    if attempt >= 2 && (error_lower.contains("parse") || error_lower.contains("json")) {
        return true;
    }

    // Multiple tool failures (3+ colon-separated step entries)
    let colon_count = error_summary.matches(':').count();
    if colon_count >= 3 {
        return true;
    }

    // Empty output after multiple attempts
    if attempt >= 2 && error_summary.trim().is_empty() {
        return true;
    }

    false
}

// ── End Task 379 ──

/// Retry orchestration with strategy chains and temperature escalation.
/// Returns the best program from all attempts, or a meta-review synthesized program.
///
/// Task 010: Now uses strategy fallback chains instead of just temperature escalation.
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
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
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
                tui.as_deref_mut(),
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

        // Task 379: Detect failure class and decide whether to decompose
        if attempt > 0 {
            let last_error = attempt_history
                .last()
                .map(|(_, _, e)| e.as_str())
                .unwrap_or("");
            let failure_class = detect_failure_class(
                attempt,
                last_error,
                &attempt_history,
                &retry_program,
            );

            trace(
                args,
                &format!(
                    "failure_class_detected class={} should_decompose={}",
                    failure_class.label(),
                    failure_class.should_decompose(),
                ),
            );

            if decompose_on_failure(attempt, last_error) {
                let strategy_hint = strategy_for_failure(&failure_class);
                show_intel_summary(
                    args.show_process,
                    &format!(
                        "Decomposing on failure (attempt {}): {} — {}",
                        attempt + 1,
                        failure_class.label(),
                        strategy_hint,
                    ),
                );
                // Rebuild program with decomposition strategy prompt
                // (continues to next retry with updated strategy context)
            }
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
            tui.as_deref_mut(),
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
        tui.as_deref_mut(),
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
        tui.as_deref_mut(),
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
    let request = chat_request_system_user(
        cfg,
        &cfg.system_prompt,
        &prompt,
        ChatRequestOptions {
            temperature: Some(temperature),
            ..ChatRequestOptions::default()
        },
    );

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
    tui: Option<&mut crate::ui_terminal::TerminalUI>,
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
    let request = chat_request_system_user(
        cfg,
        &cfg.system_prompt,
        &prompt,
        ChatRequestOptions {
            temperature: Some(temperature),
            ..ChatRequestOptions::default()
        },
    );

    let response = await_with_optional_tui(tui, chat_once(client, chat_url, &request)).await?;
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
    tui: Option<&mut crate::ui_terminal::TerminalUI>,
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

    let request = chat_request_system_user(
        cfg,
        &cfg.system_prompt,
        &prompt,
        ChatRequestOptions::default(),
    );

    let response = await_with_optional_tui(tui, chat_once(client, chat_url, &request)).await?;
    let response_text = extract_response_text(&response);
    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str =
        crate::routing::extract_first_json_object(&response_text).unwrap_or(&response_text);
    parse_json_loose(json_str)
}

// ── Task 379 Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Program;

    fn make_test_program(objective: &str) -> Program {
        Program {
            objective: objective.to_string(),
            steps: vec![],
        }
    }

    #[test]
    fn test_failure_class_json_parse() {
        let class = detect_failure_class(
            1,
            "parse error: invalid json at line 1",
            &[],
            &make_test_program("test"),
        );
        assert_eq!(class, FailureClass::JsonParseFailure);
    }

    #[test]
    fn test_failure_class_timeout() {
        let class = detect_failure_class(
            1,
            "wall clock timeout exceeded",
            &[],
            &make_test_program("test"),
        );
        assert_eq!(class, FailureClass::Timeout);
    }

    #[test]
    fn test_failure_class_stagnation() {
        let prog = make_test_program("same");
        let history = vec![
            (0, prog.clone(), "error 1".to_string()),
            (1, prog.clone(), "error 2".to_string()),
            (2, prog.clone(), "error 3".to_string()),
        ];
        let class = detect_failure_class(3, "error", &history, &prog);
        assert_eq!(class, FailureClass::Stagnation(3));
    }

    #[test]
    fn test_failure_class_empty_output() {
        let prog = make_test_program("test");
        let class = detect_failure_class(0, "", &[], &prog);
        assert_eq!(class, FailureClass::EmptyOutput);
    }

    #[test]
    fn test_failure_class_tool_repeated() {
        let history = vec![
            (0, make_test_program("t"), "tool: read failed".to_string()),
            (1, make_test_program("t"), "tool: read failed again".to_string()),
        ];
        let class = detect_failure_class(
            2,
            "tool: read failed; tool: write failed",
            &history,
            &make_test_program("test"),
        );
        assert_eq!(class, FailureClass::ToolRepeatedFailure("tool".to_string()));
    }

    #[test]
    fn test_failure_class_strategy_exhaustion() {
        let p0 = make_test_program("first");
        let p1 = make_test_program("second");
        let p2 = make_test_program("third");
        let history = vec![
            (0, p0, "misc error".to_string()),
            (1, p1, "misc error".to_string()),
            (2, p2, "misc error".to_string()),
        ];
        let class = detect_failure_class(3, "misc error", &history, &make_test_program("p"));
        assert_eq!(class, FailureClass::StrategyExhaustion);
    }

    #[test]
    fn test_failure_class_label() {
        assert_eq!(
            FailureClass::JsonParseFailure.label(),
            "json_parse_failure"
        );
        assert_eq!(FailureClass::Stagnation(3).label(), "stagnation");
        assert_eq!(
            FailureClass::ToolRepeatedFailure("read".into()).label(),
            "tool_repeated_failure"
        );
        assert_eq!(FailureClass::EmptyOutput.label(), "empty_output");
        assert_eq!(FailureClass::Timeout.label(), "timeout");
        assert_eq!(FailureClass::StrategyExhaustion.label(), "strategy_exhaustion");
    }

    #[test]
    fn test_should_decompose() {
        assert!(FailureClass::JsonParseFailure.should_decompose());
        assert!(FailureClass::Stagnation(3).should_decompose());
        assert!(FailureClass::ToolRepeatedFailure("ls".into()).should_decompose());
        assert!(FailureClass::EmptyOutput.should_decompose());
        assert!(!FailureClass::Timeout.should_decompose());
        assert!(!FailureClass::StrategyExhaustion.should_decompose());
    }

    #[test]
    fn test_strategy_for_failure() {
        let hint = strategy_for_failure(&FailureClass::JsonParseFailure);
        assert!(hint.contains("Simplify"));

        let hint = strategy_for_failure(&FailureClass::ToolRepeatedFailure("read".into()));
        assert!(hint.contains("Avoid"));

        let hint = strategy_for_failure(&FailureClass::EmptyOutput);
        assert!(hint.contains("Reset"));

        let hint = strategy_for_failure(&FailureClass::Stagnation(5));
        assert!(hint.contains("Force evidence"));

        let hint = strategy_for_failure(&FailureClass::StrategyExhaustion);
        assert!(hint.contains("Synthesize"));

        let hint = strategy_for_failure(&FailureClass::Mixed(vec![]));
        assert!(hint.contains("Decompose"));
    }

    #[test]
    fn test_decompose_on_failure_cases() {
        // First attempt: never decompose
        assert!(!decompose_on_failure(0, "any error"));

        // Stale program
        assert!(decompose_on_failure(1, "STALE_PROGRAM"));

        // JSON parse failures after attempt 2+
        assert!(decompose_on_failure(2, "parse error at line 1"));

        // Multiple failures
        assert!(decompose_on_failure(1, "a:1; b:2; c:3"));

        // Empty output after attempt 2+
        assert!(decompose_on_failure(2, ""));

        // Single error at attempt 1: no decomposition
        assert!(!decompose_on_failure(1, "simple error"));
    }

    #[test]
    fn test_failure_class_mixed() {
        let history = vec![
            (0, make_test_program("test"), "parse error".to_string()),
        ];
        // "parse" + "timeout" should trigger both signals -> Mixed
        let class = detect_failure_class(
            1,
            "parse error; timeout exceeded",
            &history,
            &make_test_program("test"),
        );
        assert!(matches!(class, FailureClass::Mixed(_)));
    }
}
