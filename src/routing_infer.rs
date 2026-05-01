//! @efficiency-role: service-orchestrator
//!
//! Routing - Inference Functions

use crate::routing_config::RoutingConfig;
use crate::*;

/// Helper function to check if a ProbabilityDecision confidently matches a specific choice
fn is_choice_confident(
    decision: &ProbabilityDecision,
    choice: &str,
    min_margin: f64,
    max_entropy: f64,
) -> bool {
    decision.choice.eq_ignore_ascii_case(choice)
        && decision.margin >= min_margin
        && decision.entropy <= max_entropy
}

/// Helper function to check if a ProbabilityDecision confidently matches a specific choice using config
fn is_choice_confident_with_config(
    decision: &ProbabilityDecision,
    choice: &str,
    config: &RoutingConfig,
) -> bool {
    match choice {
        "CHAT" => {
            decision.choice.eq_ignore_ascii_case("CHAT")
                && decision.margin >= config.speech_chat_margin_threshold
                && decision.entropy <= config.speech_chat_entropy_threshold
        }
        "WORKFLOW" => {
            decision.choice.eq_ignore_ascii_case("WORKFLOW")
                && decision.margin >= config.workflow_confident_margin_threshold
                && decision.entropy <= config.workflow_confident_entropy_threshold
        }
        "INSTRUCT" => {
            decision.choice.eq_ignore_ascii_case("INSTRUCT")
                && decision.margin >= config.speech_confident_instruct_margin_threshold
                && decision.entropy <= config.speech_confident_instruct_entropy_threshold
        }
        "DECIDE" => {
            decision.choice.eq_ignore_ascii_case("DECIDE")
            && decision.margin >= 0.0  // No specific threshold for DECIDE in original code
            && decision.entropy <= 1.0
        }
        _ => false,
    }
}

fn should_short_circuit_chat_route(
    speech_act: &ProbabilityDecision,
    workflow: &ProbabilityDecision,
    routing_config: &RoutingConfig,
) -> bool {
    // Use confidence-based assessment with config instead of hardcoded thresholds
    let is_chat_speech = is_choice_confident_with_config(speech_act, "CHAT", routing_config);
    let is_chat_workflow = is_choice_confident_with_config(workflow, "CHAT", routing_config);

    is_chat_speech
        && speech_act.entropy <= routing_config.speech_chat_entropy_threshold
        && speech_act.margin >= routing_config.speech_chat_margin_threshold
        && is_chat_workflow
        && workflow.entropy <= routing_config.workflow_chat_entropy_threshold
        && workflow.margin >= routing_config.workflow_chat_margin_threshold
}

fn should_apply_speech_chat_boost(workflow: &ProbabilityDecision) -> bool {
    // Use confidence-based assessment instead of hardcoded string comparisons
    is_choice_confident(workflow, "CHAT", 0.0, 1.0)  // Any confidence level for CHAT
        || workflow.margin < 0.15
        || workflow.entropy > 0.50
}

fn top_non_chat_route(distribution: &[(String, f64)]) -> Option<String> {
    distribution
        .iter()
        .filter(|(label, _)| label != "CHAT")
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(label, _)| label.clone())
}

fn fallback_probability_decision(
    choice: &str,
    pairs: &'static [(&'static str, &'static str)],
    source: &str,
) -> ProbabilityDecision {
    let mut distribution = Vec::with_capacity(pairs.len());
    for (_, label) in pairs {
        distribution.push((label.to_string(), if *label == choice { 1.0 } else { 0.0 }));
    }
    ProbabilityDecision {
        choice: choice.to_string(),
        source: source.to_string(),
        distribution,
        margin: 1.0,
        entropy: 1.0,
    }
}

fn conservative_classifier_fallback(pairs: &'static [(&'static str, &'static str)]) -> String {
    for preferred in ["WORKFLOW", "INSTRUCT", "INSPECT"] {
        if pairs.iter().any(|(_, label)| *label == preferred) {
            return preferred.to_string();
        }
    }
    pairs
        .first()
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| "WORKFLOW".to_string())
}

