use crate::*;

pub(crate) async fn request_program_or_repair(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    prompt: &str,
    use_grammar: bool,
) -> Result<(Program, String)> {
    let grammar = if use_grammar {
        Some(json_program_grammar())
    } else {
        None
    };

    let orch_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: orchestrator_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ],
        temperature: orchestrator_cfg.temperature,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
        grammar,
    };
    let (program, json_text) = chat_json_with_repair_text_timeout(
        client,
        chat_url,
        &orch_req,
        orchestrator_cfg.timeout_s.min(45),
    )
    .await?;
    Ok((program, json_text))
}

pub(crate) async fn request_recovery_program(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    prompt: &str,
    failed_steps: &[StepResult], // NEW: Track failed steps to forbid repetition
) -> Result<Program> {
    // Build list of failed commands to explicitly forbid
    let failed_commands: Vec<String> = failed_steps
        .iter()
        .filter(|s| s.kind == "shell" && !s.ok)
        .filter_map(|s| s.command.clone())
        .collect();

    let failed_commands_str = if failed_commands.is_empty() {
        "None".to_string()
    } else {
        failed_commands
            .iter()
            .map(|c| format!("- {}", c))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let recovery_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "{}\n\nRECOVERY MODE:\n\
                    - A previous workflow attempt failed or was unusable.\n\
                    - Return ONLY one valid Program JSON object.\n\
                    - Do not output reply-only for a non-CHAT route unless asking one concise clarifying question is the only safe next step.\n\
                    - Use current_program_steps and observed_step_results to repair the workflow, not to restate or hallucinate completion.\n\
                    - DO NOT repeat steps that are already marked as successful ('ok': true) in observed_step_results.\n\
                    - DO NOT repeat previously FAILED commands (see list below).\n\
                    - If the task asks to choose, rank, prioritize, or select workspace items, inspect evidence first, then decide or summarize, then reply.\n\
                    - If a select step exists or should exist, later shell steps that consume that selection should normally reference it directly with a placeholder such as {{sel1|shell_words}}.\n\
                    - If the task asks to show file contents, inspect the selected files before replying.\n\
                    - Prefer the smallest valid program that can still satisfy the request.\n\n\
                    PREVIOUSLY FAILED COMMANDS (DO NOT REPEAT - USE DIFFERENT APPROACH):\n{}\n",
                    orchestrator_cfg.system_prompt,
                    failed_commands_str
                ),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens.min(1536),
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
        grammar: None,
    };
    chat_json_with_repair_timeout(
        client,
        chat_url,
        &recovery_req,
        orchestrator_cfg.timeout_s.min(45),
    )
    .await
}

pub(crate) async fn request_critic_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    _line: &str,
    _route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    _sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> Result<CriticVerdict> {
    let narrative = crate::intel_narrative::build_critic_narrative(
        &program.objective,
        program,
        step_results,
        attempt,
        2, // max_retries
    );

    let critic_req = ChatCompletionRequest {
        model: critic_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: critic_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: narrative, // Plain text narrative, not JSON
            },
        ],
        temperature: critic_cfg.temperature,
        top_p: critic_cfg.top_p,
        stream: false,
        max_tokens: critic_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(critic_cfg.repeat_penalty),
        reasoning_format: Some(critic_cfg.reasoning_format.clone()),
        grammar: None,
    };
    chat_json_with_repair_for_profile_timeout(
        client,
        chat_url,
        &critic_req,
        &critic_cfg.name,
        critic_cfg.timeout_s,
    )
    .await
}

pub(crate) async fn request_reviewer_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    reviewer_cfg: &Profile,
    program: &Program,
    step_results: &[StepResult],
    review_type: &str,
) -> Result<CriticVerdict> {
    let narrative = crate::intel_narrative::build_reviewer_narrative(
        &program.objective,
        program,
        step_results,
        review_type,
    );

    let reviewer_req = ChatCompletionRequest {
        model: reviewer_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: reviewer_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: narrative,
            },
        ],
        temperature: reviewer_cfg.temperature,
        top_p: reviewer_cfg.top_p,
        stream: false,
        max_tokens: reviewer_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(reviewer_cfg.repeat_penalty),
        reasoning_format: Some(reviewer_cfg.reasoning_format.clone()),
        grammar: None,
    };
    chat_json_with_repair_for_profile_timeout(
        client,
        chat_url,
        &reviewer_req,
        &reviewer_cfg.name,
        reviewer_cfg.timeout_s,
    )
    .await
}

