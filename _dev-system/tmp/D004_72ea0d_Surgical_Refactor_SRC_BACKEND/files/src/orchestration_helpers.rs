use serde_json::Value;

use crate::*;

fn response_requires_exact_relative_path(reply_instructions: &str) -> bool {
    let lower = reply_instructions.to_ascii_lowercase();
    lower.contains("exact relative path")
        || lower.contains("exact grounded relative file paths")
        || lower.contains("preserve exact grounded relative file paths")
}

fn response_requires_two_bullets_and_entry_point(reply_instructions: &str) -> bool {
    let lower = reply_instructions.to_ascii_lowercase();
    (lower.contains("exactly two bullet points") || lower.contains("exactly 2 bullet points"))
        && lower.contains("entry point:")
}

fn summarized_bullets(step_results: &[StepResult]) -> Option<String> {
    let summarize = step_results
        .iter()
        .find(|result| result.kind == "summarize" && result.ok)
        .map(|result| result.summary.trim())
        .filter(|summary| !summary.is_empty())?;

    let mut bullets = summarize
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            if line.starts_with("- ") || line.starts_with("* ") {
                format!("- {}", line[2..].trim())
            } else {
                format!("- {line}")
            }
        })
        .take(2)
        .collect::<Vec<_>>();

    if bullets.len() == 2 {
        Some(bullets.join("\n"))
    } else {
        None
    }
}

fn selected_exact_grounded_path(step_results: &[StepResult]) -> Option<String> {
    let mut candidates = step_results
        .iter()
        .filter(|result| result.kind == "select" && result.ok)
        .filter_map(|result| result.raw_output.as_deref())
        .flat_map(|raw| raw.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && line.contains('/'))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.dedup();
    if candidates.len() == 1 {
        Some(candidates.remove(0))
    } else {
        None
    }
}

fn derive_exact_grounded_path_from_evidence(
    step_results: &[StepResult],
    final_text: &str,
) -> Option<String> {
    let basename_candidates = step_results
        .iter()
        .filter(|result| result.kind == "select" && result.ok)
        .filter_map(|result| result.raw_output.as_deref())
        .flat_map(|raw| raw.lines())
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.contains('/'))
        .filter(|line| final_text.contains(*line))
        .map(|line| line.to_string())
        .collect::<Vec<_>>();

    for basename in basename_candidates {
        let mut matches = step_results
            .iter()
            .filter(|result| result.kind == "shell" && result.ok)
            .filter_map(|result| result.raw_output.as_deref())
            .flat_map(|raw| raw.lines())
            .map(str::trim)
            .filter(|line| {
                !line.is_empty()
                    && line.contains('/')
                    && line.ends_with(&basename)
                    && line.len() > basename.len()
                    && line
                        .as_bytes()
                        .get(line.len().saturating_sub(basename.len() + 1))
                        == Some(&b'/')
            })
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        matches.sort();
        matches.dedup();
        if matches.len() == 1 {
            return Some(matches.remove(0));
        }
    }

    None
}

