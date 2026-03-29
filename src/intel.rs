use crate::*;

pub(crate) async fn assess_complexity_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ComplexityAssessment> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route_prior": {
                        "route": route_decision.route,
                        "distribution": route_decision.distribution.iter().map(|(route, p)| serde_json::json!({"route": route, "p": p})).collect::<Vec<_>>(),
                    },
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn build_scope_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ScopePlan> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "speech_act": route_decision.speech_act.choice,
                    "complexity": complexity,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn select_formula_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> Result<FormulaSelection> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "speech_act": route_decision.speech_act.choice,
                    "route": route_decision.route,
                    "complexity": complexity,
                    "scope": scope,
                    "memory_candidates": memories.iter().map(|m| {
                        serde_json::json!({
                            "id": m.id,
                            "title": m.title,
                            "route": m.route,
                            "complexity": m.complexity,
                            "formula": m.formula,
                            "objective": m.objective,
                            "example_user_message": m.user_message,
                            "program_signature": m.program_signature,
                            "success_count": m.success_count,
                            "failure_count": m.failure_count,
                            "last_success_unix_s": m.last_success_unix_s,
                            "artifact_mode_capable": m.artifact_mode_capable,
                            "active_run_id": m.active_run_id,
                        })
                    }).collect::<Vec<_>>(),
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn plan_workflow_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    _memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> Result<WorkflowPlannerOutput> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "speech_act": {
                        "choice": route_decision.speech_act.choice,
                        "distribution": route_decision.speech_act.distribution.iter().map(|(label, p)| serde_json::json!({"label": label, "p": p})).collect::<Vec<_>>(),
                    },
                    "workflow": {
                        "choice": route_decision.workflow.choice,
                        "distribution": route_decision.workflow.distribution.iter().map(|(label, p)| serde_json::json!({"label": label, "p": p})).collect::<Vec<_>>(),
                    },
                    "mode": {
                        "choice": route_decision.mode.choice,
                        "distribution": route_decision.mode.distribution.iter().map(|(label, p)| serde_json::json!({"label": label, "p": p})).collect::<Vec<_>>(),
                    },
                    "route": route_decision.route,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn select_items_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    instructions: &str,
    evidence: &str,
) -> Result<SelectionOutput> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "purpose": purpose,
                    "instructions": instructions,
                    "evidence": evidence,
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
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
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "speech_act": route_decision.speech_act.choice,
                    "reply_instructions": reply_instructions,
                    "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn compact_evidence_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    scope: &ScopePlan,
    cmd: &str,
    output: &str,
) -> Result<EvidenceCompact> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "purpose": purpose,
                    "scope": scope,
                    "cmd": cmd,
                    "output": output,
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn classify_artifacts_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    scope: &ScopePlan,
    evidence: &str,
) -> Result<ArtifactClassification> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "scope": scope,
                    "evidence": evidence,
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn present_result_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    evidence_mode: &EvidenceModeDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<String> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "speech_act": route_decision.speech_act.choice,
                    "evidence_mode": evidence_mode,
                    "instructions": reply_instructions,
                    "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
                })
                .to_string(),
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
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(text)
}

pub(crate) async fn repair_command_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    failed_cmd: &str,
    output: &str,
) -> Result<CommandRepair> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "purpose": purpose,
                    "failed_cmd": failed_cmd,
                    "stderr_or_output": summarize_shell_output(output),
                })
                .to_string(),
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
    chat_json_with_repair(client, chat_url, &req).await
}
