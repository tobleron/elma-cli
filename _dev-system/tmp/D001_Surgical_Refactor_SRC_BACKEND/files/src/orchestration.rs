use crate::*;

pub(crate) async fn orchestrate_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<(Program, String)> {
    let prompt = build_orchestrator_user_content(
        line,
        route_decision,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
    );
    let orch_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: orchestrator_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
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
    let orch_text = orch_resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();

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
    let repaired_text = repaired
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    let program = parse_json_loose(&repaired_text)?;
    Ok((program, repaired_text))
}

pub(crate) async fn run_critic_once(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
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
                    "program_steps": program.steps.iter().map(|s| {
                        serde_json::json!({
                            "id": step_id(s),
                            "type": step_kind(s),
                            "purpose": step_purpose(s),
                            "depends_on": step_depends_on(s),
                            "success_condition": step_success_condition(s),
                        })
                    }).collect::<Vec<_>>(),
                    "step_results": step_results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "type": r.kind,
                            "purpose": r.purpose,
                            "depends_on": r.depends_on,
                            "success_condition": r.success_condition,
                            "ok": r.ok,
                            "summary": r.summary,
                        })
                    }).collect::<Vec<_>>(),
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
    let verdict_resp = chat_once(client, chat_url, &critic_req).await?;
    let verdict_text = verdict_resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&verdict_text)
}

pub(crate) async fn generate_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    formatter_cfg: &Profile,
    system_content: &str,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<(String, Option<u64>)> {
    let mut usage_total: Option<u64> = None;
    let mut final_text = if route_decision.route.eq_ignore_ascii_case("CHAT") {
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
                        "step_results": step_results.iter().map(|r| {
                            serde_json::json!({
                                "id": r.id,
                                "type": r.kind,
                                "ok": r.ok,
                                "summary": r.summary,
                            })
                        }).collect::<Vec<_>>(),
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
        usage_total = parsed.usage.as_ref().and_then(|u| u.total_tokens);
        let msg = &parsed
            .choices
            .get(0)
            .context("No choices[0] in response")?
            .message;
        msg.content.as_deref().unwrap_or("").trim().to_string()
    } else {
        present_result_once(
            client,
            chat_url,
            presenter_cfg,
            line,
            route_decision,
            step_results,
            reply_instructions,
        )
        .await
        .unwrap_or_default()
    };

    if !route_decision.route.eq_ignore_ascii_case("CHAT") && !final_text.trim().is_empty() {
        if let Ok(verdict) = claim_check_once(
            client,
            chat_url,
            claim_checker_cfg,
            line,
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
                    final_text = revised;
                }
            }
        }
    }
    if !user_requested_markdown(line) && looks_like_markdown(&final_text) {
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
            usage_total = fmt_resp
                .usage
                .as_ref()
                .and_then(|u| u.total_tokens)
                .or(usage_total);
            let formatted = fmt_resp
                .choices
                .get(0)
                .and_then(|c| {
                    c.message
                        .content
                        .clone()
                        .or(c.message.reasoning_content.clone())
                })
                .unwrap_or_default();
            if !formatted.trim().is_empty() {
                final_text = formatted.trim().to_string();
            }
        }
    }
    Ok((final_text, usage_total))
}

pub(crate) async fn judge_final_answer_once(
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
                    "step_results": step_results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "type": r.kind,
                            "ok": r.ok,
                            "summary": r.summary,
                        })
                    }).collect::<Vec<_>>(),
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
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}