fn dsl_value_to_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

fn normalize_classifier_label(
    choice_or_label: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<&'static str> {
    let token = choice_or_label.trim();
    if token.is_empty() {
        return None;
    }
    pairs
        .iter()
        .find(|(code, label)| token == *code || token.eq_ignore_ascii_case(*label))
        .map(|(_, label)| *label)
}

pub(crate) async fn infer_digit_router(
    client: &reqwest::Client,
    chat_url: &Url,
    router_cfg: &Profile,
    router_cal: &RouterCalibration,
    prompt: String,
    pairs: &'static [(&'static str, &'static str)],
) -> Result<ProbabilityDecision> {
    let reasoning_format = classifier_reasoning_format(router_cfg);
    let max_tokens = if reasoning_format.eq_ignore_ascii_case("auto") {
        router_cfg.max_tokens.max(512)
    } else {
        router_cfg.max_tokens.min(256)
    };
    let mut req = chat_request_system_user(
        router_cfg,
        &router_cfg.system_prompt,
        &prompt,
        ChatRequestOptions {
            temperature: Some(0.0),
            top_p: Some(1.0),
            max_tokens: Some(max_tokens),
            repeat_penalty: Some(Some(runtime_llm_config().default_repeat_penalty)),
            reasoning_format: Some(Some(reasoning_format)),
            ..ChatRequestOptions::default()
        },
    );
    apply_profile_grammar(router_cfg, &mut req)?;
    let fallback_choice = conservative_classifier_fallback(pairs);

    // Router classifiers return a single DSL line:
    //   ROUTE choice=1 label=CHAT reason="..." entropy=0.1
    //   MODE choice=... label=... reason="..." entropy=...
    //   ACT choice=... label=... reason="..." entropy=...
    //
    // Parse strictly with the shared DSL parser (no JSON, no Markdown fences).
    let expected_template = classifier_expected_template(pairs);
    let mut raw =
        request_classifier_raw(client, chat_url, &req, router_cfg.timeout_s.min(45)).await?;
    let (chosen, entropy_val, parse_source) = if let Some(parsed) =
        parse_classifier_candidates(&raw, pairs)
    {
        parsed
    } else {
        append_trace_log_line(&format!(
            "[CLASSIFIER_DSL_REPAIR] unit={} error=INVALID_DSL",
            router_cfg.name
        ));
        req.messages.push(ChatMessage::simple(
            "user",
            &classifier_repair_observation(&raw, &expected_template),
        ));
        req.max_tokens = req.max_tokens.saturating_mul(2).min(512);
        raw = request_classifier_raw(client, chat_url, &req, router_cfg.timeout_s.min(45)).await?;
        parse_classifier_candidates(&raw, pairs)
            .unwrap_or_else(|| (fallback_choice.clone(), 0.1, "dsl_parse_failed"))
    };

    // Build distribution from classifier output (not logprobs)
    let other_count = (pairs.len() as f64 - 1.0).max(1.0);
    let mut distribution: Vec<(String, f64)> = pairs
        .iter()
        .map(|(_, label)| {
            let p = if *label == chosen {
                1.0 - entropy_val
            } else {
                // Distribute entropy among other choices
                entropy_val / other_count
            };
            ((*label).to_string(), p)
        })
        .collect();

    distribution.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let source = parse_source;
    let raw_entropy = entropy_val;
    let distribution = inject_router_noise(&distribution, raw_entropy);

    let route = distribution
        .first()
        .map(|(label, _)| label.clone())
        .unwrap_or(chosen);

    Ok(ProbabilityDecision {
        choice: route,
        source: source.to_string(),
        margin: route_margin(&distribution),
        entropy: raw_entropy,
        distribution,
    })
}

