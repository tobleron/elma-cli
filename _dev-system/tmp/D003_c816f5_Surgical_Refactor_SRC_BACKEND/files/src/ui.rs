use crate::*;

pub(crate) fn prompt_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush().ok();

    let mut line = String::new();
    let n = io::stdin().read_line(&mut line)?;
    if n == 0 {
        return Ok(None); // EOF
    }
    let line = line.trim_end_matches(['\n', '\r']).to_string();
    Ok(Some(line))
}

pub(crate) fn user_prompt_label(args: &Args) -> String {
    if args.no_color {
        "◉ ".to_string()
    } else {
        ansi_pale_yellow("◉ ")
    }
}

pub(crate) fn ansi_grey(s: &str) -> String {
    // 8-bit grey
    format!("\x1b[90m{s}\x1b[0m")
}

pub(crate) fn ansi_dim_gray(s: &str) -> String {
    // Dim grey for process steps
    format!("\x1b[2;90m{s}\x1b[0m")
}

pub(crate) fn ansi_orange(s: &str) -> String {
    // Truecolor #de218e.
    format!("\x1b[38;2;222;33;142m{s}\x1b[0m")
}

pub(crate) fn ansi_pale_yellow(s: &str) -> String {
    // 256-color pale yellow.
    format!("\x1b[38;5;229m{s}\x1b[0m")
}

pub(crate) fn ansi_paler_yellow(s: &str) -> String {
    // Pale dark golden (less bright than 229, less grey than 187).
    format!("\x1b[38;5;179m{s}\x1b[0m")
}

pub(crate) fn ansi_soft_gold(s: &str) -> String {
    // Slightly paler than 179 while staying warm and clearly golden.
    format!("\x1b[38;5;180m{s}\x1b[0m")
}

pub(crate) fn ansi_soft_green(s: &str) -> String {
    // Pale green that stays readable beside the golden working trace.
    format!("\x1b[38;5;114m{s}\x1b[0m")
}

static TRACE_LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static REASONING_DISPLAY: OnceLock<Mutex<(bool, bool)>> = OnceLock::new();
static JSON_OUTPUTTER_PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static FINAL_ANSWER_EXTRACTOR_PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static MODEL_BEHAVIOR_PROFILE: OnceLock<Mutex<Option<ModelBehaviorProfile>>> = OnceLock::new();

pub(crate) fn trace_log_state() -> &'static Mutex<Option<PathBuf>> {
    TRACE_LOG_PATH.get_or_init(|| Mutex::new(None))
}

pub(crate) fn reasoning_display_state() -> &'static Mutex<(bool, bool)> {
    REASONING_DISPLAY.get_or_init(|| Mutex::new((false, false)))
}

pub(crate) fn json_outputter_state() -> &'static Mutex<Option<Profile>> {
    JSON_OUTPUTTER_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn final_answer_extractor_state() -> &'static Mutex<Option<Profile>> {
    FINAL_ANSWER_EXTRACTOR_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn model_behavior_state() -> &'static Mutex<Option<ModelBehaviorProfile>> {
    MODEL_BEHAVIOR_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_trace_log_path(path: Option<PathBuf>) {
    if let Ok(mut slot) = trace_log_state().lock() {
        *slot = path;
    }
}

pub(crate) fn set_reasoning_display(show_terminal: bool, no_color: bool) {
    if let Ok(mut slot) = reasoning_display_state().lock() {
        *slot = (show_terminal, no_color);
    }
}

