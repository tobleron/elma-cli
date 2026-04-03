//! @efficiency-role: domain-logic
//!
//! Claim check and repair semantics verification.

use crate::*;

fn has_downstream_dependents(program: &Program, step_id_value: &str) -> bool {
    program
        .steps
        .iter()
        .any(|step| step_depends_on(step).iter().any(|dep| dep == step_id_value))
}

fn is_intermediate_shell_evidence_step(program: &Program, result: &StepResult) -> bool {
    result.kind == "shell"
        && result.exit_code == Some(0)
        && result
            .raw_output
            .as_ref()
            .is_some_and(|text| !text.trim().is_empty())
        && has_downstream_dependents(program, &result.id)
}

fn has_verified_downstream_evidence(
    program: &Program,
    step_results: &[StepResult],
    result_id: &str,
) -> bool {
    let dependent_ids: Vec<String> = program
        .steps
        .iter()
        .filter(|step| step_depends_on(step).iter().any(|dep| dep == result_id))
        .map(|step| step_id(step).to_string())
        .collect();

    if dependent_ids.is_empty() {
        return false;
    }

    step_results.iter().any(|downstream| {
        dependent_ids.iter().any(|id| id == &downstream.id)
            && downstream.ok
            && matches!(downstream.kind.as_str(), "read" | "search" | "shell")
            && downstream
                .raw_output
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
    })
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

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: narrative,
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
    let narrative = crate::intel_narrative::build_repair_semantics_narrative(
        objective,
        purpose,
        original_cmd,
        repaired_cmd,
        &summarize_shell_output(failed_output),
    );

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: narrative,
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
    chat_json_with_repair(client, chat_url, &req).await
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

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: narrative,
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
    chat_json_with_repair(client, chat_url, &req).await
}

pub(crate) async fn verify_outcome_match_intent(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    outcome_verifier_cfg: &Profile, // Use this to get base_url and model
    user_message: &str,
    _route_decision: &RouteDecision,
    _objective: &str,
    step: &Step,
    step_result: &StepResult,
) -> Result<OutcomeVerificationVerdict> {
    // PHASE 3 JSON PIPELINE: Full validation flow
    // 1. Generate reasoning text (text_generator config)
    // 2. Convert to JSON (json_converter config)
    // 3. Verify JSON (verify_checker config)
    // 4. Repair if needed (json_repair config)
    // 5. Schema validate
    // 6. Deterministic fix if schema fails

    let reasoning = format!(
        "User request: {}\nStep purpose: {}\nStep result: exit_code={:?}, output={:?}",
        user_message,
        step.purpose(),
        step_result.exit_code,
        step_result
            .raw_output
            .as_ref()
            .map(|s| &s[..s.len().min(200)])
    );

    // Get pipeline configs from defaults using actual base_url and model
    let base_url = outcome_verifier_cfg.base_url.clone();
    let model = outcome_verifier_cfg.model.clone();

    let text_gen_cfg = default_text_generator_config(&base_url, &model);
    let json_conv_cfg = default_json_converter_config(&base_url, &model);
    let verify_cfg = default_verify_checker_config(&base_url, &model);
    let repair_cfg = default_json_repair_config(&base_url, &model);

    // Step 1: Generate text from reasoning
    let text = match generate_text_from_reasoning(client, chat_url, &text_gen_cfg, &reasoning).await
    {
        Ok(t) => t,
        Err(e) => {
            trace(args, &format!("text_generator_failed error={}", e));
            reasoning // Fallback to original reasoning
        }
    };

    // Step 2: Convert text to JSON
    let schema_desc = r#"{
  "type": "object",
  "required": ["status", "reason"],
  "properties": {
    "status": {"enum": ["ok", "retry"]},
    "reason": {"type": "string", "minLength": 1}
  }
}"#;

    let json_str =
        match convert_text_to_json(client, chat_url, &json_conv_cfg, &text, schema_desc).await {
            Ok(j) => j,
            Err(e) => {
                trace(args, &format!("json_converter_failed error={}", e));
                return Ok(default_outcome_verdict(step_result.exit_code.unwrap_or(0)));
            }
        };

    // Step 3: Verify JSON
    let verify_result = match verify_json(client, chat_url, &verify_cfg, &json_str).await {
        Ok(r) => r,
        Err(e) => {
            trace(args, &format!("verify_checker_failed error={}", e));
            // Try to parse JSON directly
            return parse_verdict_from_json(&json_str, step_result);
        }
    };

    // Step 4: Repair if problems found
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
                json_str // Use original
            }
        }
    } else {
        trace(args, "json_verification_passed");
        json_str
    };

    // Step 5 & 6: Parse and schema validate (existing flow)
    parse_verdict_from_json(&final_json, step_result)
}

