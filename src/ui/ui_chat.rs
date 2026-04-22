//! @efficiency-role: util-pure
//!
//! UI - Chat Functions

use crate::*;
use anyhow::Context;
use futures::stream::StreamExt;
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

/// Result of a single HTTP attempt
enum AttemptOutcome {
    Success(ChatCompletionResponse),
    RetryableError(String),
    FatalError(anyhow::Error),
}

async fn attempt_chat_request(
    client: &reqwest::Client,
    chat_url: &Url,
    effective_req: &ChatCompletionRequest,
    timeout_secs: u64,
) -> AttemptOutcome {
    let request_builder = client
        .post(chat_url.clone())
        .json(effective_req)
        .timeout(Duration::from_secs(timeout_secs));

    append_trace_log_line(&format!("[HTTP_SEND] sending POST request..."));

    let send_future = request_builder.send();
    let resp_result =
        tokio::time::timeout(Duration::from_secs(timeout_secs + 10), send_future).await;

    match resp_result {
        Ok(Ok(resp)) => {
            append_trace_log_line(&format!("[HTTP_RESPONSE] status={}", resp.status()));
            let status = resp.status();
            let text = match resp.text().await {
                Ok(t) => t,
                Err(e) => {
                    return AttemptOutcome::FatalError(
                        anyhow::Error::from(e).context("Failed to read response body"),
                    )
                }
            };
            append_trace_log_line(&format!("[HTTP_BODY] received {} bytes", text.len()));

            if !status.is_success() {
                if status.is_server_error() {
                    return AttemptOutcome::RetryableError(format!(
                        "Server returned HTTP {status}: {text}"
                    ));
                }
                return AttemptOutcome::FatalError(anyhow::anyhow!(
                    "Server returned HTTP {status}: {text}"
                ));
            }

            let mut parsed: ChatCompletionResponse = match serde_json::from_str(&text) {
                Ok(p) => p,
                Err(e) => {
                    return AttemptOutcome::FatalError(
                        anyhow::Error::from(e).context("Invalid JSON from server"),
                    )
                }
            };
            isolate_reasoning_fields(&mut parsed);
            append_reasoning_audit_record(effective_req, &parsed);
            maybe_display_reasoning_trace(&parsed);
            append_trace_log_line(&format!("[HTTP_SUCCESS] parsed response successfully"));
            AttemptOutcome::Success(parsed)
        }
        Ok(Err(e)) => {
            append_trace_log_line(&format!("[HTTP_ERROR] {}", e));
            let is_timeout = e.is_timeout() || e.to_string().contains("timeout");
            let msg = if is_timeout {
                format!("Model API timeout after {}s", timeout_secs)
            } else {
                format!("{e:#}")
            };
            AttemptOutcome::RetryableError(msg)
        }
        Err(_elapsed) => {
            append_trace_log_line(&format!(
                "[HTTP_TIMEOUT] tokio timeout after {}s",
                timeout_secs + 10
            ));
            AttemptOutcome::RetryableError(format!(
                "Model API tokio timeout after {}s",
                timeout_secs + 10
            ))
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

    if let Some(profile_name) = profile_name {
        inject_grammar_if_configured(&mut effective_req, profile_name);
    }

    let original_reasoning = req.reasoning_format.clone();
    effective_req.reasoning_format = effective_reasoning_format(req);

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

        match attempt_chat_request(client, chat_url, &effective_req, timeout_secs).await {
            AttemptOutcome::Success(resp) => return Ok(resp),
            AttemptOutcome::RetryableError(err) => {
                if err.contains("timeout") || err.contains("Tokio timeout") {
                    is_timeout = true;
                }
                last_error = format!("{} (attempt {}/{})", err, attempt + 1, 3);
                if attempt < 2 {
                    append_trace_log_line(&format!("[HTTP_RETRY] sleeping before retry..."));
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                    continue;
                }
            }
            AttemptOutcome::FatalError(e) => return Err(e),
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
    chat_json_with_repair_impl(client, chat_url, req, None, None).await
}

pub(crate) async fn chat_json_with_repair_for_profile<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
) -> Result<T> {
    chat_json_with_repair_impl(client, chat_url, req, Some(profile_name), None).await
}

pub(crate) async fn chat_json_with_repair_timeout<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<T> {
    chat_json_with_repair_impl(client, chat_url, req, None, Some(timeout_s)).await
}

pub(crate) async fn chat_json_with_repair_for_profile_timeout<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
    timeout_s: u64,
) -> Result<T> {
    chat_json_with_repair_impl(client, chat_url, req, Some(profile_name), Some(timeout_s)).await
}

pub(crate) async fn chat_json_with_repair_text<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<(T, String)> {
    chat_json_with_repair_text_impl(client, chat_url, req, None, None).await
}

pub(crate) async fn chat_json_with_repair_text_for_profile<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
) -> Result<(T, String)> {
    chat_json_with_repair_text_impl(client, chat_url, req, Some(profile_name), None).await
}

pub(crate) async fn chat_json_with_repair_text_timeout<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<(T, String)> {
    chat_json_with_repair_text_impl(client, chat_url, req, None, Some(timeout_s)).await
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
    chat_json_with_repair_text_impl(client, chat_url, req, Some(profile_name), Some(timeout_s))
        .await
}

async fn chat_json_with_repair_impl<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: Option<&str>,
    timeout_s: Option<u64>,
) -> Result<T> {
    let text = fetch_chat_text(client, chat_url, req, profile_name, timeout_s).await?;
    parse_json_response(client, chat_url, req, &text).await
}

async fn chat_json_with_repair_text_impl<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: Option<&str>,
    timeout_s: Option<u64>,
) -> Result<(T, String)> {
    let text = fetch_chat_text(client, chat_url, req, profile_name, timeout_s).await?;
    parse_json_response_with_repaired(client, chat_url, req, &text).await
}

async fn fetch_chat_text(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: Option<&str>,
    timeout_s: Option<u64>,
) -> Result<String> {
    let resp = match (profile_name, timeout_s) {
        (Some(profile), Some(timeout)) => {
            chat_once_with_grammar_timeout(client, chat_url, req, profile, timeout).await?
        }
        (Some(profile), None) => chat_once_with_grammar(client, chat_url, req, profile).await?,
        (None, Some(timeout)) => chat_once_with_timeout(client, chat_url, req, timeout).await?,
        (None, None) => chat_once(client, chat_url, req).await?,
    };
    Ok(extract_response_text(&resp))
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
            let parsed = parse_json_loose(&repaired)
                .context("JSON parsing failed after model-based repair")?;
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
    let Some(profile) = current_model_behavior_profile() else {
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

pub(crate) fn isolate_reasoning_fields(resp: &mut ChatCompletionResponse) {
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