pub(crate) fn set_json_outputter_profile(profile: Option<Profile>) {
    if let Ok(mut slot) = json_outputter_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn set_final_answer_extractor_profile(profile: Option<Profile>) {
    if let Ok(mut slot) = final_answer_extractor_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn set_model_behavior_profile(profile: Option<ModelBehaviorProfile>) {
    if let Ok(mut slot) = model_behavior_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn append_trace_log_line(line: &str) {
    let path = trace_log_state()
        .lock()
        .ok()
        .and_then(|slot| (*slot).clone());
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

pub(crate) fn append_reasoning_audit_record(
    req: &ChatCompletionRequest,
    resp: &ChatCompletionResponse,
) {
    let path = trace_log_state()
        .lock()
        .ok()
        .and_then(|slot| (*slot).clone())
        .map(|trace_path| trace_path.with_file_name("reasoning_audit.jsonl"));
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    let system_preview = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| preview_text(&m.content, 2))
        .unwrap_or_default();
    let user_preview = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| preview_text(&m.content, 4))
        .unwrap_or_default();
    let final_text = extract_response_text(resp);
    let reasoning_text = extract_response_reasoning(resp);
    let record = serde_json::json!({
        "ts_unix_s": SystemTime::now().duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()).unwrap_or_default(),
        "model": req.model,
        "reasoning_format": req.reasoning_format,
        "system_preview": system_preview,
        "user_preview": user_preview,
        "final_text": final_text,
        "reasoning_text": reasoning_text,
        "has_reasoning": !reasoning_text.trim().is_empty(),
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{record}");
    }
}

pub(crate) fn maybe_display_reasoning_trace(resp: &ChatCompletionResponse) {
    let (show_terminal, no_color) = reasoning_display_state()
        .lock()
        .ok()
        .map(|slot| *slot)
        .unwrap_or((false, false));
    if !show_terminal {
        return;
    }
    let reasoning = extract_response_reasoning(resp);
    if reasoning.trim().is_empty() {
        return;
    }
    for line in preview_text(&reasoning, 6).lines() {
        let rendered = format!("~ {}", line.trim_end());
        if no_color {
            eprintln!("{rendered}");
        } else {
            eprintln!("{}", ansi_paler_yellow(&rendered));
        }
    }
}

pub(crate) fn trace(args: &Args, msg: &str) {
    let line = format!("trace: {msg}");
    append_trace_log_line(&line);
    // Traces go to log file only, not UI (for debugging)
}

pub(crate) fn trace_verbose(verbose: bool, msg: &str) {
    let line = format!("trace: {msg}");
    append_trace_log_line(&line);
    // Traces go to log file only, not UI (for debugging)
}

/// Show a concise process milestone (shown when verbose is enabled)
pub(crate) fn show_process_step_verbose(verbose: bool, category: &str, msg: &str) {
    let line = format!("[{}] {}", category, msg);
    append_trace_log_line(&line);
    if verbose {
        eprintln!("{}", ansi_dim_gray(&line));
    }
}

pub(crate) fn show_process_step(args: &Args, category: &str, msg: &str) {
    show_process_step_verbose(args.show_process, category, msg);
}

/// Show intel summary from model-generated feedback (Category 2)
/// These are situational summaries determined by the model
pub(crate) fn show_intel_summary(verbose: bool, summary: &str) {
    let line = format!("💡 {}", summary);
    append_trace_log_line(&line);
    if verbose {
        eprintln!("{}", ansi_soft_gold(&line));
    }
}

pub(crate) fn calibration_progress(args: &Args, msg: &str) {
    if args.tune || args.calibrate {
        let line = format!("tune> {msg}");
        append_trace_log_line(&line);
        eprintln!("{line}");
        let _ = io::stderr().flush();
    }
}

pub(crate) fn operator_trace(args: &Args, msg: &str) {
    let line = format!("-> {msg}");
    append_trace_log_line(&line);
    if !(args.tune || args.calibrate) || args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", ansi_soft_gold(&line));
        }
    }
}

pub(crate) fn shell_command_trace(args: &Args, cmd: &str) {
    let compact = cmd.replace('\n', " ");
    let line = format!("-> {compact}");
    append_trace_log_line(&line);
    if !(args.tune || args.calibrate) || args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", ansi_soft_green(&line));
        }
    }
}

pub(crate) fn print_elma_message(args: &Args, text: &str) {
    println!(
        "{}",
        if args.no_color {
            format!("Elma: {text}")
        } else {
            ansi_orange(&format!("Elma: {text}"))
        }
    );
}

/// Strip <think>...</think> blocks. If an opening tag is found without a closing tag,
/// drop the rest to avoid leaking partial reasoning.
pub(crate) fn strip_think_tags(s: &str) -> String {
    // Use the centralized thinking_content module
    thinking_content::strip_think_tags(s)
}

