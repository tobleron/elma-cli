use crate::*;

pub(crate) async fn claim_check_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    evidence_mode: &EvidenceModeDecision,
    step_results: &[StepResult],
    draft: &str,
) -> Result<ClaimCheckVerdict> {
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
                    "evidence_mode": evidence_mode,
                    "draft": draft,
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

pub(crate) async fn guard_repair_semantics_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    original_cmd: &str,
    repaired_cmd: &str,
    failed_output: &str,
) -> Result<RepairSemanticsVerdict> {
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
                    "original_cmd": original_cmd,
                    "repaired_cmd": repaired_cmd,
                    "failed_output": summarize_shell_output(failed_output),
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

pub(crate) async fn check_execution_sufficiency_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
) -> Result<ExecutionSufficiencyVerdict> {
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
                    "objective": program.objective,
                    "program_steps": program.steps.iter().map(program_step_json).collect::<Vec<_>>(),
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

pub(crate) async fn verify_outcome_match_intent(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    objective: &str,
    step: &Step,
    step_result: &StepResult,
) -> Result<OutcomeVerificationVerdict> {
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
                    "objective": objective,
                    "step": program_step_json(step),
                    "step_result": step_result_json(step_result),
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

pub(crate) async fn verify_nontrivial_step_outcomes(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &mut [StepResult],
) -> bool {
    let strict = !route_decision.route.eq_ignore_ascii_case("CHAT");
    let mut reasoning_clean = true;

    for result in step_results.iter_mut() {
        if !result.ok {
            continue;
        }
        if !matches!(result.kind.as_str(), "shell" | "edit") {
            continue;
        }
        let Some(step) = program.steps.iter().find(|step| step_id(step) == result.id) else {
            continue;
        };

        match verify_outcome_match_intent(
            client,
            chat_url,
            cfg,
            user_message,
            route_decision,
            &program.objective,
            step,
            result,
        )
        .await
        {
            Ok(verdict) => {
                result.outcome_status = Some(verdict.status.clone());
                result.outcome_reason = Some(verdict.reason.clone());
                if verdict.status.eq_ignore_ascii_case("retry") {
                    result.ok = false;
                    let reason = if verdict.reason.trim().is_empty() {
                        "step outcome did not match the intended result"
                    } else {
                        verdict.reason.trim()
                    };
                    result.summary = format!("outcome_mismatch: {reason}\n{}", result.summary);
                    trace(
                        args,
                        &format!("outcome_verification id={} status=retry reason={reason}", result.id),
                    );
                } else {
                    trace(
                        args,
                        &format!(
                            "outcome_verification id={} status=ok reason={}",
                            result.id,
                            verdict.reason.trim()
                        ),
                    );
                }
            }
            Err(error) => {
                reasoning_clean = false;
                let error_text = error.to_string();
                result.outcome_status = Some("parse_error".to_string());
                result.outcome_reason = Some(error_text.clone());
                trace(
                    args,
                    &format!("outcome_verifier_parse_error id={} error={error_text}", result.id),
                );
                if strict {
                    result.ok = false;
                    result.summary = format!(
                        "outcome_verifier_parse_error: {error_text}\n{}",
                        result.summary
                    );
                }
            }
        }
    }

    reasoning_clean
}

pub(crate) async fn gate_formula_memory_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    scope: &ScopePlan,
    program: &Program,
    step_results: &[StepResult],
) -> Result<MemoryGateVerdict> {
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
                    "complexity": complexity.complexity,
                    "formula": formula.primary,
                    "scope_objective": scope.objective,
                    "program_objective": program.objective,
                    "program_signature": program_signature(program),
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

pub(crate) async fn preflight_command_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    cmd: &str,
    platform_os: &str,
    platform_shell: &str,
    primary_bin: &str,
    command_exists: bool,
    command_lookup: &str,
) -> Result<CommandPreflightVerdict> {
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
                    "complexity": complexity,
                    "formula": formula,
                    "cmd": cmd,
                    "platform_os": platform_os,
                    "platform_shell": platform_shell,
                    "primary_bin": primary_bin,
                    "command_exists": command_exists,
                    "command_lookup": command_lookup,
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
