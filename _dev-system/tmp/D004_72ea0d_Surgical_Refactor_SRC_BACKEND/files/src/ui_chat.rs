//! @efficiency-role: service-orchestrator
//!
//! UI - Chat Functions

use crate::*;
use std::sync::OnceLock;

// ============================================================================
// Config Root for Grammar Loading
// ============================================================================

/// Global config root for grammar loading
/// Set during bootstrap, used by grammar injection
static CONFIG_ROOT: OnceLock<PathBuf> = OnceLock::new();

/// Set the global config root (called during bootstrap)
pub(crate) fn set_config_root(config_root: PathBuf) {
    let _ = CONFIG_ROOT.set(config_root);
}

/// Get the global config root
pub(crate) fn get_config_root_for_intel() -> Option<&'static PathBuf> {
    CONFIG_ROOT.get()
}

// ============================================================================
// Grammar Injection Helper
// ============================================================================

/// Inject grammar into request if profile has grammar mapping
///
/// Returns true if grammar was injected, false otherwise.
fn inject_grammar_if_configured(request: &mut ChatCompletionRequest, profile_name: &str) -> bool {
    let Some(config_root) = get_config_root_for_intel() else {
        return false;
    };

    match crate::json_grammar::inject_grammar_for_profile(request, profile_name, config_root) {
        Ok(injected) => {
            if injected {
                append_trace_log_line(&format!(
                    "[GRAMMAR] injected grammar for profile={}",
                    profile_name
                ));
            }
            injected
        }
        Err(e) => {
            append_trace_log_line(&format!(
                "[GRAMMAR] failed to inject grammar for profile={}: {}",
                profile_name, e
            ));
            false
        }
    }
}