fn classifier_reasoning_format(profile: &Profile) -> String {
    if let Some(behavior) = crate::ui_state::current_model_behavior_profile() {
        if behavior
            .preferred_reasoning_format
            .eq_ignore_ascii_case("auto")
            && !behavior.none_final_clean
        {
            return "auto".to_string();
        }
    }
    profile.reasoning_format.clone()
}

async fn request_classifier_raw(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<String> {
    let resp = chat_once_with_timeout(client, chat_url, req, timeout_s).await?;
    Ok(resp
        .choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default())
}

fn classifier_expected_template(pairs: &[(&str, &str)]) -> String {
    let labels: Vec<&str> = pairs.iter().map(|(_, l)| *l).collect();
    let command = if pairs.len() == 2 {
        "ROUTE"
    } else if pairs.len() == 3 {
        "ACT"
    } else if pairs.len() == 5 {
        "MODE"
    } else {
        ""
    };
    let choices = (1..=pairs.len())
        .map(|i| i.to_string())
        .collect::<Vec<_>>()
        .join("|");
    format!(
        "{command} choice={choices} label={labels} reason=\"justification\" entropy=N.N",
        choices = choices,
        labels = labels.join("|")
    )
}

fn classifier_repair_observation(raw: &str, template: &str) -> String {
    let preview = crate::text_utils::strip_thinking_blocks(raw);
    let preview = if preview.trim().is_empty() {
        "model returned prose/thinking or empty output instead of one classifier DSL line"
            .to_string()
    } else {
        preview.chars().take(180).collect::<String>()
    };
    format!(
        "INVALID_FORMAT\nExpected: {template}\nGot: {preview}\nReturn one DSL line matching the Expected format."
    )
}

fn parse_classifier_candidates(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<(String, f64, &'static str)> {
    for candidate in crate::text_utils::structured_output_candidates(raw) {
        let trimmed = candidate.trim();

        // Path 1: Full DSL with command token (e.g., "MODE choice=2 label=EXECUTE...")
        if let Ok((_cmd, fields)) = parse_intel_dsl_to_value(trimmed) {
            let chosen = extract_label_from_dsl_fields(&fields, pairs);
            if let Some(chosen) = chosen {
                let entropy_val = fields
                    .get("entropy")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.1)
                    .clamp(0.0, 1.0);
                return Some((chosen, entropy_val, "dsl_output"));
            }
        }

        // Path 2: Field-only DSL without command token (e.g., "choice=2 label=EXECUTE...")
        // Small models frequently omit the uppercase command prefix
        if let Some(chosen) = parse_field_only_dsl(trimmed, pairs) {
            return Some((chosen.0, chosen.1, "dsl_field_only"));
        }

        // Path 3: JSON extraction (legacy provider output)
        if let Some(chosen) = extract_from_json_output(trimmed, pairs) {
            return Some((chosen.0, chosen.1, "dsl_json_fallback"));
        }
    }
    None
}

/// Extract label from a parsed DSL fields map
fn extract_label_from_dsl_fields(
    fields: &serde_json::Value,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<String> {
    let choice_label = fields
        .get("choice")
        .and_then(dsl_value_to_string)
        .and_then(|s| normalize_classifier_label(&s, pairs).map(str::to_string));
    let label = fields
        .get("label")
        .and_then(|v| v.as_str())
        .and_then(|s| normalize_classifier_label(s, pairs).map(str::to_string));
    label.or(choice_label)
}

/// Parse field-only DSL lines that lack the uppercase command prefix.
/// Example: "choice=2 label=EXECUTE reason=\"list directory\" entropy=0.1"
fn parse_field_only_dsl(
    text: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<(String, f64)> {
    let line = text.lines().next()?.trim();
    // Must contain at least one known field pattern
    if !line.starts_with("choice=") && !line.starts_with("label=") {
        return None;
    }
    // The DSL parser requires a command token prefix. Use a synthetic
    // uppercase token and re-parse the line as a full DSL command.
    let fake_dsl = format!("FIELDS {}", line);
    let (_cmd, fields) = parse_intel_dsl_to_value(&fake_dsl).ok()?;
    let chosen = extract_label_from_dsl_fields(&fields, pairs)?;
    let entropy_val = fields
        .get("entropy")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1)
        .clamp(0.0, 1.0);
    Some((chosen, entropy_val))
}

/// Extract classifier result from JSON output (legacy provider format).
/// Example: {"choice": "2", "label": "EXECUTE", "reason": "...", "entropy": 0.1}
fn extract_from_json_output(
    text: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<(String, f64)> {
    use crate::routing_parse::extract_first_json_object;
    let json_str = extract_first_json_object(text)?;
    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
    let chosen = extract_label_from_dsl_fields(&parsed, pairs)?;
    let entropy_val = parsed
        .get("entropy")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.1)
        .clamp(0.0, 1.0);
    Some((chosen, entropy_val))
}

pub(crate) async fn infer_route_prior(
    client: &reqwest::Client,
    chat_url: &Url,
    speech_act_cfg: &Profile,
    workflow_router_cfg: &Profile,
    mode_router_cfg: &Profile,
    router_cal: &RouterCalibration,
    user_message: &str,
    workspace_facts: &str,
    workspace_brief: &str,
    recent_messages: &[ChatMessage],
) -> Result<RouteDecision> {
    let conversation = recent_messages
        .iter()
        .skip(1)
        .rev()
        .take(12)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content.replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n");

    let speech_prompt = format!(
        r#"User message:
{user_message}

Workspace facts:
{facts}

Workspace brief:
{brief}

Conversation so far (most recent last):
{conversation}"#,
        user_message = user_message,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation,
    );
    let speech_act = infer_digit_router(
        client,
        chat_url,
        speech_act_cfg,
        router_cal,
        speech_prompt,
        speech_act_code_pairs(),
    )
    .await
    .unwrap_or_else(|_| {
        fallback_probability_decision("INQUIRE", speech_act_code_pairs(), "fallback")
    });

    let workflow_prompt = format!(
        "User message:\n{user_message}\n\nWorkspace facts:\n{}\n\nWorkspace brief:\n{}\n\nConversation so far (most recent last):\n{}",
        workspace_facts.trim(),
        workspace_brief.trim(),
        conversation
    );
    let workflow = infer_digit_router(
        client,
        chat_url,
        workflow_router_cfg,
        router_cal,
        workflow_prompt,
        workflow_code_pairs(),
    )
    .await
    .unwrap_or_else(|_| {
        let choice = if is_choice_confident(&speech_act, "CHAT", 0.0, 1.0) {
            "CHAT"
        } else {
            "WORKFLOW"
        };
        fallback_probability_decision(choice, workflow_code_pairs(), "fallback")
    });

    // Create a default routing config for now - in practice this would come from application state
    let routing_config = RoutingConfig::default();
    if should_short_circuit_chat_route(&speech_act, &workflow, &routing_config) {
        let workflow = ProbabilityDecision {
            choice: "CHAT".to_string(),
            source: "speech_short_circuit".to_string(),
            distribution: vec![("CHAT".to_string(), 1.0), ("WORKFLOW".to_string(), 0.0)],
            margin: 1.0,
            entropy: 0.0,
        };
        let mode = ProbabilityDecision {
            choice: "DECIDE".to_string(),
            source: "speech_short_circuit".to_string(),
            distribution: vec![
                ("DECIDE".to_string(), 1.0),
                ("INSPECT".to_string(), 0.0),
                ("EXECUTE".to_string(), 0.0),
                ("PLAN".to_string(), 0.0),
                ("MASTERPLAN".to_string(), 0.0),
            ],
            margin: 1.0,
            entropy: 0.0,
        };
        return Ok(RouteDecision {
            route: "CHAT".to_string(),
            source: "speech_short_circuit".to_string(),
            distribution: vec![
                ("CHAT".to_string(), 1.0 - speech_act.entropy),
                ("SHELL".to_string(), 0.0),
                ("PLAN".to_string(), 0.0),
                ("MASTERPLAN".to_string(), 0.0),
                ("DECIDE".to_string(), 0.0),
            ],
            margin: 1.0,
            entropy: speech_act.entropy,
            speech_act,
            workflow,
            mode,
            evidence_required: false,
        });
    }

    // PRINCIPLE: Bypass mode classifier when routing is already clear.
    // The mode classifier is the weakest link with small models. When the
    // first two stages agree on WORKFLOW with high confidence, skip the
    // mode classifier and route directly to EXECUTE as the safe default.
    let speech_confident_instruct =
        is_choice_confident_with_config(&speech_act, "INSTRUCT", &routing_config);
    let workflow_confident =
        is_choice_confident_with_config(&workflow, "WORKFLOW", &routing_config);
    let bypass_mode = speech_confident_instruct && workflow_confident;

    let mode = if bypass_mode {
        fallback_probability_decision("EXECUTE", mode_code_pairs(), "direct_2stage")
    } else {
        let mode_prompt = format!(
            "User message:\n{user_message}\n\nWorkflow prior:\n- choice: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nWorkspace facts:\n{}\n\nWorkspace brief:\n{}\n\nConversation so far (most recent last):\n{}",
            workflow.choice,
            format_route_distribution(&workflow.distribution),
            workflow.margin,
            workflow.entropy,
            workspace_facts.trim(),
            workspace_brief.trim(),
            conversation
        );
        // infer_digit_router always returns Ok() — DSL parse failures are
        // surfaced in the source field (e.g., "dsl_parse_failed"), not as Err.
        let mode = infer_digit_router(
            client,
            chat_url,
            mode_router_cfg,
            router_cal,
            mode_prompt,
            mode_code_pairs(),
        )
        .await
        .unwrap_or_else(|_| {
            let choice = if is_choice_confident(&workflow, "CHAT", 0.0, 1.0) {
                "DECIDE"
            } else {
                "EXECUTE"
            };
            fallback_probability_decision(choice, mode_code_pairs(), "mode_http_failed")
        });

        // PRINCIPLE: When mode classifier parse failed AND workflow is WORKFLOW,
        // preserve WORKFLOW route by overriding mode to EXECUTE instead of
        // letting the degraded choice propagate. A mode classifier weakness
        // should not erase correct routing from earlier stages.
        let mode_dsl_failed = mode.source.contains("dsl_parse_failed");
        let workflow_is_workflow = is_choice_confident(&workflow, "WORKFLOW", 0.0, 1.0);
        if mode_dsl_failed && workflow_is_workflow {
            append_trace_log_line(&format!(
                "[MODE_DSL_FAILED_PRESERVED_WORKFLOW] overriding mode source={} to EXECUTE",
                mode.source
            ));
            fallback_probability_decision(
                "EXECUTE",
                mode_code_pairs(),
                "mode_dsl_failed_preserved_workflow",
            )
        } else {
            mode
        }
    };

    let speech_chat_p = probability_of(&speech_act.distribution, "CHAT");
    let instruct_p = probability_of(&speech_act.distribution, "INSTRUCT");

    let mut workflow_chat_p = probability_of(&workflow.distribution, "CHAT");
    let mut workflow_workflow_p = probability_of(&workflow.distribution, "WORKFLOW");

    // When speech act signals INSTRUCT with meaningful probability,
    // discount the workflow gate's CHAT confidence. The speech act is
    // the primary intent signal for distinguishing commands from chat.
    // Without this, a small model that splits evenly between CHAT and
    // INSTRUCT (e.g. 40/40) will have the workflow gate's CHAT
    // classification dominate and block shell commands from executing.
    let speech_instruct_meaningful = instruct_p >= 0.30;
    let instruct_over_chat = instruct_p >= speech_chat_p * 0.70;
    let split_uncertainty =
        speech_chat_p >= 0.30 && instruct_p >= 0.30 && (speech_chat_p + instruct_p) > 0.60;
    let should_discount_workflow_chat =
        speech_instruct_meaningful && (instruct_over_chat || split_uncertainty);

    if should_discount_workflow_chat {
        // Transfer mass from workflow CHAT → WORKFLOW proportional to
        // how strongly INSTRUCT dominates over CHAT in the speech act.
        let instruct_advantage = instruct_p / (instruct_p + speech_chat_p + 1e-10);
        let shift = workflow_chat_p * instruct_advantage * 0.75;
        if shift > 0.0 {
            workflow_chat_p -= shift;
            workflow_workflow_p += shift;
        }
    }

    let chat_p = workflow_chat_p;
    let workflow_p = workflow_workflow_p;
    let shell_p = workflow_p
        * (probability_of(&mode.distribution, "INSPECT")
            + probability_of(&mode.distribution, "EXECUTE"));
    let plan_p = workflow_p * probability_of(&mode.distribution, "PLAN");
    let masterplan_p = workflow_p * probability_of(&mode.distribution, "MASTERPLAN");
    let decide_p = workflow_p * probability_of(&mode.distribution, "DECIDE");
    let mut distribution = vec![
        ("CHAT".to_string(), chat_p),
        ("SHELL".to_string(), shell_p),
        ("PLAN".to_string(), plan_p),
        ("MASTERPLAN".to_string(), masterplan_p),
        ("DECIDE".to_string(), decide_p),
    ];

    // Map speech act labels to route adjustments
    // "CHAT" → boost CHAT route (user wants conversation)
    // "INQUIRE" → neutral (user wants information)
    // "INSTRUCT" → boost non-CHAT routes (user wants action)

    // If "CHAT" is high, boost CHAT route
    if speech_chat_p > 0.5 && should_apply_speech_chat_boost(&workflow) {
        for (label, p) in &mut distribution {
            if label == "CHAT" {
                *p = speech_chat_p + (1.0 - speech_chat_p) * *p;
            } else {
                *p *= 1.0 - speech_chat_p;
            }
        }
    }

    // If "INSTRUCT" is present, boost non-CHAT routes (user wants action).
    // Lower threshold (0.3 instead of 0.5) because small models rarely
    // reach 0.5 confidence on INSTRUCT for actionable inputs.
    if instruct_p > 0.3 {
        let current_chat_p = probability_of(&distribution, "CHAT");
        let workflow_boost = (instruct_p - 0.3) * 0.7;
        let non_chat_total: f64 = distribution
            .iter()
            .filter(|(l, _)| l != "CHAT")
            .map(|(_, p)| *p)
            .sum();

        for (label, p) in &mut distribution {
            if label != "CHAT" && non_chat_total > 0.0 {
                *p += workflow_boost * (*p / non_chat_total);
            } else if label == "CHAT" {
                *p *= 1.0 - workflow_boost;
            }
        }
    }

    distribution.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let margin = route_margin(&distribution);
    let entropy = route_entropy(&distribution);
    let path_scoped_request = extract_first_path_from_user_text(user_message).is_some();
    let workflow_confident = is_choice_confident(&workflow, "WORKFLOW", 0.30, 0.60);
    let speech_confident_chat = is_choice_confident(&speech_act, "CHAT", 0.50, 0.40);
    let speech_confident_instruct = is_choice_confident(&speech_act, "INSTRUCT", 0.30, 0.60);

    // PRINCIPLE: Under-execute when uncertain.
    // If entropy is high (> 0.6) or margin is low (< 0.2), fallback to CHAT if speech act is CHAT.
    // Mode classifier was bypassed or preserved — do not let uncertainty in the
    // combined distribution erase correct early-stage routing.
    let mode_is_bypass_or_preserved = mode.source.starts_with("direct_2stage")
        || mode
            .source
            .starts_with("mode_dsl_failed_preserved_workflow")
        || mode.source.starts_with("mode_failed_salvaged");
    let fallback_to_chat = !mode_is_bypass_or_preserved
        && ((is_choice_confident(&speech_act, "CHAT", 0.0, 1.0)
            && (margin < 0.20 || entropy > 0.60))
            || (margin < 0.12)); // Hard fallback for any case where we are absolutely guessing
    let preserve_workflow_route = mode_is_bypass_or_preserved
        || (path_scoped_request
            && (workflow_confident || speech_confident_instruct)
            && !speech_confident_chat);

    let route = if preserve_workflow_route {
        top_non_chat_route(&distribution).unwrap_or_else(|| "SHELL".to_string())
    } else if fallback_to_chat {
        "CHAT".to_string()
    } else {
        distribution
            .first()
            .map(|(label, _)| label.clone())
            .unwrap_or_else(|| "CHAT".to_string())
    };
    let source = if preserve_workflow_route {
        format!(
            "preserve_workflow_route speech:{} workflow:{} mode:{}",
            speech_act.source, workflow.source, mode.source
        )
    } else if fallback_to_chat {
        format!(
            "conservative_chat_fallback speech:{} workflow:{} mode:{}",
            speech_act.source, workflow.source, mode.source
        )
    } else {
        format!(
            "speech:{} workflow:{} mode:{}",
            speech_act.source, workflow.source, mode.source
        )
    };

    // PRINCIPLE: Distinguish selection from categorization.
    // Categorization (DECIDE) needs a label. Selection (SELECT) needs a choice from workspace items.
    // Use speech act classification to determine if this is a selection request
    let is_selection_request = is_choice_confident(&speech_act, "INQUIRE", 0.0, 1.0)
        || is_choice_confident(&speech_act, "INSTRUCT", 0.0, 1.0);
    let route = if is_selection_request && is_choice_confident(&workflow, "DECIDE", 0.0, 1.0) {
        "SELECT".to_string()
    } else {
        route
    };
    let evidence_required = !route.eq_ignore_ascii_case("CHAT");

    Ok(RouteDecision {
        route,
        source,
        margin,
        entropy,
        distribution,
        speech_act,
        workflow,
        mode,
        // In the current runtime, we treat any non-CHAT route as requiring
        // workspace/tool evidence before answering.
        evidence_required,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decision(choice: &str, entropy: f64, margin: f64) -> ProbabilityDecision {
        ProbabilityDecision {
            choice: choice.to_string(),
            source: "test".to_string(),
            distribution: vec![(choice.to_string(), 1.0)],
            margin,
            entropy,
        }
    }

    #[test]
    fn chat_short_circuit_requires_consensus() {
        let speech = decision("CHAT", 0.0, 1.0);
        let workflow = decision("WORKFLOW", 0.0, 1.0);
        let routing_config = RoutingConfig::default();
        assert!(!should_short_circuit_chat_route(
            &speech,
            &workflow,
            &routing_config
        ));
    }

    #[test]
    fn classifier_parse_accepts_dsl_wrapped_by_unclosed_think_tag() {
        let parsed = parse_classifier_candidates(
            "<think>\nACT choice=1 label=CHAT reason=\"greeting\" entropy=0.1\n",
            speech_act_code_pairs(),
        )
        .expect("wrapped DSL should parse");
        assert_eq!(parsed.0, "CHAT");
        assert_eq!(parsed.1, 0.1);
        assert_eq!(parsed.2, "dsl_output");
    }

    #[test]
    fn classifier_repair_observation_shows_expected_template() {
        let template = "ROUTE choice=1|2 label=CHAT|WORKFLOW reason=\"justification\" entropy=N.N";
        let repair = classifier_repair_observation("<think>reasoning only", template);
        assert!(repair.contains("INVALID_FORMAT"));
        assert!(repair.contains("Expected: ROUTE"));
        assert!(repair.contains("Got: model returned prose/thinking or empty output"));
    }

    #[test]
    fn classifier_parse_accepts_field_only_dsl_without_prefix() {
        let parsed = parse_classifier_candidates(
            "choice=2 label=EXECUTE reason=\"run command\" entropy=0.1",
            mode_code_pairs(),
        )
        .expect("field-only DSL should parse");
        assert_eq!(parsed.0, "EXECUTE");
        assert_eq!(parsed.2, "dsl_field_only");
    }

    #[test]
    fn classifier_parse_accepts_json_fallback() {
        let parsed = parse_classifier_candidates(
            r#"{"choice":"1","label":"CHAT","reason":"greeting","entropy":0.1}"#,
            speech_act_code_pairs(),
        )
        .expect("JSON fallback should parse");
        assert_eq!(parsed.0, "CHAT");
        assert_eq!(parsed.2, "dsl_json_fallback");
    }

    #[test]
    fn classifier_parse_accepts_full_dsl_with_prefix() {
        let parsed = parse_classifier_candidates(
            "MODE choice=2 label=EXECUTE reason=\"run\" entropy=0.2",
            mode_code_pairs(),
        )
        .expect("full DSL should parse");
        assert_eq!(parsed.0, "EXECUTE");
        assert_eq!(parsed.2, "dsl_output");
    }

    #[test]
    fn classifier_parse_rejects_nonsense() {
        let parsed = parse_classifier_candidates("hello world", speech_act_code_pairs());
        assert!(parsed.is_none());
    }

    #[test]
    fn classifier_expected_template_for_mode() {
        let template = classifier_expected_template(mode_code_pairs());
        assert!(template.starts_with("MODE"));
        assert!(template.contains("INSPECT|EXECUTE|PLAN|MASTERPLAN|DECIDE"));
    }

    #[test]
    fn classifier_expected_template_for_speech_act() {
        let template = classifier_expected_template(speech_act_code_pairs());
        assert!(template.starts_with("ACT"));
        assert!(template.contains("CHAT|INSTRUCT|INQUIRE"));
    }

    #[test]
    fn chat_short_circuit_allows_high_confidence_agreement() {
        let speech = decision("CHAT", 0.0, 1.0);
        let workflow = decision("CHAT", 0.0, 1.0);
        let routing_config = RoutingConfig::default();
        assert!(should_short_circuit_chat_route(
            &speech,
            &workflow,
            &routing_config
        ));
    }

    #[test]
    fn speech_chat_boost_is_disabled_when_workflow_is_confidently_not_chat() {
        let workflow = decision("WORKFLOW", 0.0, 1.0);
        assert!(!should_apply_speech_chat_boost(&workflow));
    }

    #[test]
    fn speech_chat_boost_is_allowed_when_workflow_is_confident_chat() {
        let workflow = decision("CHAT", 0.0, 1.0);
        assert!(should_apply_speech_chat_boost(&workflow));
    }

    #[test]
    fn top_non_chat_route_prefers_highest_non_chat_candidate() {
        let distribution = vec![
            ("CHAT".to_string(), 0.60),
            ("SHELL".to_string(), 0.25),
            ("DECIDE".to_string(), 0.15),
        ];
        assert_eq!(top_non_chat_route(&distribution).as_deref(), Some("SHELL"));
    }

    #[test]
    fn classifier_parse_fallback_prefers_non_chat_safe_route() {
        assert_eq!(
            conservative_classifier_fallback(workflow_code_pairs()),
            "WORKFLOW"
        );
        assert_eq!(
            conservative_classifier_fallback(speech_act_code_pairs()),
            "INSTRUCT"
        );
        assert_eq!(
            conservative_classifier_fallback(mode_code_pairs()),
            "INSPECT"
        );
    }
}
