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
    chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await
}

pub(crate) async fn assess_evidence_needs_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<(bool, bool)> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                }).to_string(),
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
    let result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await?;
    let needs_evidence = result.get("needs_evidence").and_then(|v| v.as_bool()).unwrap_or(false);
    let needs_tools = result.get("needs_tools").and_then(|v| v.as_bool()).unwrap_or(false);
    Ok((needs_evidence, needs_tools))
}

pub(crate) async fn assess_action_needs_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<(bool, bool)> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                }).to_string(),
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
    let result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await?;
    let needs_decision = result.get("needs_decision").and_then(|v| v.as_bool()).unwrap_or(false);
    let needs_plan = result.get("needs_plan").and_then(|v| v.as_bool()).unwrap_or(false);
    Ok((needs_decision, needs_plan))
}

pub(crate) async fn suggest_pattern_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    messages: &[ChatMessage],
) -> Result<String> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "conversation": conversation_excerpt(messages, 12),
                }).to_string(),
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
    let result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await?;
    Ok(result.get("suggested_pattern").and_then(|v| v.as_str()).unwrap_or("reply_only").to_string())
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
    // First build the scope objective
    let objective_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
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
                }).to_string(),
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
    let objective_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &objective_req, cfg.timeout_s).await?;
    let objective = objective_result.get("objective").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Then build the scope details
    let scope_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
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
                }).to_string(),
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
    let scope_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &scope_req, cfg.timeout_s).await?;
    
    Ok(ScopePlan {
        objective,
        focus_paths: scope_result.get("focus_paths").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
        include_globs: scope_result.get("include_globs").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
        exclude_globs: scope_result.get("exclude_globs").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
        query_terms: scope_result.get("query_terms").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
        expected_artifacts: scope_result.get("expected_artifacts").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
        reason: scope_result.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    })
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
    // First match to a formula memory if applicable
    let memory_match_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
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
                            "id": m.id, "title": m.title, "route": m.route, "complexity": m.complexity,
                            "formula": m.formula, "objective": m.objective, "example_user_message": m.user_message,
                            "program_signature": m.program_signature, "success_count": m.success_count,
                            "failure_count": m.failure_count, "last_success_unix_s": m.last_success_unix_s,
                            "artifact_mode_capable": m.artifact_mode_capable, "active_run_id": m.active_run_id,
                        })
                    }).collect::<Vec<_>>(),
                    "conversation": conversation_excerpt(messages, 12),
                }).to_string(),
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
    let memory_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &memory_match_req, cfg.timeout_s).await?;
    let memory_id = memory_result.get("memory_id").and_then(|v| v.as_str()).unwrap_or("").to_string();

    // Then select the formula
    let selector_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
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
                            "id": m.id, "title": m.title, "route": m.route, "complexity": m.complexity,
                            "formula": m.formula, "objective": m.objective, "example_user_message": m.user_message,
                            "program_signature": m.program_signature, "success_count": m.success_count,
                            "failure_count": m.failure_count, "last_success_unix_s": m.last_success_unix_s,
                            "artifact_mode_capable": m.artifact_mode_capable, "active_run_id": m.active_run_id,
                        })
                    }).collect::<Vec<_>>(),
                    "conversation": conversation_excerpt(messages, 12),
                }).to_string(),
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
    let selector_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &selector_req, cfg.timeout_s).await?;
    
    Ok(FormulaSelection {
        primary: selector_result.get("primary").and_then(|v| v.as_str()).unwrap_or("reply_only").to_string(),
        alternatives: selector_result.get("alternatives").and_then(|v| v.as_array())
            .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
        reason: selector_result.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        memory_id,
    })
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
    // Plan scope and evidence needs
    let scope_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "speech_act": {"choice": route_decision.speech_act.choice, "distribution": route_decision.speech_act.distribution.iter().map(|(label, p)| serde_json::json!({"label": label, "p": p})).collect::<Vec<_>>()},
                    "workflow": {"choice": route_decision.workflow.choice, "distribution": route_decision.workflow.distribution.iter().map(|(label, p)| serde_json::json!({"label": label, "p": p})).collect::<Vec<_>>()},
                    "mode": {"choice": route_decision.mode.choice, "distribution": route_decision.mode.distribution.iter().map(|(label, p)| serde_json::json!({"label": label, "p": p})).collect::<Vec<_>>()},
                    "route": route_decision.route,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                }).to_string(),
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
    let scope_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &scope_req, cfg.timeout_s).await?;
    
    // Plan complexity and risk
    let complexity_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                }).to_string(),
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
    let complexity_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &complexity_req, cfg.timeout_s).await?;
    
    // Get reason
    let reason_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                }).to_string(),
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
    let reason_result: serde_json::Value = chat_json_with_repair_timeout(client, chat_url, &reason_req, cfg.timeout_s).await?;
    
    Ok(WorkflowPlannerOutput {
        objective: scope_result.get("objective").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        complexity: complexity_result.get("complexity").and_then(|v| v.as_str()).unwrap_or("DIRECT").to_string(),
        risk: complexity_result.get("risk").and_then(|v| v.as_str()).unwrap_or("LOW").to_string(),
        needs_evidence: scope_result.get("needs_evidence").and_then(|v| v.as_bool()).unwrap_or(false),
        scope: ScopePlan {
            objective: scope_result.get("objective").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            focus_paths: scope_result.get("scope").and_then(|v| v.get("focus_paths")).and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
            include_globs: scope_result.get("scope").and_then(|v| v.get("include_globs")).and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
            exclude_globs: scope_result.get("scope").and_then(|v| v.get("exclude_globs")).and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
            query_terms: scope_result.get("scope").and_then(|v| v.get("query_terms")).and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
            expected_artifacts: scope_result.get("scope").and_then(|v| v.get("expected_artifacts")).and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|v| v.as_str()).map(|s| s.to_string()).collect()).unwrap_or_default(),
            reason: scope_result.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        },
        preferred_formula: "reply_only".to_string(),
        alternatives: vec![],
        memory_id: String::new(),
        reason: reason_result.get("reason").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    })
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
    chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await
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
    let has_command_request = user_message.to_lowercase()
        .split_whitespace()
        .any(|w| ["run", "execute", "show", "display", "print"].contains(&w));

    // Check if any step actually executed a command
    let has_command_execution = step_results.iter()
        .any(|s| s.command.as_ref().is_some_and(|c| !c.is_empty()));

    // Check if step results have artifact_path (indicates output was captured to file)
    let has_artifact = step_results.iter()
        .any(|s| s.artifact_path.as_ref().is_some_and(|p| !p.is_empty()));

    // Deterministic override for command execution requests
    // This ensures RAW output is shown when user explicitly asks to run/see commands
    if has_command_request || has_command_execution {
        // Estimate output size from step results
        let output_is_short = step_results.iter()
            .filter_map(|s| s.raw_output.as_ref())
            .all(|out| out.lines().count() < 100);

        // Force RAW or RAW_PLUS_COMPACT for command execution
        let mode = if has_artifact {
            "RAW_PLUS_COMPACT".to_string()  // Has file artifact, show both
        } else if output_is_short {
            "RAW".to_string()  // Short output, show raw
        } else {
            "RAW_PLUS_COMPACT".to_string()  // Long output, show raw + compact summary
        };

        return Ok(EvidenceModeDecision {
            mode,
            reason: "Command execution detected - showing raw output".to_string(),
        });
    }

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
                    "has_command_request": has_command_request,
                    "has_command_execution": has_command_execution,
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
    chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await
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
    chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await
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
    chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await
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
    let resp = chat_once_with_timeout(client, chat_url, &req, cfg.timeout_s).await?;
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
    chat_json_with_repair_timeout(client, chat_url, &req, cfg.timeout_s).await
}
