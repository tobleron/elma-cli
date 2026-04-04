//! @efficiency-role: domain-logic
//!
//! Claim check and repair semantics verification.

use crate::*;
pub(crate) use verification_evidence::{
    has_downstream_dependents, has_verified_downstream_evidence,
    is_intermediate_shell_evidence_step,
};

fn mk_intel_req(cfg: &Profile, user_content: String) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
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
    }
}

async fn chat_and_parse(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    narrative: String,
) -> Result<ClaimCheckVerdict> {
    chat_json_with_repair(client, chat_url, &mk_intel_req(cfg, narrative)).await
}

pub(crate) async fn claim_check_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    evidence_mode: &EvidenceModeDecision,
    step_results: &[StepResult],
    draft: &str,
) -> Result<ClaimCheckVerdict> {
    let narrative = crate::intel_narrative::build_claim_check_narrative(
        user_message,
        &evidence_mode.mode,
        draft,
        step_results,
    );
    chat_and_parse(client, chat_url, cfg, narrative).await
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
    let narrative = crate::intel_narrative::build_repair_semantics_narrative(
        objective,
        purpose,
        original_cmd,
        repaired_cmd,
        &summarize_shell_output(failed_output),
    );
    chat_json_with_repair(client, chat_url, &mk_intel_req(cfg, narrative)).await
}

pub(crate) async fn check_execution_sufficiency_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    _user_message: &str,
    _route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
) -> Result<ExecutionSufficiencyVerdict> {
    let narrative = crate::intel_narrative::build_sufficiency_narrative(
        &program.objective,
        program,
        step_results,
    );
    chat_json_with_repair(client, chat_url, &mk_intel_req(cfg, narrative)).await
}

fn truncate_output(s: &String) -> &str {
    &s[..s.len().min(200)]
}

fn outcome_verifier_configs(cfg: &Profile) -> (Profile, Profile, Profile, Profile) {
    let base = &cfg.base_url;
    let model = &cfg.model;
    (
        default_text_generator_config(base, model),
        default_json_converter_config(base, model),
        default_verify_checker_config(base, model),
        default_json_repair_config(base, model),
    )
}

pub(crate) async fn verify_outcome_match_intent(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    outcome_verifier_cfg: &Profile,
    user_message: &str,
    _route_decision: &RouteDecision,
    _objective: &str,
    step: &Step,
    step_result: &StepResult,
) -> Result<OutcomeVerificationVerdict> {
    let reasoning = format!(
        "User request: {}\nStep purpose: {}\nStep result: exit_code={:?}, output={:?}",
        user_message,
        step.purpose(),
        step_result.exit_code,
        step_result.raw_output.as_ref().map(truncate_output)
    );

    let (text_gen_cfg, json_conv_cfg, verify_cfg, repair_cfg) =
        outcome_verifier_configs(outcome_verifier_cfg);

    let text = match generate_text_from_reasoning(client, chat_url, &text_gen_cfg, &reasoning).await
    {
        Ok(t) => t,
        Err(e) => {
            trace(args, &format!("text_generator_failed error={}", e));
            reasoning
        }
    };

    let schema_desc = r#"{"type":"object","required":["status","reason"],"properties":{"status":{"enum":["ok","retry"]},"reason":{"type":"string","minLength":1}}}"#;
    let json_str =
        match convert_text_to_json(client, chat_url, &json_conv_cfg, &text, schema_desc).await {
            Ok(j) => j,
            Err(e) => {
                trace(args, &format!("json_converter_failed error={}", e));
                return Ok(default_outcome_verdict(step_result.exit_code.unwrap_or(0)));
            }
        };

    let verify_result = match verify_json(client, chat_url, &verify_cfg, &json_str).await {
        Ok(r) => r,
        Err(e) => {
            trace(args, &format!("verify_checker_failed error={}", e));
            return parse_verdict_from_json(&json_str, step_result);
        }
    };

    let final_json = if verify_result.status == "problems" && !verify_result.problems.is_empty() {
        match repair_json(
            client,
            chat_url,
            &repair_cfg,
            &json_str,
            &verify_result.problems,
        )
        .await
        {
            Ok(repaired) => {
                trace(args, "json_repaired successfully");
                repaired
            }
            Err(e) => {
                trace(args, &format!("json_repair_failed error={}", e));
                json_str
            }
        }
    } else {
        trace(args, "json_verification_passed");
        json_str
    };
    parse_verdict_from_json(&final_json, step_result)
}

