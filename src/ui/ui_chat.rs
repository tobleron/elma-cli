//! @efficiency-role: util-pure
//!
//! UI - Chat Functions

use crate::*;
use anyhow::Context;
use futures::stream::StreamExt;
use std::sync::OnceLock;

// ============================================================================
// Response Completeness Validation
// ============================================================================

/// Check if a response appears to be truncated (incomplete)
///
/// Returns true if the response appears to be cut off mid-sentence or abruptly.
fn is_response_truncated(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Check for incomplete JSON
    let json_start_count = trimmed.matches('{').count();
    let json_end_count = trimmed.matches('}').count();
    if json_start_count > json_end_count {
        return true;
    }

    // Check for incomplete code blocks
    let code_block_start = trimmed.matches("```").count();
    if code_block_start % 2 != 0 {
        return true;
    }

    false
}

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
            AttemptOutcome::Success(mut resp) => {
                // Check if response is truncated and retry if needed
                if attempt < 2 {
                    // Only retry on attempts 0 and 1
                    // If the model stopped naturally (finish_reason="stop"),
                    // the response is complete regardless of last character.
                    let finish_reason = resp
                        .choices
                        .first()
                        .and_then(|c| c.finish_reason.as_deref());
                    let truncated = if finish_reason == Some("stop") {
                        false
                    } else {
                        resp.choices
                            .first()
                            .and_then(|c| c.message.content.as_ref())
                            .map(|content| is_response_truncated(content))
                            .unwrap_or(false)
                    };

                    if truncated {
                        append_trace_log_line("[HTTP_RETRY] response appears truncated, retrying with increased max_tokens...");
                        // Retry with increased max_tokens
                        let mut retry_req = effective_req.clone();
                        retry_req.max_tokens = retry_req.max_tokens.saturating_mul(2).min(4096);
                        // Also try without grammar if it was injected
                        retry_req.grammar = None;

                        tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                        continue;
                    }
                }
                return Ok(resp);
            }
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
        return Err(crate::diagnostics::ElmaDiagnostic::ModelApiTimeout {
            timeout_secs,
            last_error,
        }
        .into());
    }

    Err(crate::diagnostics::ElmaDiagnostic::ModelApiError { last_error }.into())
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

// ── DSL output functions ──

/// Send a request and parse the response as compact DSL.
///
/// Unlike the removed JSON auto-repair pipeline, this does not invoke a separate
/// repair model. Malformed output is retried once with a compact deterministic
/// DSL repair observation, then returned as an error if still invalid.
pub(crate) async fn chat_dsl_with_repair(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<serde_json::Value> {
    chat_dsl_with_repair_impl(client, chat_url, req, None, None).await
}

pub(crate) async fn chat_dsl_with_repair_for_profile_timeout(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: &str,
    timeout_s: u64,
) -> Result<serde_json::Value> {
    chat_dsl_with_repair_impl(client, chat_url, req, Some(profile_name), Some(timeout_s)).await
}

async fn chat_dsl_with_repair_impl(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    profile_name: Option<&str>,
    timeout_s: Option<u64>,
) -> Result<serde_json::Value> {
    const MAX_REPAIR_RETRIES: usize = 1;
    let mut working_req = req.clone();
    let mut last_repair = None;

    for attempt in 0..=MAX_REPAIR_RETRIES {
        let text = fetch_chat_text(client, chat_url, &working_req, profile_name, timeout_s).await?;
        let candidates = crate::text_utils::structured_output_candidates(&text);
        let mut last_error = None;
        for candidate in &candidates {
            match crate::intel_units::parse_auto_dsl(candidate) {
                Ok(value) => return Ok(value),
                Err(dsl_err) => last_error = Some(dsl_err),
            }
        }

        let dsl_err = last_error.unwrap_or_else(|| {
            crate::dsl::DslError::empty(crate::dsl::ParseContext {
                dsl_variant: "intel",
                line: None,
            })
        });
        let preview = if dsl_err.debug_preview.is_empty() {
            "(empty or whitespace only)".to_string()
        } else {
            dsl_err.debug_preview.clone()
        };
        // Persist raw model output to trace artifacts for DSL failure analysis
        let text_preview: String = text.chars().take(500).collect();
        append_trace_log_line(&format!(
            "[INTEL_DSL_PREVIEW] attempt={} raw_preview=\"{}\"",
            attempt + 1,
            text_preview.replace('"', "'")
        ));
        let expected_format = crate::dsl::detect_expected_format(&text);
        let repair_msg = format!(
            "INVALID_DSL\ncode: {}\nerror: {}\nExpected: {}\nReturn exactly one DSL line matching the Expected format.",
            dsl_err.code, preview, expected_format,
        );
        last_repair = Some(repair_msg.clone());
        if attempt < MAX_REPAIR_RETRIES {
            append_trace_log_line(&format!(
                "[INTEL_DSL_REPAIR] attempt={} error={}",
                attempt + 1,
                dsl_err.code
            ));
            working_req
                .messages
                .push(ChatMessage::simple("user", &repair_msg));
        }
    }

    Err(anyhow::anyhow!(last_repair.unwrap_or_else(|| {
        "INVALID_DSL\ncode: INVALID_DSL\nerror: parser did not return a repair observation\nExpected: one DSL line: COMMAND key=value key2=\"val\"\nReturn exactly one DSL line matching the Expected format.".to_string()
    })))
}

fn effective_reasoning_format(req: &ChatCompletionRequest) -> Option<String> {
    let requested = req.reasoning_format.as_deref()?.trim();
    if requested.is_empty() {
        return None;
    }
    if !requested.eq_ignore_ascii_case("none") {
        return Some(requested.to_string());
    }
    // T206: callers that explicitly request "none" must not be silently
    // escalated to visible reasoning.
    Some("none".to_string())
}

// Intentionally no JSON-mode helpers: model-produced structured output is DSL.

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
