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

pub(crate) async fn infer_digit_router(
    client: &reqwest::Client,
    chat_url: &Url,
    router_cfg: &Profile,
    router_cal: &RouterCalibration,
    prompt: String,
    pairs: &'static [(&'static str, &'static str)],
) -> Result<ProbabilityDecision> {
    let req = chat_request_system_user(
        router_cfg,
        &router_cfg.system_prompt,
        &prompt,
        ChatRequestOptions::default(),
    );
    let resp = chat_once_with_timeout(client, chat_url, &req, router_cfg.timeout_s.min(45)).await?;
    let raw = resp
        .choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();
    let fallback_choice = pairs
        .first()
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| "CHAT".to_string());

    // Parse JSON output to get choice, label, and entropy
    let json_entropy = extract_entropy(&raw);
    let chosen = extract_label(&raw, pairs)
        .unwrap_or(fallback_choice.as_str())
        .to_string();

    // Build distribution from JSON choice (not logprobs)
    let other_count = (pairs.len() as f64 - 1.0).max(1.0);
    let entropy_val = json_entropy.unwrap_or(0.0);
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

    let source = "json_output";

    // Use JSON entropy
    let raw_entropy = json_entropy.unwrap_or_else(|| route_entropy(&distribution));
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
        } else if is_choice_confident(&speech_act, "INSTRUCT", 0.0, 1.0) {
            "EXECUTE"
        } else {
            "INSPECT"
        };
        fallback_probability_decision(choice, mode_code_pairs(), "fallback")
    });

    let chat_p = probability_of(&workflow.distribution, "CHAT");
    let workflow_p = probability_of(&workflow.distribution, "WORKFLOW");
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
    let chat_p = probability_of(&speech_act.distribution, "CHAT");
    let instruct_p = probability_of(&speech_act.distribution, "INSTRUCT");

    // If "CHAT" is high, boost CHAT route
    if chat_p > 0.5 && should_apply_speech_chat_boost(&workflow) {
        for (label, p) in &mut distribution {
            if label == "CHAT" {
                *p = chat_p + (1.0 - chat_p) * *p;
            } else {
                *p *= 1.0 - chat_p;
            }
        }
    }

    // If "INSTRUCT" is high, boost non-CHAT routes (user wants action)
    if instruct_p > 0.5 {
        let current_chat_p = probability_of(&distribution, "CHAT");
        let workflow_boost = (instruct_p - 0.5) * 0.4; // Up to 40% boost
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
    let fallback_to_chat = (is_choice_confident(&speech_act, "CHAT", 0.0, 1.0)
        && (margin < 0.20 || entropy > 0.60))
        || (margin < 0.12); // Hard fallback for any case where we are absolutely guessing
    let preserve_workflow_route = path_scoped_request
        && (workflow_confident || speech_confident_instruct)
        && !speech_confident_chat;

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

    Ok(RouteDecision {
        route,
        source,
        margin,
        entropy,
        distribution,
        speech_act,
        workflow,
        mode,
        evidence_required: false, // Task 290: Will be set by evidence classifier
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
}
