//! @efficiency-role: infra-adapter

use crate::*;

async fn fetch_models_response(client: &reqwest::Client, base_url: &Url) -> Result<ModelsList> {
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        anyhow::bail!("GET /v1/models returned HTTP {status}: {text}");
    }
    serde_json::from_str(&text).context("Invalid JSON from /v1/models")
}

fn model_id(item: ModelItem) -> Option<String> {
    item.id
        .or(item.name)
        .or(item.model)
        .filter(|id| !id.trim().is_empty())
}

pub(crate) async fn fetch_first_model_id(
    client: &reqwest::Client,
    base_url: &Url,
) -> Result<String> {
    let parsed = fetch_models_response(client, base_url).await?;
    for item in parsed
        .data
        .or(parsed.models)
        .unwrap_or_default()
        .into_iter()
    {
        if let Some(id) = model_id(item) {
            return Ok(id);
        }
    }
    anyhow::bail!("No model ids found in /v1/models response")
}

pub(crate) async fn fetch_all_model_ids(
    client: &reqwest::Client,
    base_url: &Url,
) -> Result<Vec<String>> {
    let parsed = fetch_models_response(client, base_url).await?;
    let mut out = Vec::new();
    for item in parsed.data.or(parsed.models).unwrap_or_default() {
        if let Some(id) = model_id(item) {
            let id = id.trim().to_string();
            if !out.contains(&id) {
                out.push(id);
            }
        }
    }
    if out.is_empty() {
        anyhow::bail!("No model ids found in /v1/models response");
    }
    Ok(out)
}

/// Try to fetch n_ctx from a given endpoint, extracting the value at a JSON path
async fn try_fetch_ctx(
    client: &reqwest::Client,
    base_url: &Url,
    endpoint: &str,
    path: impl FnOnce(&serde_json::Value) -> Option<u64>,
) -> Option<Option<u64>> {
    let url = base_url.join(endpoint).ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return Some(None);
    }
    let text = resp.text().await.ok()?;
    let v = serde_json::from_str::<serde_json::Value>(&text).ok()?;
    Some(path(&v))
}

/// Try to fetch n_ctx from /slots endpoint
async fn try_fetch_ctx_from_slots(client: &reqwest::Client, base_url: &Url) -> Option<Option<u64>> {
    try_fetch_ctx(client, base_url, "/slots", |v| {
        v.get(0)
            .and_then(|s| s.get("n_ctx"))
            .and_then(|x| x.as_u64())
    })
    .await
}

/// Try to fetch n_ctx from /props endpoint
async fn try_fetch_ctx_from_props(client: &reqwest::Client, base_url: &Url) -> Option<Option<u64>> {
    try_fetch_ctx(client, base_url, "/props", |v| {
        v.get("default_generation_settings")
            .and_then(|d| d.get("n_ctx"))
            .and_then(|x| x.as_u64())
    })
    .await
}

/// Try to fetch n_ctx from /v1/models metadata
async fn try_fetch_ctx_from_models(
    client: &reqwest::Client,
    base_url: &Url,
) -> Result<Option<u64>> {
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        return Ok(None);
    }
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    Ok(v.get("data")
        .and_then(|d| d.get(0))
        .and_then(|m| m.get("meta"))
        .and_then(|meta| meta.get("n_ctx_train"))
        .and_then(|x| x.as_u64()))
}

pub(crate) async fn fetch_ctx_max(client: &reqwest::Client, base_url: &Url) -> Result<Option<u64>> {
    if let Some(n) = try_fetch_ctx_from_slots(client, base_url).await {
        return Ok(n);
    }
    if let Some(n) = try_fetch_ctx_from_props(client, base_url).await {
        return Ok(n);
    }
    try_fetch_ctx_from_models(client, base_url).await
}

pub(crate) async fn fetch_runtime_generation_defaults(
    client: &reqwest::Client,
    base_url: &Url,
) -> Result<Option<RuntimeGenerationDefaults>> {
    let Ok(url) = base_url.join("/props") else {
        return Ok(None);
    };
    let Ok(resp) = client.get(url).send().await else {
        return Ok(None);
    };
    if !resp.status().is_success() {
        return Ok(None);
    }
    let Ok(text) = resp.text().await else {
        return Ok(None);
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Ok(None);
    };
    let Some(defaults) = value.get("default_generation_settings") else {
        return Ok(None);
    };

    let temperature = defaults.get("temperature").and_then(|v| v.as_f64());
    let top_p = defaults.get("top_p").and_then(|v| v.as_f64());
    let repeat_penalty = defaults
        .get("repeat_penalty")
        .and_then(|v| v.as_f64())
        .or_else(|| {
            defaults
                .get("repeat_penalty")
                .and_then(|v| v.as_u64())
                .map(|v| v as f64)
        });
    let max_tokens = defaults
        .get("n_predict")
        .and_then(|v| v.as_u64())
        .or_else(|| defaults.get("max_tokens").and_then(|v| v.as_u64()))
        .map(|v| v.min(u32::MAX as u64) as u32);

    if temperature.is_none() && top_p.is_none() && repeat_penalty.is_none() && max_tokens.is_none()
    {
        return Ok(None);
    }
    Ok(Some(RuntimeGenerationDefaults {
        temperature,
        top_p,
        repeat_penalty,
        max_tokens,
        source: "/props.default_generation_settings".to_string(),
    }))
}