fn parse_verdict_from_json(
    json_str: &str,
    step_result: &StepResult,
) -> Result<OutcomeVerificationVerdict> {
    serde_json::from_str(json_str)
        .or_else(|_| Ok(default_outcome_verdict(step_result.exit_code.unwrap_or(0))))
}

fn mark_result_ok(result: &mut StepResult, id: &str, reason: &str, args: &Args) {
    result.outcome_status = Some("ok".to_string());
    result.outcome_reason = Some(reason.to_string());
    trace(
        args,
        &format!("outcome_verification id={} status=ok reason={}", id, reason),
    );
}

fn try_apply_downstream_validation(
    program: &Program,
    step_results: &mut [StepResult],
    idx: usize,
    args: &Args,
) -> bool {
    let id = step_results[idx].id.clone();
    if step_results[idx].kind == "edit"
        && has_verified_downstream_evidence(program, step_results, &id)
    {
        mark_result_ok(
            &mut step_results[idx],
            &id,
            "edit was validated by downstream grounded verification evidence",
            args,
        );
        return true;
    }
    false
}

fn try_skip_intermediate_evidence_step(
    program: &Program,
    result: &mut StepResult,
    args: &Args,
) -> bool {
    if is_intermediate_shell_evidence_step(program, result) {
        let id = result.id.clone();
        mark_result_ok(
            result,
            &id,
            "intermediate evidence step produced grounded output for downstream workflow steps",
            args,
        );
        return true;
    }
    false
}

fn handle_schema_error(
    args: &Args,
    result: &mut StepResult,
    verdict: &OutcomeVerificationVerdict,
    schema_err: &SchemaValidationError,
) -> bool {
    record_json_failure(args, "outcome_schema");
    if let Ok(json) = serde_json::to_value(verdict) {
        let errors = match schema_err {
            SchemaValidationError::ValidationErrors(errs) => errs.clone(),
            _ => vec![schema_err.to_string()],
        };
        if let Some(fixed) = deterministic_fix_outcome_verdict(args, verdict, &errors) {
            log_fallback_usage(
                args,
                "outcome_verifier",
                &schema_err.to_string(),
                "schema_deterministic_fix",
            );
            trace(args, &format!("outcome_schema_fixed id={}", result.id));
            result.outcome_status = Some(fixed.status.clone());
            result.outcome_reason = Some(fixed.reason.clone());
            if fixed.status.eq_ignore_ascii_case("ok") {
                result.ok = true;
            }
            return true;
        }
    }
    log_fallback_usage(
        args,
        "outcome_verifier",
        &schema_err.to_string(),
        "schema_validation_fallback",
    );
    trace(
        args,
        &format!(
            "outcome_schema_invalid id={} error={}",
            result.id, schema_err
        ),
    );
    false
}

