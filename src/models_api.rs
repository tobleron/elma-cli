use crate::*;

pub(crate) async fn fetch_first_model_id(client: &reqwest::Client, base_url: &Url) -> Result<String> {
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
    let parsed: ModelsList = serde_json::from_str(&text).context("Invalid JSON from /v1/models")?;
    let list = parsed
        .data
        .or(parsed.models)
        .unwrap_or_default()
        .into_iter();
    for item in list {
        if let Some(id) = item.id.or(item.name).or(item.model) {
            if !id.trim().is_empty() {
                return Ok(id);
            }
        }
    }
    anyhow::bail!("No model ids found in /v1/models response")
}

pub(crate) async fn fetch_all_model_ids(client: &reqwest::Client, base_url: &Url) -> Result<Vec<String>> {
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
    let parsed: ModelsList = serde_json::from_str(&text).context("Invalid JSON from /v1/models")?;
    let mut out = Vec::new();
    let list = parsed.data.or(parsed.models).unwrap_or_default();
    for item in list {
        if let Some(id) = item.id.or(item.name).or(item.model) {
            let id = id.trim().to_string();
            if !id.is_empty() && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    if out.is_empty() {
        anyhow::bail!("No model ids found in /v1/models response");
    }
    Ok(out)
}

pub(crate) async fn fetch_ctx_max(client: &reqwest::Client, base_url: &Url) -> Result<Option<u64>> {
    // Best-effort, ordered by "most likely runtime truth":
    // 1) /slots[0].n_ctx (runtime ctx size)
    // 2) /props.default_generation_settings.n_ctx (runtime default)
    // 3) /v1/models meta.n_ctx_train (training ctx, can be larger than runtime)

    if let Ok(url) = base_url.join("/slots") {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let n = v
                            .get(0)
                            .and_then(|s| s.get("n_ctx"))
                            .and_then(|x| x.as_u64());
                        if n.is_some() {
                            return Ok(n);
                        }
                    }
                }
            }
        }
    }

    if let Ok(url) = base_url.join("/props") {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let n = v
                            .get("default_generation_settings")
                            .and_then(|d| d.get("n_ctx"))
                            .and_then(|x| x.as_u64());
                        if n.is_some() {
                            return Ok(n);
                        }
                    }
                }
            }
        }
    }

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
    let content = response_content(resp);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
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
            ChatMessage {
                role: "system".to_string(),
                content: "Return exactly one digit: 1.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            },
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: 1,
        n_probs: Some(8),
        repeat_penalty: Some(1.0),
        reasoning_format: Some("none".to_string()),
    };
    let resp = probe_chat_completion_raw(client, chat_url, &req).await?;
    Ok(resp
        .choices
        .get(0)
        .and_then(|choice| choice.logprobs.as_ref())
        .is_some())
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
        if existing.version == 3
            && existing.model == model_id
            && existing.base_url == base_url
        {
            return Ok(existing);
        }
    }

    let exact_token = "7";
    let json_token = r#"{"status":"ok"}"#;

    let make_req = |reasoning_format: &str, system_content: &str, user_content: &str, max_tokens| {
        ChatCompletionRequest {
            model: model_id.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_content.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_content.to_string(),
                },
            ],
            temperature: 0.0,
            top_p: 1.0,
            stream: false,
            max_tokens,
            n_probs: None,
            repeat_penalty: Some(1.0),
            reasoning_format: Some(reasoning_format.to_string()),
        }
    };

    let auto_exact_resp = probe_chat_completion_raw(
        client,
        chat_url,
        &make_req(
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
        &make_req(
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
        &make_req(
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
        &make_req(
            "none",
            "Return exactly one JSON object: {\"status\":\"ok\"}. No prose.",
            "Return the JSON object now.",
            128,
        ),
    )
    .await?;

    let auto_exact_content = response_content(&auto_exact_resp);
    let none_exact_content = response_content(&none_exact_resp);
    
    // Use centralized thinking detection
    let auto_reasoning = thinking_content::is_thinking_model(
        auto_exact_resp.choices.get(0).and_then(|c| c.message.content.as_deref()),
        auto_exact_resp.choices.get(0).and_then(|c| c.message.reasoning_content.as_deref()),
    );
    
    let auto_truncated_before_final = auto_reasoning
        && auto_exact_content.is_empty()
        && auto_exact_resp
            .choices
            .get(0)
            .and_then(|c| c.finish_reason.as_deref())
            == Some("length");
    
    // Check for thinking in none mode as well
    let none_has_thinking = thinking_content::is_thinking_model(
        none_exact_resp.choices.get(0).and_then(|c| c.message.content.as_deref()),
        none_exact_resp.choices.get(0).and_then(|c| c.message.reasoning_content.as_deref()),
    );
    
    let none_leak_suspected = !none_exact_content.eq(exact_token)
        && !none_has_thinking
        && none_exact_content.lines().count() > 1;

    let profile = ModelBehaviorProfile {
        version: 3,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        auto_reasoning_separated: auto_reasoning,
        auto_final_clean: auto_exact_content == exact_token,
        auto_truncated_before_final,
        none_final_clean: none_exact_content == exact_token,
        none_reasoning_leak_suspected: none_leak_suspected,
        json_clean_with_auto: probe_json_clean(&auto_json_resp),
        json_clean_with_none: probe_json_clean(&none_json_resp),
        needs_text_finalizer: auto_reasoning && (auto_exact_content != exact_token),
        preferred_reasoning_format: if auto_reasoning
            && (none_exact_content != exact_token || !probe_json_clean(&none_json_resp))
        {
            "auto".to_string()
        } else {
            "none".to_string()
        },
    };

    let _supports_logprobs = probe_logprobs_support(client, chat_url, model_id).await.ok();
    save_model_behavior_profile(&path, &profile)?;
    Ok(profile)
}

pub(crate) fn isolate_reasoning_fields(resp: &mut ChatCompletionResponse) {
    for choice in &mut resp.choices {
        let content = choice.message.content.as_deref();
        let reasoning = choice.message.reasoning_content.as_deref();
        
        // Use centralized thinking extraction
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
        assert_eq!(resp.choices[0].message.content.as_deref(), Some("plain answer"));
        assert!(resp.choices[0].message.reasoning_content.is_none());
    }
}