async fn chat_once_base(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: Option<u64>,
    profile_name: Option<&str>,
) -> Result<ChatCompletionResponse> {
    let mut effective_req = req.clone();

    // Inject grammar if profile has grammar mapping
    if let Some(profile_name) = profile_name {
        inject_grammar_if_configured(&mut effective_req, profile_name);
    }

    let original_reasoning = req.reasoning_format.clone();
    effective_req.reasoning_format = effective_reasoning_format(req);

    // VERBOSE LOGGING for troubleshooting
    append_trace_log_line(&format!(
        "[HTTP_START] model={} url={} timeout={:?}s",
        effective_req.model, chat_url, timeout_s
    ));

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
        append_trace_log_line(&format!("[HTTP_ATTEMPT] attempt={}/3", attempt + 1));

        let request_builder = client
            .post(chat_url.clone())
            .json(&effective_req)
            .timeout(Duration::from_secs(timeout_secs));

        append_trace_log_line(&format!("[HTTP_SEND] sending POST request..."));

        // Add explicit timeout wrapper for debugging
        let send_future = request_builder.send();
        let resp_result =
            tokio::time::timeout(Duration::from_secs(timeout_secs + 10), send_future).await;

        match resp_result {
            Ok(Ok(resp)) => {
                append_trace_log_line(&format!("[HTTP_RESPONSE] status={}", resp.status()));
                let status = resp.status();
                let text = resp.text().await.context("Failed to read response body")?;
                append_trace_log_line(&format!("[HTTP_BODY] received {} bytes", text.len()));

                if !status.is_success() {
                    if status.is_server_error() && attempt < 2 {
                        append_trace_log_line(&format!("[HTTP_RETRY] server error, retrying..."));
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
                append_trace_log_line(&format!("[HTTP_SUCCESS] parsed response successfully"));
                return Ok(parsed);
            }
            Ok(Err(e)) => {
                // HTTP request failed
                append_trace_log_line(&format!("[HTTP_ERROR] {}", e));
                if e.is_timeout() || e.to_string().contains("timeout") {
                    is_timeout = true;
                    last_error = format!(
                        "Model API timeout after {}s (attempt {}/{})",
                        timeout_secs,
                        attempt + 1,
                        3
                    );
                } else {
                    last_error = format!("{e:#}");
                }

                if attempt < 2 {
                    append_trace_log_line(&format!("[HTTP_RETRY] sleeping before retry..."));
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                    continue;
                }
            }
            Err(_elapsed) => {
                // tokio timeout elapsed
                append_trace_log_line(&format!(
                    "[HTTP_TIMEOUT] tokio timeout after {}s",
                    timeout_secs + 10
                ));
                is_timeout = true;
                last_error = format!(
                    "Model API tokio timeout after {}s (attempt {}/{})",
                    timeout_secs + 10,
                    attempt + 1,
                    3
                );

                if attempt < 2 {
                    append_trace_log_line(&format!("[HTTP_RETRY] sleeping before retry..."));
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
    chat_once_base(client, chat_url, req, None, None).await
}

pub(crate) async fn chat_once_with_timeout(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<ChatCompletionResponse> {
    chat_once_base(client, chat_url, req, Some(timeout_s), None).await
}

/// Chat once with grammar injection for a specific profile
pub(crate) async fn chat_once_with_grammar(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
) -> Result<ChatCompletionResponse> {
    chat_once_base(client, chat_url, req, None, Some(profile_name)).await
}

/// Chat once with grammar injection and timeout
pub(crate) async fn chat_once_with_grammar_timeout(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
    timeout_s: u64,
) -> Result<ChatCompletionResponse> {
    chat_once_base(client, chat_url, req, Some(timeout_s), Some(profile_name)).await
}

pub(crate) async fn chat_json_with_repair<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<T> {
    let resp = chat_once(client, chat_url, req).await?;
    let text = extract_response_text(&resp);
    parse_json_response(client, chat_url, req, &text).await
}

pub(crate) async fn chat_json_with_repair_for_profile<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
) -> Result<T> {
    let resp = chat_once_with_grammar(client, chat_url, req, profile_name).await?;
    let text = extract_response_text(&resp);
    parse_json_response(client, chat_url, req, &text).await
}

pub(crate) async fn chat_json_with_repair_timeout<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<T> {
    let resp = chat_once_with_timeout(client, chat_url, req, timeout_s).await?;
    let text = extract_response_text(&resp);
    parse_json_response(client, chat_url, req, &text).await
}

pub(crate) async fn chat_json_with_repair_for_profile_timeout<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
    timeout_s: u64,
) -> Result<T> {
    let resp =
        chat_once_with_grammar_timeout(client, chat_url, req, profile_name, timeout_s).await?;
    let text = extract_response_text(&resp);
    parse_json_response(client, chat_url, req, &text).await
}

pub(crate) async fn chat_json_with_repair_text<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<(T, String)> {
    let resp = chat_once(client, chat_url, req).await?;
    let text = extract_response_text(&resp);
    let (parsed, repaired): (T, String) = parse_json_response_with_repaired(client, chat_url, req, &text).await?;
    Ok((parsed, repaired))
}

pub(crate) async fn chat_json_with_repair_text_for_profile<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
) -> Result<(T, String)> {
    let resp = chat_once_with_grammar(client, chat_url, req, profile_name).await?;
    let text = extract_response_text(&resp);
    let (parsed, repaired): (T, String) = parse_json_response_with_repaired(client, chat_url, req, &text).await?;
    Ok((parsed, repaired))
}

pub(crate) async fn chat_json_with_repair_text_timeout<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<(T, String)> {
    let resp = chat_once_with_timeout(client, chat_url, req, timeout_s).await?;
    let text = extract_response_text(&resp);
    let (parsed, repaired): (T, String) = parse_json_response_with_repaired(client, chat_url, req, &text).await?;
    Ok((parsed, repaired))
}

pub(crate) async fn chat_json_with_repair_text_for_profile_timeout<
    T: DeserializeOwned + 'static,
>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
    timeout_s: u64,
) -> Result<(T, String)> {
    let resp =
        chat_once_with_grammar_timeout(client, chat_url, req, profile_name, timeout_s).await?;
    let text = extract_response_text(&resp);
    let (parsed, repaired): (T, String) = parse_json_response_with_repaired(client, chat_url, req, &text).await?;
    Ok((parsed, repaired))
}

async fn parse_json_response<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    text: &str,
) -> Result<T> {
    let (parsed, _) = parse_json_response_with_repaired::<T>(client, chat_url, req, text).await?;
    Ok(parsed)
}

async fn parse_json_response_with_repaired<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    text: &str,
) -> Result<(T, String)> {
    match parse_json_loose(text) {
        Ok(parsed) => Ok((parsed, text.to_string())),
        Err(parse_error) => {
            let repair_profile =
                default_json_repair_config(&base_url_from_chat_url(chat_url), &req.model);
            let repair_unit = JsonRepairUnit::new(repair_profile);
            let repaired = repair_unit
                .repair_with_fallback(
                    client,
                    chat_url,
                    text,
                    &[format!("Parse failure: {}", parse_error)],
                )
                .await?;
            let parsed =
                parse_json_loose(&repaired).context("JSON parsing failed after model-based repair")?;
            Ok((parsed, repaired))
        }
    }
}