pub(crate) async fn request_risk_review(
    client: &reqwest::Client,
    chat_url: &Url,
    risk_cfg: &Profile,
    _line: &str,
    _route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    _attempt: u32,
) -> Result<RiskReviewVerdict> {
    let narrative = crate::intel_narrative::build_reviewer_narrative(
        &program.objective,
        program,
        step_results,
        "risk",
    );

    let risk_req = ChatCompletionRequest {
        model: risk_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: risk_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: narrative,
            },
        ],
        temperature: risk_cfg.temperature,
        top_p: risk_cfg.top_p,
        stream: false,
        max_tokens: risk_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(risk_cfg.repeat_penalty),
        reasoning_format: Some(risk_cfg.reasoning_format.clone()),
        grammar: None,
    };
    chat_json_with_repair_for_profile_timeout(
        client,
        chat_url,
        &risk_req,
        &risk_cfg.name,
        risk_cfg.timeout_s,
    )
    .await
}

pub(crate) async fn request_chat_final_text(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    system_content: &str,
    line: &str,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<(String, Option<u64>)> {
    let reply_req = ChatCompletionRequest {
        model: elma_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_content.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": line,
                    "instructions": reply_instructions,
                    "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
                })
                .to_string(),
            },
        ],
        temperature: elma_cfg.temperature,
        top_p: elma_cfg.top_p,
        stream: false,
        max_tokens: elma_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(elma_cfg.repeat_penalty),
        reasoning_format: Some(elma_cfg.reasoning_format.clone()),
        grammar: None,
    };
    let parsed = chat_once_with_timeout(client, chat_url, &reply_req, elma_cfg.timeout_s).await?;
    let usage_total = parsed.usage.as_ref().and_then(|u| u.total_tokens);
    let msg = &parsed
        .choices
        .get(0)
        .context("No choices[0] in response")?
        .message;
    Ok((
        msg.content.as_deref().unwrap_or("").trim().to_string(),
        usage_total,
    ))
}

pub(crate) async fn maybe_revise_presented_result(
    client: &reqwest::Client,
    chat_url: &Url,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
    response_advice: &ExpertResponderAdvice,
    step_results: &[StepResult],
    reply_instructions: &str,
    final_text: String,
) -> String {
    if let Ok(verdict) = claim_check_once(
        client,
        chat_url,
        claim_checker_cfg,
        line,
        evidence_mode,
        step_results,
        &final_text,
    )
    .await
    {
        if verdict.status.eq_ignore_ascii_case("revise") {
            let revised = present_result_via_unit(
                client,
                presenter_cfg,
                line,
                route_decision,
                evidence_mode,
                response_advice,
                step_results,
                &format!(
                    "{}\n\nRevision guidance:\n{}",
                    reply_instructions,
                    if verdict.rewrite_instructions.trim().is_empty() {
                        verdict.reason.trim()
                    } else {
                        verdict.rewrite_instructions.trim()
                    }
                ),
            )
            .await
            .unwrap_or_default();
            if !revised.trim().is_empty() {
                return revised;
            }
        }
    }
    final_text
}

