//! @efficiency-role: domain-logic
//!
//! Claim check and repair semantics verification.

use crate::json_error_handler::schemas_verdicts::*;
use crate::*;
pub(crate) use verification_evidence::{
    has_downstream_dependents, has_verified_downstream_evidence,
    is_intermediate_shell_evidence_step,
};

fn mk_intel_req(cfg: &Profile, user_content: String) -> ChatCompletionRequest {
    chat_request_system_user(
        cfg,
        &cfg.system_prompt,
        &user_content,
        ChatRequestOptions::default(),
    )
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
    schema_err: &anyhow::Error,
) -> bool {
    record_json_failure(args, "outcome_schema");
    if let Ok(json) = serde_json::to_value(verdict) {
        let errors = if let Some(SchemaValidationError::ValidationErrors(errs)) =
            schema_err.downcast_ref::<SchemaValidationError>()
        {
            errs.clone()
        } else {
            vec![schema_err.to_string()]
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
    route: &str,
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
            route,
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
    route: &str,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    scope: &ScopePlan,
    program: &Program,
    step_results: &[StepResult],
) -> Result<MemoryGateVerdict> {
    let payload = serde_json::json!({
        "user_message": user_message, "route": route,
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
pub(crate) async fn verify_outcome_match_intent(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route: &str,
    objective: &str,
    step: &Step,
    result: &StepResult,
) -> Result<OutcomeVerificationVerdict> {
    let narrative = crate::intel_narrative::build_outcome_verification_narrative(
        user_message,
        route,
        objective,
        step,
        result,
    );
    chat_json_with_repair(client, chat_url, &mk_intel_req(cfg, narrative)).await
}

fn validate_outcome_verdict(_args: &Args, verdict: &OutcomeVerificationVerdict) -> Result<()> {
    if verdict.status.is_empty() {
        return Err(anyhow::anyhow!("Missing 'status' field in verdict"));
    }
    let s = verdict.status.to_lowercase();
    if s != "ok" && s != "retry" {
        return Err(anyhow::anyhow!("Invalid status: {}", verdict.status));
    }
    Ok(())
}

fn default_outcome_verdict(exit_code: i32) -> OutcomeVerificationVerdict {
    if exit_code == 0 {
        OutcomeVerificationVerdict {
            status: "ok".to_string(),
            reason: "default: exit_code 0".to_string(),
            answered_request: false,
            faithful_to_evidence: false,
            plain_text: false,
        }
    } else {
        OutcomeVerificationVerdict {
            status: "retry".to_string(),
            reason: format!("default: non-zero exit code {}", exit_code),
            answered_request: false,
            faithful_to_evidence: false,
            plain_text: false,
        }
    }
}
