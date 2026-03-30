//! @efficiency-role: service-orchestrator
//!
//! UI - Chat Functions

use crate::*;

async fn chat_once_base(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: Option<u64>,
) -> Result<ChatCompletionResponse> {
    let mut effective_req = req.clone();
    let original_reasoning = req.reasoning_format.clone();
    effective_req.reasoning_format = effective_reasoning_format(req);
    if effective_req.reasoning_format != original_reasoning {
        append_trace_log_line(&format!(
            "trace: reasoning_format_override requested={} effective={} model={}",
            original_reasoning.as_deref().unwrap_or("-"),
            effective_req.reasoning_format.as_deref().unwrap_or("-"),
            effective_req.model
        ));
    }

    let timeout_secs = timeout_s.unwrap_or(120);
    let mut last_error = String::new();
    let mut is_timeout = false;

    for attempt in 0..3u32 {
        let request_builder = client
            .post(chat_url.clone())
            .json(&effective_req)
            .timeout(Duration::from_secs(timeout_secs));

        match request_builder.send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.context("Failed to read response body")?;
                if !status.is_success() {
                    if status.is_server_error() && attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                        last_error = format!("Server returned HTTP {status}: {text}");
                        continue;
                    }
                    anyhow::bail!("Server returned HTTP {status}: {text}");
                }

                let mut parsed: ChatCompletionResponse =
                    serde_json::from_str(&text).context("Invalid JSON from server")?;
                isolate_reasoning_fields(&mut parsed);
                append_reasoning_audit_record(&effective_req, &parsed);
                maybe_display_reasoning_trace(&parsed);
                return Ok(parsed);
            }
            Err(e) => {
                if e.is_timeout() || e.to_string().contains("timeout") {
                    is_timeout = true;
                    last_error = format!("Model API timeout after {}s (attempt {}/{})", timeout_secs, attempt + 1, 3);
                } else {
                    last_error = format!("{e:#}");
                }

                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                    continue;
                }
            }
        }
    }

    if is_timeout {
        append_trace_log_line(&format!(
            "[ERROR] timeout: Model API call timed out after {}s (model={})",
            timeout_secs, effective_req.model
        ));
        anyhow::bail!("Model API timeout after {}s: {}", timeout_secs, last_error);
    }

    anyhow::bail!("Model API error after 3 attempts: {}", last_error)
}

pub(crate) async fn chat_once(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse> {
    chat_once_base(client, chat_url, req, None).await
}

pub(crate) async fn chat_once_with_timeout(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<ChatCompletionResponse> {
    chat_once_base(client, chat_url, req, Some(timeout_s)).await
}

pub(crate) async fn chat_json_with_repair<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<T> {
    let resp = chat_once(client, chat_url, req).await?;
    let text = extract_response_text(&resp);
    parse_json_loose(&text)
}

pub(crate) async fn chat_json_with_repair_timeout<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<T> {
    let resp = chat_once_with_timeout(client, chat_url, req, timeout_s).await?;
    let text = extract_response_text(&resp);
    parse_json_loose(&text)
}

pub(crate) async fn chat_json_with_repair_text<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<(T, String)> {
    let resp = chat_once(client, chat_url, req).await?;
    let text = extract_response_text(&resp);
    let parsed: T = parse_json_loose(&text)?;
    Ok((parsed, text))
}

pub(crate) async fn chat_json_with_repair_text_timeout<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<(T, String)> {
    let resp = chat_once_with_timeout(client, chat_url, req, timeout_s).await?;
    let text = extract_response_text(&resp);
    let parsed: T = parse_json_loose(&text)?;
    Ok((parsed, text))
}

fn effective_reasoning_format(req: &ChatCompletionRequest) -> Option<String> {
    let requested = req.reasoning_format.as_deref()?.trim();
    if requested.is_empty() {
        return None;
    }
    if !requested.eq_ignore_ascii_case("none") {
        return Some(requested.to_string());
    }
    if req.max_tokens <= 16 {
        return Some("none".to_string());
    }
    if request_expects_json(req) {
        return Some("none".to_string());
    }
    let profile = current_model_behavior_profile();
    let Some(profile) = profile else {
        return Some("none".to_string());
    };
    if profile.preferred_reasoning_format.eq_ignore_ascii_case("auto")
        && profile.auto_reasoning_separated
    {
        return Some("auto".to_string());
    }
    Some("none".to_string())
}

fn request_expects_json(req: &ChatCompletionRequest) -> bool {
    let system_prompt = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.to_ascii_lowercase())
        .unwrap_or_default();
    system_prompt.contains("json")
        || system_prompt.contains("schema:")
        || system_prompt.contains("output only")
}

fn isolate_reasoning_fields(resp: &mut ChatCompletionResponse) {
    for choice in &mut resp.choices {
        if let Some(content) = choice.message.content.as_mut() {
            let (plain, thinking) = split_llama_sentinel_reasoning(content);
            choice.message.content = Some(plain);
            if choice.message.reasoning_content.is_none() && thinking.is_some() {
                choice.message.reasoning_content = thinking;
            }
        }
    }
}
