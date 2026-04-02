//! @efficiency-role: service-orchestrator
//!
//! Routing - Inference Functions

use crate::*;

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
        n_probs: Some(router_cal.n_probs.max(16)),
        repeat_penalty: Some(router_cfg.repeat_penalty),
        reasoning_format: Some(router_cfg.reasoning_format.clone()),
        grammar: None,
    };
    let resp = chat_once(client, chat_url, &req).await?;
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
    .await?;

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
    .await?;

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
    .await?;

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
    if chat_p > 0.5 {
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
    let route = distribution
        .first()
        .map(|(label, _)| label.clone())
        .unwrap_or_else(|| "CHAT".to_string());

    Ok(RouteDecision {
        route,
        source: format!(
            "speech:{} workflow:{} mode:{}",
            speech_act.source, workflow.source, mode.source
        ),
        margin: route_margin(&distribution),
        entropy: route_entropy(&distribution),
        distribution,
        speech_act,
        workflow,
        mode,
    })
}