fn handle_verify_error(args: &Args, result: &mut StepResult, error: &anyhow::Error) {
    record_json_failure(args, "outcome_verifier");
    let fallback = default_outcome_verdict(result.exit_code.unwrap_or(0));
    log_fallback_usage(
        args,
        "outcome_verifier",
        &error.to_string(),
        "exit_code_fallback",
    );
    result.outcome_status = Some(fallback.status.clone());
    result.outcome_reason = Some(fallback.reason.clone());
    trace(
        args,
        &format!(
            "outcome_verifier_fallback id={} exit_code={:?}",
            result.id, result.exit_code
        ),
    );
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
    let mut reasoning_clean = true;
    for idx in 0..step_results.len() {
        let (id, ok, kind) = {
            let r = &step_results[idx];
            (r.id.clone(), r.ok, r.kind.clone())
        };
        if !ok || !matches!(kind.as_str(), "shell" | "edit") {
            continue;
        }
        if try_apply_downstream_validation(program, step_results, idx, args) {
            continue;
        }
        if try_skip_intermediate_evidence_step(program, &mut step_results[idx], args) {
            continue;
        }

        let Some(step) = program
            .steps
            .iter()
            .find(|s| step_id(s) == step_results[idx].id)
        else {
            continue;
        };
        let result = &mut step_results[idx];

        match verify_outcome_match_intent(
            args,
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
                if let Err(schema_err) = validate_outcome_verdict(args, &verdict) {
                    if handle_schema_error(args, result, &verdict, &schema_err) {
                        return true;
                    }
                } else {
                    record_json_success(args);
                }
                apply_verdict_to_result(args, result, &verdict);
            }
            Err(error) => {
                handle_verify_error(args, result, &error);
                reasoning_clean = false;
            }
        }
        reasoning_clean &= ground_outcome_reason_if_needed(args, result);
    }
    reasoning_clean
}

fn apply_verdict_to_result(
    args: &Args,
    result: &mut StepResult,
    verdict: &OutcomeVerificationVerdict,
) {
    result.outcome_status = Some(verdict.status.clone());
    result.outcome_reason = Some(verdict.reason.clone());
    if verdict.status.eq_ignore_ascii_case("retry") {
        result.ok = false;
        let reason = verdict
            .reason
            .trim()
            .is_empty()
            .then(|| "step outcome did not match the intended result")
            .unwrap_or_else(|| verdict.reason.trim());
        result.summary = format!("outcome_mismatch: {reason}\n{}", result.summary);
        trace(
            args,
            &format!(
                "outcome_verification id={} status=retry reason={reason}",
                result.id
            ),
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

fn ground_outcome_reason_if_needed(args: &Args, result: &mut StepResult) -> bool {
    let Some(ref outcome_status) = result.outcome_status else {
        return true;
    };
    if !outcome_status.eq_ignore_ascii_case("retry") {
        return true;
    }
    let Some(ref outcome_reason) = result.outcome_reason else {
        return true;
    };

    match ground_critic_reason(args, outcome_reason, &[result.clone()]) {
        Ok(_) => {
            trace(args, &format!("outcome_reason_grounded id={}", result.id));
            true
        }
        Err(grounding_err) => {
            record_json_failure(args, "outcome_grounding");
            let grounded = default_outcome_verdict(result.exit_code.unwrap_or(0));
            log_fallback_usage(
                args,
                "outcome_verifier",
                &grounding_err.to_string(),
                "grounding_override",
            );
            result.outcome_status = Some(grounded.status.clone());
            result.outcome_reason = Some(grounded.reason.clone());
            if grounded.status.eq_ignore_ascii_case("ok") {
                result.ok = true;
            }
            trace(
                args,
                &format!(
                    "outcome_reason_hallucinated_overridden id={} exit_code={:?}",
                    result.id, result.exit_code
                ),
            );
            false
        }
    }
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
    let payload = serde_json::json!({
        "user_message": user_message, "route": route_decision.route,
        "complexity": complexity.complexity, "formula": formula.primary,
        "scope_objective": scope.objective, "program_objective": program.objective,
        "program_signature": program_signature(program),
        "step_results": step_results.iter().map(step_result_json).collect::<Vec<_>>(),
    });
    chat_json_with_repair(client, chat_url, &mk_intel_req(cfg, payload.to_string())).await
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
    let payload = serde_json::json!({
        "objective": objective, "purpose": purpose, "scope": scope,
        "complexity": complexity, "formula": formula, "cmd": cmd,
        "platform_os": platform_os, "platform_shell": platform_shell,
        "primary_bin": primary_bin, "command_exists": command_exists,
        "command_lookup": command_lookup,
    });
    chat_json_with_repair(client, chat_url, &mk_intel_req(cfg, payload.to_string())).await
}