/// Parse verdict from JSON string with schema validation
fn parse_verdict_from_json(
    json_str: &str,
    step_result: &StepResult,
) -> Result<OutcomeVerificationVerdict> {
    // Try to parse JSON
    let verdict: OutcomeVerificationVerdict = match serde_json::from_str(json_str) {
        Ok(v) => v,
        Err(e) => {
            // JSON parse failed, use fallback
            return Ok(default_outcome_verdict(step_result.exit_code.unwrap_or(0)));
        }
    };

    // Schema validation happens in the caller (verify_nontrivial_step_outcomes)
    Ok(verdict)
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

    for idx in 0..step_results.len() {
        let downstream_verified = {
            let result = &step_results[idx];
            result.kind == "edit"
                && has_verified_downstream_evidence(program, step_results, &result.id)
        };

        let result = &mut step_results[idx];
        if !result.ok {
            continue;
        }
        if !matches!(result.kind.as_str(), "shell" | "edit") {
            continue;
        }
        if is_intermediate_shell_evidence_step(program, result) {
            result.outcome_status = Some("ok".to_string());
            result.outcome_reason = Some(
                "intermediate evidence step produced grounded output for downstream workflow steps"
                    .to_string(),
            );
            trace(
                args,
                &format!(
                    "outcome_verification id={} status=ok reason=intermediate_evidence_step",
                    result.id
                ),
            );
            continue;
        }
        if downstream_verified {
            result.outcome_status = Some("ok".to_string());
            result.outcome_reason =
                Some("edit was validated by downstream grounded verification evidence".to_string());
            trace(
                args,
                &format!(
                    "outcome_verification id={} status=ok reason=downstream_edit_verification",
                    result.id
                ),
            );
            continue;
        }
        let Some(step) = program.steps.iter().find(|step| step_id(step) == result.id) else {
            continue;
        };

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
                // PHASE 3: Validate verdict against schema
                match validate_outcome_verdict(args, &verdict) {
                    Ok(_) => {
                        // Schema valid
                        record_json_success(args);
                    }
                    Err(schema_err) => {
                        // Schema invalid, try deterministic fix
                        record_json_failure(args, "outcome_schema");

                        if let Ok(json) = serde_json::to_value(&verdict) {
                            let error_messages: Vec<String> = match &schema_err {
                                SchemaValidationError::ValidationErrors(errs) => errs.clone(),
                                _ => vec![schema_err.to_string()],
                            };

                            if let Some(fixed) =
                                deterministic_fix_outcome_verdict(args, &verdict, &error_messages)
                            {
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
                                return true; // Exit early after fix (success)
                            }
                        }

                        // Fix failed, use exit code fallback
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
                    }
                }

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
            Err(error) => {
                record_json_failure(args, "outcome_verifier");
                reasoning_clean = false;
                let error_text = error.to_string();

                // FALLBACK: Use exit code as ground truth
                let fallback_verdict = default_outcome_verdict(result.exit_code.unwrap_or(0));
                log_fallback_usage(args, "outcome_verifier", &error_text, "exit_code_fallback");

                result.outcome_status = Some(fallback_verdict.status.clone());
                result.outcome_reason = Some(fallback_verdict.reason.clone());

                trace(
                    args,
                    &format!(
                        "outcome_verifier_fallback id={} exit_code={:?}",
                        result.id, result.exit_code
                    ),
                );
            }
        }

        // PHASE 2: Ground outcome verdict in actual output
        if let Some(ref outcome_status) = result.outcome_status {
            if outcome_status.eq_ignore_ascii_case("retry") {
                if let Some(ref outcome_reason) = result.outcome_reason {
                    match ground_critic_reason(args, outcome_reason, &[result.clone()]) {
                        Ok(_) => {
                            // Reason is grounded, keep it
                            trace(args, &format!("outcome_reason_grounded id={}", result.id));
                        }
                        Err(grounding_err) => {
                            // Hallucinated criticism - override with exit code verdict
                            record_json_failure(args, "outcome_grounding");
                            let grounded_verdict =
                                default_outcome_verdict(result.exit_code.unwrap_or(0));
                            log_fallback_usage(
                                args,
                                "outcome_verifier",
                                &grounding_err.to_string(),
                                "grounding_override",
                            );

                            result.outcome_status = Some(grounded_verdict.status.clone());
                            result.outcome_reason = Some(grounded_verdict.reason.clone());

                            if grounded_verdict.status.eq_ignore_ascii_case("ok") {
                                result.ok = true;
                            }

                            trace(
                                args,
                                &format!(
                                    "outcome_reason_hallucinated_overridden id={} exit_code={:?}",
                                    result.id, result.exit_code
                                ),
                            );
                        }
                    }
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
        grammar: None,
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
        grammar: None,
    };
    chat_json_with_repair(client, chat_url, &req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intermediate_shell_evidence_step_is_detected() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![
                Step::Shell {
                    id: "s1".to_string(),
                    cmd: "ls".to_string(),
                    common: StepCommon::default(),
                },
                Step::Select {
                    id: "sel1".to_string(),
                    instructions: "pick one".to_string(),
                    common: StepCommon {
                        depends_on: vec!["s1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let result = StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            ok: true,
            raw_output: Some("main.go\ncmd/root.go".to_string()),
            exit_code: Some(0),
            ..StepResult::default()
        };

        assert!(is_intermediate_shell_evidence_step(&program, &result));
    }

    #[test]
    fn standalone_shell_step_is_not_treated_as_intermediate_evidence() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![Step::Shell {
                id: "s1".to_string(),
                cmd: "ls".to_string(),
                common: StepCommon::default(),
            }],
        };

        let result = StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            ok: true,
            raw_output: Some("main.go".to_string()),
            exit_code: Some(0),
            ..StepResult::default()
        };

        assert!(!is_intermediate_shell_evidence_step(&program, &result));
    }

    #[test]
    fn edit_with_downstream_read_verification_is_detected() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![
                Step::Edit {
                    id: "e1".to_string(),
                    spec: EditSpec {
                        path: "README.md".to_string(),
                        operation: "append_text".to_string(),
                        content: "hello".to_string(),
                        ..EditSpec::default()
                    },
                    common: StepCommon::default(),
                },
                Step::Read {
                    id: "r1".to_string(),
                    path: "README.md".to_string(),
                    common: StepCommon {
                        depends_on: vec!["e1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let results = vec![
            StepResult {
                id: "e1".to_string(),
                kind: "edit".to_string(),
                ok: true,
                ..StepResult::default()
            },
            StepResult {
                id: "r1".to_string(),
                kind: "read".to_string(),
                ok: true,
                raw_output: Some("## Heading\nThis sandbox was exercised.".to_string()),
                exit_code: Some(0),
                ..StepResult::default()
            },
        ];

        assert!(has_verified_downstream_evidence(
            &program,
            &results,
            &results[0].id
        ));
    }

    #[test]
    fn edit_without_grounded_downstream_evidence_is_not_detected() {
        let program = Program {
            objective: "test".to_string(),
            steps: vec![
                Step::Edit {
                    id: "e1".to_string(),
                    spec: EditSpec {
                        path: "README.md".to_string(),
                        operation: "append_text".to_string(),
                        content: "hello".to_string(),
                        ..EditSpec::default()
                    },
                    common: StepCommon::default(),
                },
                Step::Read {
                    id: "r1".to_string(),
                    path: "README.md".to_string(),
                    common: StepCommon {
                        depends_on: vec!["e1".to_string()],
                        ..StepCommon::default()
                    },
                },
            ],
        };

        let results = vec![
            StepResult {
                id: "e1".to_string(),
                kind: "edit".to_string(),
                ok: true,
                ..StepResult::default()
            },
            StepResult {
                id: "r1".to_string(),
                kind: "read".to_string(),
                ok: true,
                raw_output: None,
                exit_code: Some(0),
                ..StepResult::default()
            },
        ];

        assert!(!has_verified_downstream_evidence(
            &program,
            &results,
            &results[0].id
        ));
    }
}