pub(crate) fn describe_operator_intent(
    route: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> String {
    if route
        .speech_act
        .choice
        .eq_ignore_ascii_case("CAPABILITY_CHECK")
        && probability_of(&route.speech_act.distribution, "CAPABILITY_CHECK") >= 0.65
    {
        return "answering a capability question".to_string();
    }
    let pattern = if !formula.primary.trim().is_empty() {
        formula.primary.trim()
    } else if !complexity.suggested_pattern.trim().is_empty() {
        complexity.suggested_pattern.trim()
    } else {
        ""
    };
    match pattern {
        "inspect_decide_reply" => {
            "checking workspace evidence before making a decision".to_string()
        }
        "inspect_summarize_reply" => "inspecting workspace evidence and summarizing it".to_string(),
        "inspect_edit_verify_reply" => "editing files and verifying the result".to_string(),
        "inspect_reply" => "looking at workspace evidence before answering".to_string(),
        "execute_reply" => "running a terminal action and preparing the answer".to_string(),
        "plan_reply" => "building a concrete plan".to_string(),
        "masterplan_reply" => "building an overall plan".to_string(),
        "reply" | "reply_only" => "answering directly".to_string(),
        _ => match route.route.as_str() {
            "SHELL" => "working through the workspace".to_string(),
            "PLAN" => "building a concrete plan".to_string(),
            "MASTERPLAN" => "building an overall plan".to_string(),
            "DECIDE" => "weighing the options".to_string(),
            _ => "answering directly".to_string(),
        },
    }
}

pub(crate) fn squash_blank_lines(s: &str) -> String {
    // For chat display: keep newlines, but remove empty-line breaks that look like "two messages".
    let mut out = String::with_capacity(s.len());
    let mut last_was_nl = false;
    let mut nl_run = 0u32;
    for ch in s.chars() {
        if ch == '\n' {
            nl_run += 1;
            if nl_run <= 1 {
                out.push('\n');
            } else {
                // drop extra newlines
            }
            last_was_nl = true;
        } else {
            nl_run = 0;
            if last_was_nl && ch == '\r' {
                continue;
            }
            out.push(ch);
            last_was_nl = false;
        }
    }
    out.trim().to_string()
}

const LLAMA_REASONING_START: &str = "<<<reasoning_content_start>>>";
const LLAMA_REASONING_END: &str = "<<<reasoning_content_end>>>";

pub(crate) fn split_llama_sentinel_reasoning(content: &str) -> (String, Option<String>) {
    // Use the centralized thinking_content module
    let (plain, thinking) = thinking_content::extract_llama_sentinel_reasoning(content);
    (plain, thinking)
}

/// Extract (thinking, final) from either structured fields or tagged output.
///
/// Mirrors the Open WebUI "compatible provider" strategy:
/// - Prefer `content` if non-empty, else fall back to `reasoning_content`.
/// - Strip thinking blocks from the final user-visible text.
/// - If tags are present, also return extracted thinking for display/logging.
pub(crate) fn split_thinking_and_final(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> (Option<String>, String) {
    // Use the centralized thinking_content module
    let extraction = thinking_content::extract_thinking(content, reasoning_content);
    (extraction.thinking, extraction.final_answer)
}

pub(crate) fn extract_final_line(text: &str, prefix: &str) -> Option<String> {
    let p = prefix.trim();
    if p.is_empty() {
        return None;
    }
    // Find the last line that begins with the prefix (case-sensitive).
    let mut last: Option<String> = None;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with(p) {
            let rest = l[p.len()..].trim();
            last = Some(rest.to_string());
        }
    }
    last.filter(|s| !s.trim().is_empty())
}

pub(crate) fn remove_final_lines(text: &str, prefix: &str) -> String {
    let p = prefix.trim();
    if p.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with(p) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
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

fn current_model_behavior_profile() -> Option<ModelBehaviorProfile> {
    model_behavior_state()
        .lock()
        .ok()
        .and_then(|slot| (*slot).clone())
}

fn final_answer_extractor_profile() -> Option<Profile> {
    final_answer_extractor_state()
        .lock()
        .ok()
        .and_then(|slot| (*slot).clone())
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

#[derive(Debug, Deserialize)]
struct FinalAnswerEnvelope {
    #[serde(rename = "final")]
    final_text: String,
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
    let choice = resp
        .choices
        .get(0)
        .context("No choices available for final-answer extraction")?;
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "original_system_prompt": original_system_prompt,
                    "original_user_input": original_user_input,
                    "assistant_draft": choice.message.content.clone().unwrap_or_default(),
                    "assistant_reasoning": choice.message.reasoning_content.clone().unwrap_or_default(),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let envelope: FinalAnswerEnvelope = chat_json_with_repair(client, chat_url, &req).await?;
    Ok(envelope.final_text.trim().to_string())
}

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
    if let Some(previous) = maybe_cap_auto_reasoning_tokens(&mut effective_req) {
        append_trace_log_line(&format!(
            "trace: reasoning_token_cap applied previous={} effective={} model={}",
            previous, effective_req.max_tokens, effective_req.model
        ));
    }

    // Use provided timeout or default to 120s
    let timeout_secs = timeout_s.unwrap_or(120);
    let mut last_error = String::new();
    let mut is_timeout = false;

    for attempt in 0..3u32 {
        // Apply timeout to this specific request
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
                // Check if this is a timeout error
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

    // Log timeout specifically for better diagnostics
    if is_timeout {
        append_trace_log_line(&format!(
            "[ERROR] timeout: Model API call timed out after {}s (model={})",
            timeout_secs, effective_req.model
        ));
        anyhow::bail!("Model API timeout after {}s: {}", timeout_secs, last_error);
    }

    anyhow::bail!("POST /v1/chat/completions failed after retries: {last_error}")
}

pub(crate) async fn chat_once(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse> {
    // Default timeout - callers should use chat_once_with_timeout for custom timeouts
    let mut parsed = chat_once_base(client, chat_url, req, None).await?;
    
    // Isolate reasoning fields using centralized thinking extraction
    isolate_reasoning_fields(&mut parsed);

    let mut effective_req = req.clone();
    effective_req.reasoning_format = effective_reasoning_format(req);
    let _ = maybe_cap_auto_reasoning_tokens(&mut effective_req);
    if response_needs_text_finalizer(&effective_req, &parsed) {
        match finalize_text_response_once(client, chat_url, &effective_req, &parsed).await {
            Ok(final_text) if !final_text.is_empty() => {
                if let Some(choice) = parsed.choices.get_mut(0) {
                    choice.message.content = Some(final_text);
                }
                append_trace_log_line(&format!(
                    "trace: text_finalizer_applied model={}",
                    effective_req.model
                ));
            }
            Ok(_) => {
                append_trace_log_line(&format!(
                    "trace: text_finalizer_empty model={}",
                    effective_req.model
                ));
            }
            Err(error) => {
                append_trace_log_line(&format!(
                    "trace: text_finalizer_failed model={} error={:#}",
                    effective_req.model, error
                ));
            }
        }
    }
    Ok(parsed)
}

/// Chat with explicit timeout configuration.
/// Use this when you need to respect Profile-specific timeout settings.
pub(crate) async fn chat_once_with_timeout(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<ChatCompletionResponse> {
    let mut parsed = chat_once_base(client, chat_url, req, Some(timeout_s)).await?;
    
    // Isolate reasoning fields using centralized thinking extraction
    isolate_reasoning_fields(&mut parsed);

    let mut effective_req = req.clone();
    effective_req.reasoning_format = effective_reasoning_format(req);
    let _ = maybe_cap_auto_reasoning_tokens(&mut effective_req);
    if response_needs_text_finalizer(&effective_req, &parsed) {
        match finalize_text_response_once(client, chat_url, &effective_req, &parsed).await {
            Ok(final_text) if !final_text.is_empty() => {
                if let Some(choice) = parsed.choices.get_mut(0) {
                    choice.message.content = Some(final_text);
                }
                append_trace_log_line(&format!(
                    "trace: text_finalizer_applied model={}",
                    effective_req.model
                ));
            }
            Ok(_) => {
                append_trace_log_line(&format!(
                    "trace: text_finalizer_empty model={}",
                    effective_req.model
                ));
            }
            Err(error) => {
                append_trace_log_line(&format!(
                    "trace: text_finalizer_failed model={} error={:#}",
                    effective_req.model, error
                ));
            }
        }
    }
    Ok(parsed)
}

pub(crate) fn extract_response_text(resp: &ChatCompletionResponse) -> String {
    // Only return the content field (final answer), not reasoning_content
    resp.choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default()
}

pub(crate) fn extract_response_reasoning(resp: &ChatCompletionResponse) -> String {
    resp.choices
        .get(0)
        .and_then(|c| c.message.reasoning_content.clone())
        .unwrap_or_default()
}

fn json_outputter_profile() -> Option<Profile> {
    json_outputter_state()
        .lock()
        .ok()
        .and_then(|slot| (*slot).clone())
}

fn structured_output_context(req: &ChatCompletionRequest) -> (String, String) {
    let target_system_prompt = req
        .messages
        .iter()
        .find(|m| m.role == "system")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let target_user_input = req
        .messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    (target_system_prompt, target_user_input)
}

fn canonical_json_text(text: &str) -> String {
    extract_first_json_object(text)
        .unwrap_or(text)
        .trim()
        .to_string()
}

async fn compile_json_once(
    client: &reqwest::Client,
    chat_url: &Url,
    target_req: &ChatCompletionRequest,
    raw_draft: &str,
    parser_error: Option<&str>,
    timeout_s: Option<u64>,
) -> Result<String> {
    let Some(cfg) = json_outputter_profile() else {
        return Ok(raw_draft.trim().to_string());
    };
    let (target_system_prompt, target_user_input) = structured_output_context(target_req);
    let compile_req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "target_system_prompt": target_system_prompt,
                    "target_user_input": target_user_input,
                    "raw_model_draft": raw_draft,
                    "parser_error": parser_error.unwrap_or(""),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let compiled = chat_once_base(client, chat_url, &compile_req, timeout_s).await?;
    Ok(extract_response_text(&compiled).trim().to_string())
}

async fn legacy_repair_json_text(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    invalid_text: &str,
    timeout_s: Option<u64>,
) -> Result<String> {
    let repair_context = req
        .messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_default();
    let mut repair_req = req.clone();
    repair_req.messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: req
                .messages
                .iter()
                .find(|m| m.role == "system")
                .map(|m| m.content.clone())
                .unwrap_or_default(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: format!(
                "Return ONLY one valid JSON object that matches your required schema.\n\nOriginal input:\n{}\n\nYour previous invalid output:\n{}",
                repair_context.trim(),
                invalid_text.trim()
            ),
        },
    ];
    let repaired = chat_once_base(client, chat_url, &repair_req, timeout_s).await?;
    Ok(extract_response_text(&repaired).trim().to_string())
}

async fn chat_json_text_with_repair(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: Option<u64>,
) -> Result<String> {
    let resp = chat_once_base(client, chat_url, req, timeout_s).await?;
    let draft = extract_response_text(&resp);
    let first_pass = compile_json_once(client, chat_url, req, &draft, None, timeout_s).await?;
    if parse_json_loose::<serde_json::Value>(&first_pass).is_ok() {
        return Ok(canonical_json_text(&first_pass));
    }

    let parse_error = parse_json_loose::<serde_json::Value>(&first_pass)
        .err()
        .map(|e| format!("{e:#}"))
        .unwrap_or_else(|| "Unknown JSON parse error".to_string());

    if json_outputter_profile().is_some() {
        let repaired = compile_json_once(client, chat_url, req, &first_pass, Some(&parse_error), timeout_s).await?;
        return Ok(canonical_json_text(&repaired));
    }

    let repaired = legacy_repair_json_text(client, chat_url, req, &first_pass, timeout_s).await?;
    Ok(canonical_json_text(&repaired))
}

pub(crate) async fn chat_json_with_repair<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<T> {
    // Default timeout - use chat_json_with_repair_timeout for custom timeouts
    let text = chat_json_text_with_repair(client, chat_url, req, None).await?;
    parse_json_loose(&text)
}

/// Chat JSON with repair and explicit timeout.
/// Use this when you need to respect Profile-specific timeout settings.
pub(crate) async fn chat_json_with_repair_timeout<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<T> {
    let text = chat_json_text_with_repair(client, chat_url, req, Some(timeout_s)).await?;
    parse_json_loose(&text)
}

pub(crate) async fn chat_json_with_repair_text<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<(T, String)> {
    // Default timeout - use chat_json_with_repair_text_timeout for custom timeouts
    let text = chat_json_text_with_repair(client, chat_url, req, None).await?;
    let parsed = parse_json_loose(&text)?;
    Ok((parsed, text))
}

/// Chat JSON with repair, text output, and explicit timeout.
pub(crate) async fn chat_json_with_repair_text_timeout<T: DeserializeOwned>(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
    timeout_s: u64,
) -> Result<(T, String)> {
    let text = chat_json_text_with_repair(client, chat_url, req, Some(timeout_s)).await?;
    let parsed = parse_json_loose(&text)?;
    Ok((parsed, text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_separated_reasoning_when_content_is_empty() {
        let (thinking, final_text) = split_thinking_and_final(Some(""), Some("hidden reasoning"));
        assert_eq!(thinking.as_deref(), Some("hidden reasoning"));
        assert!(final_text.is_empty());
    }
}
