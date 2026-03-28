use crate::*;

pub(crate) async fn request_program_or_repair(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    prompt: &str,
) -> Result<(Program, String)> {
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
    };
    let orch_resp = chat_once(client, chat_url, &orch_req).await?;
    let orch_text = extract_response_text(&orch_resp);

    if let Ok(program) = parse_json_loose(&orch_text) {
        return Ok((program, orch_text));
    }

    let repair_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: orchestrator_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Your previous answer was invalid. Return ONLY a valid Program JSON object for this request.\n\n{}\n\nPrevious invalid output:\n{}",
                    prompt,
                    orch_text.trim()
                ),
            },
        ],
        temperature: orchestrator_cfg.temperature,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
    };
    let repaired = chat_once(client, chat_url, &repair_req).await?;
    let repaired_text = extract_response_text(&repaired);
    let program = parse_json_loose(&repaired_text)?;
    Ok((program, repaired_text))
}

pub(crate) async fn request_recovery_program(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    prompt: &str,
) -> Result<Program> {
    let recovery_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: format!(
                    "{}\n\nRECOVERY MODE:\n- A previous workflow attempt failed or was unusable.\n- Return ONLY one valid Program JSON object.\n- Do not output reply-only for a non-CHAT route unless asking one concise clarifying question is the only safe next step.\n- Use current_program_steps and observed_step_results to repair the workflow, not to restate or hallucinate completion.\n- If the task asks to choose, rank, prioritize, or select workspace items, inspect evidence first, then decide or summarize, then reply.\n- If a select step exists or should exist, later shell steps that consume that selection should normally reference it directly with a placeholder such as {{sel1|shell_words}}.\n- If the task asks to show file contents, inspect the selected files before replying.\n- Prefer the smallest valid program that can still satisfy the request.",
                    orchestrator_cfg.system_prompt
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
    };
    chat_json_with_repair(client, chat_url, &recovery_req).await
}

pub(crate) async fn request_critic_verdict(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    sufficiency: Option<&ExecutionSufficiencyVerdict>,
    attempt: u32,
) -> Result<CriticVerdict> {
    let critic_req = ChatCompletionRequest {
        model: critic_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: critic_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": line,
                    "objective": program.objective,
                    "speech_act_prior": {
                        "choice": route_decision.speech_act.choice,
                        "source": route_decision.speech_act.source,
                        "distribution": route_decision.speech_act.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.speech_act.margin,
                        "entropy": route_decision.speech_act.entropy,
                    },
                    "workflow_prior": {
                        "choice": route_decision.workflow.choice,
                        "source": route_decision.workflow.source,
                        "distribution": route_decision.workflow.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.workflow.margin,
                        "entropy": route_decision.workflow.entropy,
                    },
                    "mode_prior": {
                        "choice": route_decision.mode.choice,
                        "source": route_decision.mode.source,
                        "distribution": route_decision.mode.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.mode.margin,
                        "entropy": route_decision.mode.entropy,
                    },
                    "route_prior": {
                        "route": route_decision.route,
                        "source": route_decision.source,
                        "distribution": route_decision.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.margin,
                        "entropy": route_decision.entropy,
                    },
                    "attempt": attempt,
                    "sufficiency_verdict": sufficiency.map(|verdict| {
                        serde_json::json!({
                            "status": verdict.status,
                            "reason": verdict.reason,
                        })
                    }),
                    "program_steps": program.steps.iter().map(program_step_json).collect::<Vec<_>>(),
                    "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
                })
                .to_string(),
            },
        ],
        temperature: critic_cfg.temperature,
        top_p: critic_cfg.top_p,
        stream: false,
        max_tokens: critic_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(critic_cfg.repeat_penalty),
        reasoning_format: Some(critic_cfg.reasoning_format.clone()),
    };
    chat_json_with_repair(client, chat_url, &critic_req).await
}

pub(crate) async fn request_risk_review(
    client: &reqwest::Client,
    chat_url: &Url,
    risk_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    attempt: u32,
) -> Result<RiskReviewVerdict> {
    let risk_req = ChatCompletionRequest {
        model: risk_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: risk_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": line,
                    "route": route_decision.route,
                    "attempt": attempt,
                    "program_steps": program.steps.iter().map(program_step_json).collect::<Vec<_>>(),
                    "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
                })
                .to_string(),
            },
        ],
        temperature: risk_cfg.temperature,
        top_p: risk_cfg.top_p,
        stream: false,
        max_tokens: risk_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(risk_cfg.repeat_penalty),
        reasoning_format: Some(risk_cfg.reasoning_format.clone()),
    };
    chat_json_with_repair(client, chat_url, &risk_req).await
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
    };
    let parsed = chat_once(client, chat_url, &reply_req).await?;
    let usage_total = parsed.usage.as_ref().and_then(|u| u.total_tokens);
    let msg = &parsed
        .choices
        .get(0)
        .context("No choices[0] in response")?
        .message;
    Ok((msg.content.as_deref().unwrap_or("").trim().to_string(), usage_total))
}

pub(crate) async fn maybe_revise_presented_result(
    client: &reqwest::Client,
    chat_url: &Url,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
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
            let revised = present_result_once(
                client,
                chat_url,
                presenter_cfg,
                line,
                route_decision,
                evidence_mode,
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
    };
    if let Ok(fmt_resp) = chat_once(client, chat_url, &fmt_req).await {
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
    };
    chat_json_with_repair(client, chat_url, &req).await
}
