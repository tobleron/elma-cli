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
    _speech_act_cfg: &Profile,
    _workflow_router_cfg: &Profile,
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

    let mode_prompt = format!(
        "User message:\n{user_message}\n\nWorkspace facts:\n{facts}\n\nWorkspace brief:\n{brief}\n\nConversation so far (most recent last):\n{conversation}",
        user_message = user_message,
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation,
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
        fallback_probability_decision("EXECUTE", mode_code_pairs(), "fallback")
    });

    // Map mode classification to route
    let route = match mode.choice.as_str() {
        "EXECUTE" | "INSPECT" => "SHELL",
        "DECIDE" => "CHAT",
        "PLAN" => "PLAN",
        "MASTERPLAN" => "MASTERPLAN",
        _ => "SHELL",
    };

    let distribution = vec![
        ("CHAT".to_string(), probability_of(&mode.distribution, "DECIDE")),
        ("SHELL".to_string(), probability_of(&mode.distribution, "EXECUTE") + probability_of(&mode.distribution, "INSPECT")),
        ("PLAN".to_string(), probability_of(&mode.distribution, "PLAN")),
        ("MASTERPLAN".to_string(), probability_of(&mode.distribution, "MASTERPLAN")),
        ("DECIDE".to_string(), 0.0),
    ];

    let margin = route_margin(&distribution);
    let entropy = route_entropy(&distribution);

    Ok(RouteDecision {
        route: route.to_string(),
        source: format!("mode:{} entropy:{:.2} margin:{:.2}", mode.choice, entropy, margin),
        margin,
        entropy,
        distribution,
        speech_act: ProbabilityDecision {
            choice: "INSTRUCT".to_string(),
            source: "mode_derived".to_string(),
            distribution: vec![("INSTRUCT".to_string(), 1.0), ("CHAT".to_string(), 0.0)],
            margin: 1.0,
            entropy: 0.0,
        },
        workflow: ProbabilityDecision {
            choice: "WORKFLOW".to_string(),
            source: "mode_derived".to_string(),
            distribution: vec![("WORKFLOW".to_string(), 1.0), ("CHAT".to_string(), 0.0)],
            margin: 1.0,
            entropy: 0.0,
        },
        mode,
        evidence_required: false,
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