pub(crate) async fn decide_evidence_mode_via_unit(
    client: &reqwest::Client,
    evidence_mode_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
) -> Result<EvidenceModeDecision> {
    let has_command_request = user_message
        .to_lowercase()
        .split_whitespace()
        .any(|w| ["run", "execute", "show", "display", "print"].contains(&w));
    let has_command_execution = step_results
        .iter()
        .any(|s| s.command.as_ref().is_some_and(|c| !c.is_empty()));
    let has_artifact = step_results
        .iter()
        .any(|s| s.artifact_path.as_ref().is_some_and(|p| !p.is_empty()));

    let narrative = crate::intel_narrative::build_evidence_mode_narrative(
        user_message,
        route_decision,
        reply_instructions,
        step_results,
        has_command_request,
        has_command_execution,
        has_artifact,
    );

    let unit = EvidenceModeUnit::new(evidence_mode_cfg.clone());
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("narrative", narrative)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse evidence mode decision: {}", e))
}

pub(crate) async fn request_response_advice_via_unit(
    client: &reqwest::Client,
    expert_responder_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
    reply_instructions: &str,
    step_results: &[StepResult],
) -> Result<ExpertResponderAdvice> {
    let unit = ExpertResponderUnit::new(expert_responder_cfg.clone());
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("evidence_mode", evidence_mode)?
    .with_extra(
        "step_results",
        step_results.iter().map(step_result_json).collect::<Vec<_>>(),
    )?
    .with_extra("reply_instructions", reply_instructions)?;
    let output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse expert responder advice: {}", e))
}

pub(crate) async fn present_result_via_unit(
    client: &reqwest::Client,
    presenter_cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
    response_advice: &ExpertResponderAdvice,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<String> {
    let unit = ResultPresenterUnit::new(presenter_cfg.clone());
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("evidence_mode", evidence_mode)?
    .with_extra("response_advice", response_advice)?
    .with_extra(
        "step_results",
        step_results.iter().map(step_result_json).collect::<Vec<_>>(),
    )?
    .with_extra("reply_instructions", reply_instructions)?;
    let output = unit.execute_with_fallback(&context).await?;
    Ok(output.get_str("final_text").unwrap_or_default().to_string())
}

pub(crate) async fn maybe_format_final_text(
    client: &reqwest::Client,
    chat_url: &Url,
    formatter_cfg: &Profile,
    line: &str,
    final_text: String,
    usage_total: Option<u64>,
) -> (String, Option<u64>) {
    if user_requested_markdown(line) || !looks_like_markdown(&final_text) {
        return (final_text, usage_total);
    }

    let fmt_req = ChatCompletionRequest {
        model: formatter_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: formatter_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: final_text.clone(),
            },
        ],
        temperature: formatter_cfg.temperature,
        top_p: formatter_cfg.top_p,
        stream: false,
        max_tokens: formatter_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(formatter_cfg.repeat_penalty),
        reasoning_format: Some(formatter_cfg.reasoning_format.clone()),
        grammar: None,
    };
    if let Ok(fmt_resp) = chat_once_with_timeout(client, chat_url, &fmt_req, formatter_cfg.timeout_s).await {
        let next_usage = fmt_resp
            .usage
            .as_ref()
            .and_then(|u| u.total_tokens)
            .or(usage_total);
        let formatted = extract_response_text(&fmt_resp);
        if !formatted.trim().is_empty() {
            return (formatted.trim().to_string(), next_usage);
        }
        return (final_text, next_usage);
    }
    (final_text, usage_total)
}

pub(crate) async fn request_judge_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    judge_cfg: &Profile,
    scenario: &CalibrationScenario,
    user_message: &str,
    step_results: &[StepResult],
    final_text: &str,
) -> Result<CalibrationJudgeVerdict> {
    let req = ChatCompletionRequest {
        model: judge_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: judge_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "scenario_notes": scenario.notes,
                    "expected_route": scenario.route,
                    "expected_speech_act": scenario.speech_act,
                    "user_message": user_message,
                    "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
                    "final_answer": final_text,
                    "markdown_requested": user_requested_markdown(user_message),
                })
                .to_string(),
            },
        ],
        temperature: judge_cfg.temperature,
        top_p: judge_cfg.top_p,
        stream: false,
        max_tokens: judge_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(judge_cfg.repeat_penalty),
        reasoning_format: Some(judge_cfg.reasoning_format.clone()),
        grammar: None,
    };
    chat_json_with_repair(client, chat_url, &req).await
}
