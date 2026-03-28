use crate::*;

pub(crate) fn extract_first_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &b) in bytes.iter().enumerate() {
        if start.is_none() {
            if b == b'{' {
                start = Some(i);
                depth = 1;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let s = start?;
                    return text.get(s..=i);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn parse_json_loose<T: DeserializeOwned>(text: &str) -> Result<T> {
    if let Ok(v) = serde_json::from_str::<T>(text.trim()) {
        return Ok(v);
    }
    if let Some(obj) = extract_first_json_object(text) {
        return serde_json::from_str::<T>(obj.trim())
            .context("Failed to parse extracted JSON object");
    }
    anyhow::bail!("No JSON object found")
}

pub(crate) fn workflow_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[("1", "CHAT"), ("2", "WORKFLOW")]
}

pub(crate) fn mode_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("1", "INSPECT"),
        ("2", "EXECUTE"),
        ("3", "PLAN"),
        ("4", "MASTERPLAN"),
        ("5", "DECIDE"),
    ]
}

pub(crate) fn speech_act_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("1", "CAPABILITY_CHECK"),
        ("2", "INFO_REQUEST"),
        ("3", "ACTION_REQUEST"),
    ]
}

pub(crate) fn route_label_from_router_output(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<&'static str> {
    let token = raw
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim();
    for (code, label) in pairs {
        if token == *code || token.eq_ignore_ascii_case(label) {
            return Some(label);
        }
    }
    None
}

pub(crate) fn logsumexp(values: &[f64]) -> f64 {
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max_v.is_finite() {
        return f64::NEG_INFINITY;
    }
    let sum = values.iter().map(|v| (v - max_v).exp()).sum::<f64>();
    max_v + sum.ln()
}

pub(crate) fn parse_router_distribution(
    logprobs: &serde_json::Value,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<Vec<(String, f64)>> {
    let top_logprobs = logprobs
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .and_then(|v| v.get("top_logprobs"))
        .and_then(|v| v.as_array())?;

    let mut route_logprobs: HashMap<String, Vec<f64>> = HashMap::new();
    for item in top_logprobs {
        let token = item
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let Some(logprob) = item.get("logprob").and_then(|v| v.as_f64()) else {
            continue;
        };
        if let Some(label) = route_label_from_router_output(token, pairs) {
            route_logprobs
                .entry(label.to_string())
                .or_default()
                .push(logprob);
        }
    }
    if route_logprobs.is_empty() {
        return None;
    }

    let mut entries: Vec<(String, f64)> = pairs
        .iter()
        .map(|(_, label)| {
            let lp = route_logprobs
                .get(*label)
                .map(|values| logsumexp(values))
                .unwrap_or(f64::NEG_INFINITY);
            ((*label).to_string(), lp)
        })
        .collect();

    let max_lp = entries
        .iter()
        .map(|(_, lp)| *lp)
        .filter(|lp| lp.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_lp.is_finite() {
        return None;
    }
    let denom = entries
        .iter()
        .map(|(_, lp)| {
            if lp.is_finite() {
                (lp - max_lp).exp()
            } else {
                0.0
            }
        })
        .sum::<f64>();
    if denom <= 0.0 {
        return None;
    }
    for (_, lp) in &mut entries {
        let p = if lp.is_finite() {
            (*lp - max_lp).exp() / denom
        } else {
            0.0
        };
        *lp = p;
    }
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Some(entries)
}

pub(crate) fn route_margin(distribution: &[(String, f64)]) -> f64 {
    let top = distribution.first().map(|(_, p)| *p).unwrap_or(0.0);
    let second = distribution.get(1).map(|(_, p)| *p).unwrap_or(0.0);
    top - second
}

pub(crate) fn route_entropy(distribution: &[(String, f64)]) -> f64 {
    distribution
        .iter()
        .map(|(_, p)| if *p > 0.0 { -p * p.ln() } else { 0.0 })
        .sum()
}

pub(crate) fn format_route_distribution(distribution: &[(String, f64)]) -> String {
    distribution
        .iter()
        .map(|(route, p)| format!("{route}:{p:.2}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn probability_of(distribution: &[(String, f64)], label: &str) -> f64 {
    distribution
        .iter()
        .find(|(name, _)| name == label)
        .map(|(_, p)| *p)
        .unwrap_or(0.0)
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
        n_probs: Some(router_cal.n_probs.max(16)),
        repeat_penalty: Some(router_cfg.repeat_penalty),
        reasoning_format: Some(router_cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let raw = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    let fallback_choice = pairs
        .first()
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| "CHAT".to_string());
    let chosen = route_label_from_router_output(&raw, pairs)
        .unwrap_or(fallback_choice.as_str())
        .to_string();

    let logprob_distribution = resp
        .choices
        .get(0)
        .and_then(|c| c.logprobs.as_ref())
        .and_then(|v| parse_router_distribution(v, pairs));
    let used_logprobs = logprob_distribution.is_some();
    let mut distribution = logprob_distribution.unwrap_or_else(|| {
        pairs
            .iter()
            .map(|(_, label)| {
                (
                    (*label).to_string(),
                    if *label == chosen { 1.0 } else { 0.0 },
                )
            })
            .collect()
    });
    distribution.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let source = if used_logprobs {
        "logprobs"
    } else {
        "token_only"
    };

    let route = distribution
        .first()
        .map(|(label, _)| label.clone())
        .unwrap_or(chosen);

    Ok(ProbabilityDecision {
        choice: route,
        source: source.to_string(),
        margin: route_margin(&distribution),
        entropy: route_entropy(&distribution),
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
        "User message:\n{user_message}\n\nConversation so far (most recent last):\n{}",
        conversation
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
    let capability_p = probability_of(&speech_act.distribution, "CAPABILITY_CHECK");
    for (label, p) in &mut distribution {
        if label == "CHAT" {
            *p = capability_p + (1.0 - capability_p) * *p;
        } else {
            *p *= 1.0 - capability_p;
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
