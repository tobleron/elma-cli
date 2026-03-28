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

pub(crate) fn ansi_grey(s: &str) -> String {
    // 8-bit grey
    format!("\x1b[90m{s}\x1b[0m")
}

pub(crate) fn ansi_orange(s: &str) -> String {
    // 256-color "orange-ish" (208). Falls back to default if terminal doesn't support it.
    format!("\x1b[38;5;208m{s}\x1b[0m")
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

pub(crate) fn trace_log_state() -> &'static Mutex<Option<PathBuf>> {
    TRACE_LOG_PATH.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_trace_log_path(path: Option<PathBuf>) {
    if let Ok(mut slot) = trace_log_state().lock() {
        *slot = path;
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

pub(crate) fn trace(args: &Args, msg: &str) {
    let line = format!("trace: {msg}");
    append_trace_log_line(&line);
    if args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", ansi_paler_yellow(&line));
        }
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
    let line = format!("cmd> {compact}");
    append_trace_log_line(&line);
    if !(args.tune || args.calibrate) || args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", ansi_soft_green(&line));
        }
    }
}

/// Strip <think>...</think> blocks. If an opening tag is found without a closing tag,
/// drop the rest to avoid leaking partial reasoning.
pub(crate) fn strip_think_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + "<think>".len()..];
        if let Some(end) = after_start.find("</think>") {
            rest = &after_start[end + "</think>".len()..];
        } else {
            // Unclosed tag: drop rest.
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out.trim().to_string()
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
    // Mirrors llama.cpp WebUI parseReasoningContent(): split content into plain + reasoning parts
    // based on sentinel markers. Unterminated reasoning marker consumes rest.
    let mut plain_parts: Vec<&str> = Vec::new();
    let mut reasoning_parts: Vec<&str> = Vec::new();

    let mut cursor = 0usize;
    while cursor < content.len() {
        let Some(start_idx_rel) = content[cursor..].find(LLAMA_REASONING_START) else {
            plain_parts.push(&content[cursor..]);
            break;
        };
        let start_idx = cursor + start_idx_rel;
        plain_parts.push(&content[cursor..start_idx]);

        let reasoning_start = start_idx + LLAMA_REASONING_START.len();
        if reasoning_start >= content.len() {
            break;
        }

        let Some(end_idx_rel) = content[reasoning_start..].find(LLAMA_REASONING_END) else {
            reasoning_parts.push(&content[reasoning_start..]);
            break;
        };
        let end_idx = reasoning_start + end_idx_rel;
        reasoning_parts.push(&content[reasoning_start..end_idx]);
        cursor = end_idx + LLAMA_REASONING_END.len();
    }

    let plain = plain_parts.join("");
    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join("\n\n"))
    };
    (plain, reasoning)
}

/// Extract (thinking, final) from either structured fields or tagged output.
///
/// Mirrors the Open WebUI "compatible provider" strategy:
/// - Prefer `content` if non-empty, else fall back to `reasoning_content`.
/// - Strip `<think>...</think>` blocks from the final user-visible text.
/// - If tags are present, also return extracted thinking for display.
pub(crate) fn split_thinking_and_final(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> (Option<String>, String) {
    let c0 = content.unwrap_or("").trim();
    let r = reasoning_content.unwrap_or("").trim();

    // First, strip llama.cpp sentinel reasoning blocks out of content if present.
    let (c_plain, c_reasoning_from_sentinels) =
        if !c0.is_empty() && c0.contains(LLAMA_REASONING_START) {
            split_llama_sentinel_reasoning(c0)
        } else {
            (c0.to_string(), None)
        };
    let c = c_plain.trim();

    // If both exist, treat reasoning_content as thinking and content as final.
    // Also treat sentinel-extracted reasoning as thinking when present.
    if !c.is_empty() && (!r.is_empty() || c_reasoning_from_sentinels.is_some()) {
        let thinking = if !r.is_empty() {
            Some(r.to_string())
        } else {
            c_reasoning_from_sentinels
        };
        return (thinking, strip_think_tags(c));
    }

    let text = if !c.is_empty() { c } else { r };
    if text.is_empty() {
        return (None, String::new());
    }

    // Parse <think> tags if present.
    if text.contains("<think>") {
        let mut thinking = String::new();
        let mut final_out = String::new();
        let mut rest = text;

        while let Some(s) = rest.find("<think>") {
            final_out.push_str(&rest[..s]);
            let after_start = &rest[s + "<think>".len()..];
            if let Some(e) = after_start.find("</think>") {
                let chunk = &after_start[..e];
                if !chunk.trim().is_empty() {
                    if !thinking.is_empty() {
                        thinking.push_str("\n\n");
                    }
                    thinking.push_str(chunk.trim());
                }
                rest = &after_start[e + "</think>".len()..];
            } else {
                // Unclosed tag: treat remaining as thinking and stop.
                let chunk = after_start;
                if !chunk.trim().is_empty() {
                    if !thinking.is_empty() {
                        thinking.push_str("\n\n");
                    }
                    thinking.push_str(chunk.trim());
                }
                rest = "";
                break;
            }
        }
        final_out.push_str(rest);
        let final_out = final_out.trim().to_string();

        let thinking_opt = if thinking.trim().is_empty() {
            None
        } else {
            Some(thinking)
        };
        return (thinking_opt, final_out);
    }

    (None, strip_think_tags(text))
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

pub(crate) async fn chat_once(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse> {
    let mut last_error = String::new();
    for attempt in 0..3u32 {
        match client.post(chat_url.clone()).json(req).send().await {
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

                let parsed: ChatCompletionResponse =
                    serde_json::from_str(&text).context("Invalid JSON from server")?;
                return Ok(parsed);
            }
            Err(e) => {
                last_error = format!("{e:#}");
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                    continue;
                }
            }
        }
    }
    anyhow::bail!("POST /v1/chat/completions failed after retries: {last_error}")
}
