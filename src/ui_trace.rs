//! @efficiency-role: util-pure
//!
//! UI - Trace and Display Functions

use crate::ui_theme::*;
use crate::*;

/// Display an ultra-concise status message about what Elma is doing
pub(crate) fn show_status_message(args: &Args, status: &str) {
    let line = format!("→ {}", status);
    append_trace_log_line(&line);
    if args.show_process {
        if args.no_color {
            eprintln!("{}", line);
        } else {
            eprintln!("{}", info_cyan(&line));
        }
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
            eprintln!("{}", warn_yellow(&rendered));
        }
    }
}

pub(crate) fn trace(args: &Args, msg: &str) {
    let line = format!("trace: {msg}");
    append_trace_log_line(&line);
}

pub(crate) fn trace_verbose(verbose: bool, msg: &str) {
    let line = format!("trace: {msg}");
    append_trace_log_line(&line);
}

pub(crate) fn show_process_step_verbose(verbose: bool, category: &str, msg: &str) {
    let line = format!("[{}] {}", category, msg);
    append_trace_log_line(&line);
    if verbose {
        match category {
            "CLASSIFY" => eprintln!("{}", meta_comment(&line)),
            "PLAN" => eprintln!("{}", info_cyan(&line)),
            "REFLECT" => eprintln!("{}", elma_accent(&line)),
            _ => eprintln!("{}", meta_comment(&line)),
        }
    }
}

pub(crate) fn show_process_step(args: &Args, category: &str, msg: &str) {
    show_process_step_verbose(args.show_process, category, msg);
}

pub(crate) fn show_intel_summary(verbose: bool, summary: &str) {
    let line = format!("note: {}", summary);
    append_trace_log_line(&line);
    if verbose {
        eprintln!("{}", warn_yellow(&line));
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
    let line = format!("→ {msg}");
    append_trace_log_line(&line);
    if !(args.tune || args.calibrate) || args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", info_cyan(&line));
        }
    }
}

pub(crate) fn shell_command_trace(args: &Args, cmd: &str) {
    let compact = cmd.replace('\n', " ");
    let line = format!("→ {compact}");
    append_trace_log_line(&line);
    if !(args.tune || args.calibrate) || args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", warn_yellow(&line));
        }
    }
}

/// Fallback message printer for when TUI is not available (command handlers).
/// Uses plain println — the TUI will override these when active.
pub(crate) fn print_elma_message(args: &Args, text: &str) {
    if args.no_color {
        println!("● {}", text);
    } else {
        println!("{} {}", elma_accent("●"), text);
    }
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
    let mut out = String::with_capacity(s.len());
    let mut last_was_nl = false;
    let mut nl_run = 0u32;
    for ch in s.chars() {
        if ch == '\n' {
            nl_run += 1;
            if nl_run <= 1 {
                out.push('\n');
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

pub(crate) fn strip_think_tags(s: &str) -> String {
    thinking_content::strip_think_tags(s)
}

pub(crate) fn split_llama_sentinel_reasoning(content: &str) -> (String, Option<String>) {
    let (plain, thinking) = thinking_content::extract_llama_sentinel_reasoning(content);
    (plain, thinking)
}

pub(crate) fn split_thinking_and_final(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> (Option<String>, String) {
    let extraction = thinking_content::extract_thinking(content, reasoning_content);
    (extraction.thinking, extraction.final_answer)
}

pub(crate) fn extract_final_line(text: &str, prefix: &str) -> Option<String> {
    let p = prefix.trim();
    if p.is_empty() {
        return None;
    }
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

pub(crate) fn extract_response_text(resp: &ChatCompletionResponse) -> String {
    resp.choices
        .get(0)
        .and_then(|c| c.message.content.as_deref())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub(crate) fn extract_response_reasoning(resp: &ChatCompletionResponse) -> String {
    resp.choices
        .get(0)
        .and_then(|c| c.message.reasoning_content.as_deref())
        .unwrap_or("")
        .trim()
        .to_string()
}