fn base_url_from_chat_url(chat_url: &Url) -> String {
    let mut base = format!(
        "{}://{}",
        chat_url.scheme(),
        chat_url.host_str().unwrap_or("localhost")
    );
    if let Some(port) = chat_url.port() {
        base.push(':');
        base.push_str(&port.to_string());
    }
    base
}

pub(crate) async fn chat_json_text_with_repair(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<String> {
    let resp = chat_once(client, chat_url, req).await?;
    Ok(extract_response_text(&resp))
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
    if profile
        .preferred_reasoning_format
        .eq_ignore_ascii_case("auto")
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

fn maybe_cap_auto_reasoning_tokens(req: &mut ChatCompletionRequest) -> Option<u32> {
    let profile = current_model_behavior_profile()?;
    if !profile.needs_text_finalizer {
        return None;
    }
    if request_expects_json(req) {
        return None;
    }
    if !req
        .reasoning_format
        .as_deref()
        .unwrap_or("none")
        .eq_ignore_ascii_case("auto")
    {
        return None;
    }
    if req.max_tokens <= 256 {
        return None;
    }
    let previous = req.max_tokens;
    req.max_tokens = 256;
    Some(previous)
}

fn response_needs_text_finalizer(
    req: &ChatCompletionRequest,
    resp: &ChatCompletionResponse,
) -> bool {
    let profile = match current_model_behavior_profile() {
        Some(p) => p,
        None => return false,
    };
    if !profile.needs_text_finalizer || request_expects_json(req) {
        return false;
    }
    let Some(choice) = resp.choices.get(0) else {
        return false;
    };
    let content = choice.message.content.as_deref().unwrap_or("").trim();
    let reasoning = choice
        .message
        .reasoning_content
        .as_deref()
        .unwrap_or("")
        .trim();
    content.is_empty() && !reasoning.is_empty()
}

#[derive(Debug, Deserialize)]
struct FinalAnswerEnvelope {
    #[serde(rename = "final")]
    final_text: String,
}

async fn finalize_text_response_once(
    client: &reqwest::Client,
    chat_url: &Url,
    original_req: &ChatCompletionRequest,
    resp: &ChatCompletionResponse,
) -> Result<String> {
    let Some(cfg) = final_answer_extractor_profile() else {
        anyhow::bail!("No final-answer extractor profile loaded");
    };
    let original_system_prompt = original_req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let original_user_input = original_req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let choice = resp.choices.get(0).context("No choices available")?;
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage { role: "system".to_string(), content: cfg.system_prompt.clone() },
            ChatMessage { role: "user".to_string(), content: serde_json::json!({
                "original_system_prompt": original_system_prompt,
                "original_user_input": original_user_input,
                "assistant_draft": choice.message.content.clone().unwrap_or_default(),
                "assistant_reasoning": choice.message.reasoning_content.clone().unwrap_or_default(),
            }).to_string() },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };
    let envelope: FinalAnswerEnvelope = chat_json_with_repair(client, chat_url, &req).await?;
    Ok(envelope.final_text.trim().to_string())
}

fn canonical_json_text(text: &str) -> String {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

async fn compile_json_once(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<String> {
    let resp = chat_once(client, chat_url, req).await?;
    Ok(extract_response_text(&resp))
}

async fn legacy_repair_json_text(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<String> {
    let resp = chat_once(client, chat_url, req).await?;
    let text = extract_response_text(&resp);
    if let Some(json) = crate::routing::extract_first_json_object(&text) {
        Ok(json.to_string())
    } else {
        Ok(text)
    }
}

fn structured_output_context(req: &ChatCompletionRequest) -> (String, String) {
    let system = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let user = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    (system, user)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_separated_reasoning_when_content_is_empty() {
        // Test that reasoning content is extracted correctly from reasoning_content field
        let (thinking, final_answer) = split_thinking_and_final(None, Some("This is reasoning"));
        assert!(thinking.is_some());
        assert_eq!(thinking.unwrap(), "This is reasoning");
        assert_eq!(final_answer, "");
    }
}