async fn probe_chat_completion_raw(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse> {
    let resp = client
        .post(chat_url.clone())
        .json(req)
        .send()
        .await
        .context("model behavior probe request failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("failed to read model behavior probe body")?;
    if !status.is_success() {
        anyhow::bail!("model behavior probe returned HTTP {status}: {text}");
    }
    let mut parsed: ChatCompletionResponse =
        serde_json::from_str(&text).context("invalid JSON from model behavior probe")?;
    isolate_reasoning_fields(&mut parsed);
    Ok(parsed)
}

fn response_content(resp: &ChatCompletionResponse) -> String {
    resp.choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn probe_json_clean(resp: &ChatCompletionResponse) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&response_content(resp)) else {
        return false;
    };
    value
        .get("status")
        .and_then(|v| v.as_str())
        .map(|v| v == "ok")
        .unwrap_or(false)
}

async fn probe_logprobs_support(
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
) -> Result<bool> {
    let req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: vec![
            ChatMessage::simple("system", &"Return exactly one digit: 1.".to_string()),
            ChatMessage::simple("user", &"ping".to_string()),
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: 1,
        n_probs: Some(8),
        repeat_penalty: Some(1.0),
        reasoning_format: Some("none".to_string()),
        grammar: None,
    tools: None,
    };
    let resp = probe_chat_completion_raw(client, chat_url, &req).await?;
    Ok(resp
        .choices
        .get(0)
        .and_then(|c| c.logprobs.as_ref())
        .is_some())
}

/// Build a probe chat request
fn make_probe_request(
    model_id: &str,
    reasoning_format: &str,
    system_content: &str,
    user_content: &str,
    max_tokens: u32,
) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model_id.to_string(),
        messages: vec![
            ChatMessage::simple("system", &system_content.to_string()),
            ChatMessage::simple("user", &user_content.to_string()),
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens,
        n_probs: None,
        repeat_penalty: Some(1.0),
        reasoning_format: Some(reasoning_format.to_string()),
        grammar: None,
    tools: None,
    }
}

/// Probe results for behavior detection
struct BehaviorProbe {
    auto_exact_content: String,
    auto_reasoning: bool,
    auto_truncated_before_final: bool,
    none_exact_content: String,
    none_has_thinking: bool,
    none_leak_suspected: bool,
    json_clean_with_auto: bool,
    json_clean_with_none: bool,
}

fn first_msg_content(resp: &ChatCompletionResponse) -> (Option<&str>, Option<&str>) {
    (
        resp.choices
            .get(0)
            .and_then(|c| c.message.content.as_deref()),
        resp.choices
            .get(0)
            .and_then(|c| c.message.reasoning_content.as_deref()),
    )
}

fn preferred_reasoning_format(probe: &BehaviorProbe) -> String {
    if probe.auto_reasoning && (probe.none_exact_content != "7" || !probe.json_clean_with_none) {
        "auto".to_string()
    } else {
        "none".to_string()
    }
}

fn detect_behavior_probe(
    auto_exact_resp: &ChatCompletionResponse,
    none_exact_resp: &ChatCompletionResponse,
    auto_json_resp: &ChatCompletionResponse,
    none_json_resp: &ChatCompletionResponse,
    exact_token: &str,
) -> BehaviorProbe {
    let auto_exact_content = response_content(auto_exact_resp);
    let none_exact_content = response_content(none_exact_resp);
    let (auto_content, auto_reasoning) = first_msg_content(auto_exact_resp);
    let (none_content, _) = first_msg_content(none_exact_resp);
    let auto_reasoning = thinking_content::is_thinking_model(auto_content, auto_reasoning);
    let auto_truncated_before_final = auto_reasoning
        && auto_exact_content.is_empty()
        && auto_exact_resp
            .choices
            .get(0)
            .and_then(|c| c.finish_reason.as_deref())
            == Some("length");
    let none_has_thinking =
        thinking_content::is_thinking_model(none_content, first_msg_content(none_exact_resp).1);
    let none_leak_suspected = !none_exact_content.eq(exact_token)
        && !none_has_thinking
        && none_exact_content.lines().count() > 1;
    BehaviorProbe {
        auto_exact_content,
        auto_reasoning,
        auto_truncated_before_final,
        none_exact_content,
        none_has_thinking,
        none_leak_suspected,
        json_clean_with_auto: probe_json_clean(auto_json_resp),
        json_clean_with_none: probe_json_clean(none_json_resp),
    }
}

