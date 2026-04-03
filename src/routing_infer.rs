//! @efficiency-role: service-orchestrator
//!
//! Routing - Inference Functions

use crate::*;

fn should_short_circuit_chat_route(
    speech_act: &ProbabilityDecision,
    workflow: &ProbabilityDecision,
) -> bool {
    speech_act.choice.eq_ignore_ascii_case("CHAT")
        && speech_act.entropy <= 0.20
        && speech_act.margin >= 0.70
        && workflow.choice.eq_ignore_ascii_case("CHAT")
        && workflow.entropy <= 0.20
        && workflow.margin >= 0.70
}

fn should_apply_speech_chat_boost(workflow: &ProbabilityDecision) -> bool {
    workflow.choice.eq_ignore_ascii_case("CHAT") || workflow.margin < 0.15 || workflow.entropy > 0.50
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
    let req = ChatCompletionRequest {
        model: router_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: router_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: router_cfg.temperature,
        top_p: router_cfg.top_p,
        stream: false,
        max_tokens: router_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(router_cfg.repeat_penalty),
        reasoning_format: Some(router_cfg.reasoning_format.clone()),
        grammar: None,
    };
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
    let mut distribution: Vec<(String, f64)> = pairs
        .iter()
        .map(|(_, label)| {
            let p = if *label == chosen {
                1.0 - json_entropy.unwrap_or(0.0)
            } else {
                0.0
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
    .unwrap_or_else(|_| fallback_probability_decision("INQUIRE", speech_act_code_pairs(), "fallback"));

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
        let choice = if speech_act.choice.eq_ignore_ascii_case("CHAT") {
            "CHAT"
        } else {
            "WORKFLOW"
        };
        fallback_probability_decision(choice, workflow_code_pairs(), "fallback")
    });

    if should_short_circuit_chat_route(&speech_act, &workflow) {
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
        let choice = if workflow.choice.eq_ignore_ascii_case("CHAT") {
            "DECIDE"
        } else if speech_act.choice.eq_ignore_ascii_case("INSTRUCT") {
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
    let route = if speech_act.choice.eq_ignore_ascii_case("CHAT") && margin < 0.15 {
        "CHAT".to_string()
    } else {
        distribution
            .first()
            .map(|(label, _)| label.clone())
            .unwrap_or_else(|| "CHAT".to_string())
    };
    let source = if speech_act.choice.eq_ignore_ascii_case("CHAT") && margin < 0.15 {
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

    Ok(RouteDecision {
        route,
        source,
        margin,
        entropy,
        distribution,
        speech_act,
        workflow,
        mode,
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
        assert!(!should_short_circuit_chat_route(&speech, &workflow));
    }

    #[test]
    fn chat_short_circuit_allows_high_confidence_agreement() {
        let speech = decision("CHAT", 0.0, 1.0);
        let workflow = decision("CHAT", 0.0, 1.0);
        assert!(should_short_circuit_chat_route(&speech, &workflow));
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
}