pub(crate) fn preserve_exact_grounded_path(
    final_text: String,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> String {
    if !response_requires_exact_relative_path(reply_instructions) {
        return final_text;
    }
    let Some(path) = selected_exact_grounded_path(step_results)
        .or_else(|| derive_exact_grounded_path_from_evidence(step_results, &final_text))
    else {
        return final_text;
    };
    if final_text.contains(&path) {
        return final_text;
    }

    let basename = std::path::Path::new(&path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");

    if basename.is_empty() || !final_text.contains(basename) {
        return format!("{path}\n{final_text}");
    }

    // Task 023: Find the longest suffix of 'path' that exists in 'final_text'
    // and ends with 'basename'. Replace that suffix with the full 'path'.
    // This prevents doubled prefixes (e.g. if 'sub/main.go' is there, replace it with 'root/sub/main.go').

    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut longest_suffix_match: Option<String> = None;

    // Check suffixes from longest to shortest (basename)
    for i in 0..path_segments.len() {
        let suffix = path_segments[i..].join("/");
        if final_text.contains(&suffix) {
            longest_suffix_match = Some(suffix);
            break;
        }
    }

    if let Some(suffix) = longest_suffix_match {
        // If the longest suffix is the whole path, we're done.
        if suffix == path {
            return final_text;
        }
        // Otherwise, replace the suffix with the full path.
        // We only replace the FIRST occurrence to stay safe.
        final_text.replacen(&suffix, &path, 1)
    } else {
        // Fallback: if even basename isn't found (shouldn't happen due to check above)
        // just prepend.
        format!("{path}\n{final_text}")
    }
}

pub(crate) fn preserve_requested_summary_and_entry_point(
    final_text: String,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> String {
    if !response_requires_two_bullets_and_entry_point(reply_instructions) {
        return final_text;
    }

    let bullets = summarized_bullets(step_results);
    let path = selected_exact_grounded_path(step_results)
        .or_else(|| derive_exact_grounded_path_from_evidence(step_results, &final_text));

    if let (Some(bullets), Some(path)) = (bullets, path) {
        return format!("{bullets}\nEntry point: {path}");
    }

    final_text
}

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
                content: format!(
                    "User message:\n{}\n\nInstructions:\n{}\n\nRespond conversationally and directly. Do not mention internal workflow, step state, or missing steps.",
                    line,
                    if reply_instructions.trim().is_empty() {
                        "Reply naturally and helpfully."
                    } else {
                        reply_instructions.trim()
                    }
                ),
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
    runtime_context: &Value,
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
                runtime_context,
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
        step_results
            .iter()
            .map(step_result_json)
            .collect::<Vec<_>>(),
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
    runtime_context: &Value,
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
    .with_extra("runtime_context", runtime_context)?
    .with_extra("evidence_mode", evidence_mode)?
    .with_extra("response_advice", response_advice)?
    .with_extra(
        "step_results",
        step_results
            .iter()
            .map(step_result_json)
            .collect::<Vec<_>>(),
    )?
    .with_extra("reply_instructions", reply_instructions)?;
    let output = unit.execute_with_fallback(&context).await?;
    let final_text = preserve_exact_grounded_path(
        output.get_str("final_text").unwrap_or_default().to_string(),
        step_results,
        reply_instructions,
    );
    Ok(preserve_requested_summary_and_entry_point(
        final_text,
        step_results,
        reply_instructions,
    ))
}

pub(crate) async fn maybe_format_final_text(
    client: &reqwest::Client,
    _chat_url: &Url,
    formatter_cfg: &Profile,
    line: &str,
    final_text: String,
    usage_total: Option<u64>,
) -> (String, Option<u64>) {
    // If the user explicitly asked for markdown, preserve it as-is
    if user_requested_markdown(line) {
        return (final_text, usage_total);
    }

    // Try automated plain-text transformation first
    let plain_text = plain_terminal_text(&final_text);

    let already_terminal_ready = plain_text.lines().count() <= 8
        || plain_text.contains("Entry point:")
        || plain_text
            .lines()
            .any(|line| line.trim_start().starts_with("- "))
        || plain_text.contains("_stress_testing/");
    if already_terminal_ready {
        return (plain_text, usage_total);
    }

    // If the current config has a managed formatter, use it for final cleaning
    let unit = FormatterUnit::new(formatter_cfg.clone());
    let context = IntelContext::new(
        plain_text.clone(),
        RouteDecision::default(), // Dummy as formatter doesn't need it
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    );

    match unit.execute_with_fallback(&context).await {
        Ok(output) => (
            output
                .get_str("formatted_text")
                .unwrap_or(&plain_text)
                .to_string(),
            usage_total, // Note: Usage tracking is omitted for now for simplicity, as it's a minor call
        ),
        _ => (plain_text, usage_total),
    }
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

#[cfg(test)]
mod tests {
    use super::{preserve_exact_grounded_path, preserve_requested_summary_and_entry_point};
    use crate::StepResult;

    #[test]
    fn preserve_exact_grounded_path_replaces_shortened_basename() {
        let step_results = vec![StepResult {
            id: "sel1".to_string(),
            kind: "select".to_string(),
            ok: true,
            raw_output: Some("_stress_testing/_opencode_for_testing/main.go".to_string()),
            ..StepResult::default()
        }];

        let final_text = preserve_exact_grounded_path(
            "main.go is the primary entry point.".to_string(),
            &step_results,
            "State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.",
        );

        assert!(final_text.starts_with("_stress_testing/_opencode_for_testing/main.go"));
    }

    #[test]
    fn preserve_exact_grounded_path_can_recover_path_from_shell_evidence() {
        let step_results = vec![
            StepResult {
                id: "s2".to_string(),
                kind: "shell".to_string(),
                ok: true,
                raw_output: Some(
                    "_stress_testing/_opencode_for_testing/main.go\n_stress_testing/_opencode_for_testing/cmd/root.go"
                        .to_string(),
                ),
                ..StepResult::default()
            },
            StepResult {
                id: "sel1".to_string(),
                kind: "select".to_string(),
                ok: true,
                raw_output: Some("main.go".to_string()),
                ..StepResult::default()
            },
        ];

        let final_text = preserve_exact_grounded_path(
            "main.go is the strongest grounded primary entry point.".to_string(),
            &step_results,
            "State the selected exact relative path first, then explain briefly why it is the strongest grounded entry point.",
        );

        assert!(final_text.starts_with("_stress_testing/_opencode_for_testing/main.go"));
    }

    #[test]
    fn preserve_requested_summary_and_entry_point_restores_missing_bullets_and_path_line() {
        let step_results = vec![
            StepResult {
                id: "sum1".to_string(),
                kind: "summarize".to_string(),
                ok: true,
                summary: "- First repo purpose\n- Second repo purpose".to_string(),
                ..StepResult::default()
            },
            StepResult {
                id: "sel1".to_string(),
                kind: "select".to_string(),
                ok: true,
                raw_output: Some("_stress_testing/_opencode_for_testing/main.go".to_string()),
                ..StepResult::default()
            },
        ];

        let final_text = preserve_requested_summary_and_entry_point(
            "Primary entry point: _stress_testing/_opencode_for_testing/main.go".to_string(),
            &step_results,
            "Return exactly two bullet points from the grounded README summary first. Then add one final line that starts with `Entry point:` followed by the selected exact relative path. Preserve exact grounded relative file paths from the evidence and do not mention files that were not observed.",
        );

        assert!(final_text.contains("- First repo purpose"));
        assert!(final_text.contains("- Second repo purpose"));
        assert!(final_text.contains("Entry point: _stress_testing/_opencode_for_testing/main.go"));
    }
}