pub(crate) async fn ensure_model_behavior_profile(
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
) -> Result<ModelBehaviorProfile> {
    let path = model_cfg_dir.join("model_behavior.toml");
    if let Ok(existing) = load_model_behavior_profile(&path) {
        if existing.version == 3 && existing.model == model_id && existing.base_url == base_url {
            return Ok(existing);
        }
    }

    let exact_token = "7";
    let mk = |rf, sc, uc, mt| make_probe_request(model_id, rf, sc, uc, mt);
    let auto_exact_resp = probe_chat_completion_raw(
        client,
        chat_url,
        &mk(
            "auto",
            "Return exactly 7 and nothing else.",
            "Return 7.",
            64,
        ),
    )
    .await?;
    let none_exact_resp = probe_chat_completion_raw(
        client,
        chat_url,
        &mk(
            "none",
            "Return exactly 7 and nothing else.",
            "Return 7.",
            64,
        ),
    )
    .await?;
    let auto_json_resp = probe_chat_completion_raw(
        client,
        chat_url,
        &mk(
            "auto",
            "Return exactly one JSON object: {\"status\":\"ok\"}. No prose.",
            "Return the JSON object now.",
            128,
        ),
    )
    .await?;
    let none_json_resp = probe_chat_completion_raw(
        client,
        chat_url,
        &mk(
            "none",
            "Return exactly one JSON object: {\"status\":\"ok\"}. No prose.",
            "Return the JSON object now.",
            128,
        ),
    )
    .await?;

    let probe = detect_behavior_probe(
        &auto_exact_resp,
        &none_exact_resp,
        &auto_json_resp,
        &none_json_resp,
        exact_token,
    );
    let preferred_rf = preferred_reasoning_format(&probe);
    let profile = ModelBehaviorProfile {
        version: 3,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        auto_reasoning_separated: probe.auto_reasoning,
        auto_final_clean: probe.auto_exact_content == exact_token,
        auto_truncated_before_final: probe.auto_truncated_before_final,
        none_final_clean: probe.none_exact_content == exact_token,
        none_reasoning_leak_suspected: probe.none_leak_suspected,
        json_clean_with_auto: probe.json_clean_with_auto,
        json_clean_with_none: probe.json_clean_with_none,
        needs_text_finalizer: probe.auto_reasoning && (probe.auto_exact_content != exact_token),
        preferred_reasoning_format: preferred_rf,
    };
    let _ = probe_logprobs_support(client, chat_url, model_id)
        .await
        .ok();
    save_model_behavior_profile(&path, &profile)?;
    Ok(profile)
}

pub(crate) fn isolate_reasoning_fields(resp: &mut ChatCompletionResponse) {
    for choice in &mut resp.choices {
        let content = choice.message.content.as_deref();
        let reasoning = choice.message.reasoning_content.as_deref();
        let extraction = thinking_content::extract_thinking(content, reasoning);
        choice.message.reasoning_content = extraction.thinking.filter(|s| !s.trim().is_empty());
        choice.message.content = if extraction.final_answer.trim().is_empty() {
            match content {
                Some(existing) if !existing.trim().is_empty() => Some(existing.trim().to_string()),
                _ => None,
            }
        } else {
            Some(extraction.final_answer)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolates_reasoning_from_tagged_content() {
        let mut resp = ChatCompletionResponse {
            choices: vec![Choice {
                message: ChoiceMessage {
                    role: Some("assistant".to_string()),
                    content: Some(
                        "<<<reasoning_content_start>>>thoughts<<<reasoning_content_end>>>answer"
                            .to_string(),
                    ),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            id: None,
            created: None,
            model: None,
            system_fingerprint: None,
            usage: None,
            timings: None,
        };
        isolate_reasoning_fields(&mut resp);
        assert_eq!(resp.choices[0].message.content.as_deref(), Some("answer"));
        assert_eq!(
            resp.choices[0].message.reasoning_content.as_deref(),
            Some("thoughts")
        );
    }

    #[test]
    fn keeps_response_usable_when_reasoning_absent() {
        let mut resp = ChatCompletionResponse {
            choices: vec![Choice {
                message: ChoiceMessage {
                    role: Some("assistant".to_string()),
                    content: Some("plain answer".to_string()),
                    reasoning_content: None,
                    tool_calls: None,
                },
                finish_reason: None,
                logprobs: None,
            }],
            id: None,
            created: None,
            model: None,
            system_fingerprint: None,
            usage: None,
            timings: None,
        };
        isolate_reasoning_fields(&mut resp);
        assert_eq!(
            resp.choices[0].message.content.as_deref(),
            Some("plain answer")
        );
        assert!(resp.choices[0].message.reasoning_content.is_none());
    }
}
