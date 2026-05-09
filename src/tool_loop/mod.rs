use crate::*;
use std::future::Future;
use std::time::{Duration};
use std::path::{PathBuf};
use crate::orchestration::tool::ToolStateMachine;

pub mod streaming;
pub mod finalization;
pub mod coverage;

pub(crate) struct ToolLoopResult {
    pub(crate) final_answer: String,
    pub(crate) iterations: usize,
    pub(crate) tool_calls_made: usize,
    pub(crate) stopped_by_max: bool,
    pub(crate) stop_outcome: Option<crate::stop_policy::StopOutcome>,
    pub(crate) total_elapsed_s: f64,
    pub(crate) timeout_reason: Option<String>,
    pub(crate) evidence_progress_summary: Option<String>,
    pub(crate) loop_summary: ToolLoopSummary,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ToolLoopSummary {
    pub tool_calls_made: usize,
    pub tool_call_ids: Vec<String>,
    pub successful_reads: Vec<String>,
    pub successful_searches: Vec<String>,
    pub failed_operations: Vec<(String, String)>,
    pub duplicate_suppressions: usize,
    pub coverage: Option<(usize, usize)>,
    pub stop_reason: String,
    pub stop_iteration: usize,
}

pub(crate) async fn await_with_busy_input<T, F>(
    tui: &mut crate::ui_terminal::TerminalUI,
    future: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
           result = &mut future => return result,
            _ = tokio::time::sleep(Duration::from_millis(40)) => {
                tui.process_pending_input_events();
                let _ = tui.pump_ui();
                if let Ok(Some(queued)) = tui.poll_busy_submission() {
                    tui.enqueue_submission(queued);
                }
            }
        }
    }
}

pub(crate) fn is_tool_call_markup(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }
    let lower = t.to_ascii_lowercase();
    lower.contains("<tool_call>")
        || lower.contains("</tool_call>")
        || (lower.contains("\"name\"")
            && lower.contains("\"arguments\"")
            && (lower.contains("\"name\":\"shell\"")
                || lower.contains("\"name\":\"read\"")
                || lower.contains("\"name\":\"search\"")
                || lower.contains("\"name\":\"respond\"")
                || lower.contains("\"name\":\"update_todo_list\"")
                || lower.contains("\"name\": \"shell\"")
                || lower.contains("\"name\": \"read\"")
                || lower.contains("\"name\": \"search\"")
                || lower.contains("\"name\": \"respond\"")
                || lower.contains("\"name\": \"update_todo_list\"")))
}

pub(crate) fn is_intent_only_response(text: &str) -> bool {
    crate::orchestration::tool::is_intent_only_response(text)
}

pub(crate) fn tool_signal(tc: &ToolCall) -> String {
    crate::orchestration::tool::tool_signal(tc)
}

pub(crate) fn extract_tool_arg_preview(args_json: &str, field: &str, max_len: usize) -> String {
    match serde_json::from_str::<serde_json::Value>(args_json) {
        Ok(val) => val
            .get(field)
            .and_then(|v| v.as_str())
            .map(|s| {
                if s.len() > max_len {
                    format!("{}...", s.chars().take(max_len).collect::<String>())
                } else {
                    s.to_string()
                }
            })
            .unwrap_or_else(|| args_json.chars().take(max_len).collect()),
        Err(_) => args_json.chars().take(max_len).collect(),
    }
}

pub(crate) async fn run_tool_loop(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    system_prompt: &str,
    user_message: &str,
    workdir: &PathBuf,
    sess: &SessionPaths,
    temperature: f64,
    max_tokens: u32,
    tui: &mut crate::ui_terminal::TerminalUI,
    summarizer_cfg: Option<&Profile>,
    context_hint: &str,
    evidence_required: bool,
    ctx_max: Option<u64>,
    goal_state: &GoalState,
    complexity: &str,
    raw_user_request: Option<&str>,
    work_graph_runner: crate::work_graph_runner::WorkGraphRunner,
) -> Result<ToolLoopResult> {
    let state_machine = ToolStateMachine::new(
        args,
        client,
        chat_url,
        model_id,
        system_prompt,
        user_message,
        workdir,
        sess,
        temperature,
        max_tokens,
        tui,
        summarizer_cfg,
        context_hint,
        evidence_required,
        ctx_max,
        goal_state,
        complexity,
        raw_user_request,
        work_graph_runner,
    );

    state_machine.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_tool_call_markup() {
        assert!(is_tool_call_markup(
            "<tool_call>{\"name\":\"shell\"}</tool_call>"
        ));
        assert!(is_tool_call_markup(
            "{\"name\":\"shell\",\"arguments\":{\"command\":\"ls\"}}"
        ));
        assert!(!is_tool_call_markup(
            "The latest prompts are in sessions/history.txt."
        ));
    }

    #[test]
    fn tool_signal_uses_semantic_fields() {
        let tc = ToolCall {
            id: "c1".to_string(),
            call_type: "function".to_string(),
            function: ToolFunctionCall {
                name: "read".to_string(),
                arguments: r#"{"path":"sessions/history.txt"}"#.to_string(),
            },
        };
        assert_eq!(tool_signal(&tc), "read:sessions/history.txt");
    }

    #[test]
    fn normalizes_shell_signal_session_ids() {
        let a =
            crate::text_utils::normalize_shell_signal("ls sessions/s_1776868918_801751000/shell/");
        let b =
            crate::text_utils::normalize_shell_signal("ls sessions/s_1775151941_439997000/shell/");
        assert_eq!(a, b);
        assert!(a.contains("s_SESSION"));
    }

    #[test]
    fn fallback_uses_recent_tool_content() {
        use crate::tool_loop::finalization::build_fallback_from_recent_tool_evidence;
        let msgs = vec![
            ChatMessage::simple("user", "hello"),
            ChatMessage {
                role: "tool".to_string(),
                content: "line one\nline two".to_string(),
                name: Some("shell".to_string()),
                tool_calls: None,
                tool_call_id: Some("t1".to_string()),
                reasoning_content: None,
                summarized: false,
            },
        ];
        let out = build_fallback_from_recent_tool_evidence(&msgs, None);
        assert!(out.contains("line one"));
    }

    #[test]
    fn finalization_evidence_is_bounded_and_recent() {
        use crate::tool_loop::finalization::{build_bounded_final_evidence, FINAL_EVIDENCE_TOTAL_MAX_CHARS};
        let old = "old evidence ".repeat(500);
        let recent = "recent evidence ".repeat(500);
        let mut msgs = vec![ChatMessage::simple("user", "summarize")];
        msgs.push(ChatMessage {
            role: "tool".to_string(),
            content: old,
            name: Some("read".to_string()),
            tool_calls: None,
            tool_call_id: Some("old".to_string()),
            reasoning_content: None,
            summarized: false,
        });
        msgs.push(ChatMessage {
            role: "tool".to_string(),
            content: recent,
            name: Some("read".to_string()),
            tool_calls: None,
            tool_call_id: Some("recent".to_string()),
            reasoning_content: None,
            summarized: false,
        });

        let block = build_bounded_final_evidence(&msgs);
        assert!(block.contains("recent evidence"));
        assert!(block.chars().count() <= FINAL_EVIDENCE_TOTAL_MAX_CHARS + 120);
        assert!(block.contains("omitted from finalization evidence"));
    }

    #[test]
    fn normalize_final_answer_strips_think_and_tool_call_blocks() {
        use crate::tool_loop::finalization::normalize_final_answer_candidate;
        let raw = "<think>hidden</think>\nAnswer\n<tool_call>{\"name\":\"respond\"}</tool_call>";
        assert_eq!(normalize_final_answer_candidate(raw), "Answer");
    }

    #[test]
    fn broad_read_scope_blocks_until_discovered_files_are_covered() {
        use crate::tool_loop::coverage::{read_call_requests_broad_scope, scope_coverage_blocks_finalization};
        let mut ledger = crate::scope_coverage::ScopeCoverageLedger::new();
        assert!(read_call_requests_broad_scope(&vec![
            "docs/a.md".to_string(),
            "docs/b.md".to_string(),
        ]));
        assert!(scope_coverage_blocks_finalization(true, &ledger));

        ledger.register_items(&["docs/a.md".to_string(), "docs/b.md".to_string()], "file");
        ledger.mark_covered("docs/a.md");
        assert!(scope_coverage_blocks_finalization(true, &ledger));

        ledger.mark_covered("docs/b.md");
        assert!(!scope_coverage_blocks_finalization(true, &ledger));
    }

    #[test]
    fn ls_scope_paths_are_concrete_workspace_files() {
        use crate::tool_loop::coverage::extract_ls_scope_paths;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs").join("a.md"), "a").unwrap();
        std::fs::write(root.join("docs").join("b.md"), "b").unwrap();
        std::fs::create_dir_all(root.join("docs").join("nested")).unwrap();

        let output = "docs/  (3 item(s))\n    nested/\n    a.md  (1 B, now)\n    b.md  (1 B, now)";
        let paths = extract_ls_scope_paths(r#"{"path":"docs"}"#, output, root);
        assert_eq!(
            paths,
            vec!["docs/a.md".to_string(), "docs/b.md".to_string()]
        );
    }

    #[test]
    fn shell_scope_output_registers_concrete_workspace_files_when_active() {
        use crate::tool_loop::coverage::update_scope_coverage_from_tool;
        use crate::tools::ToolExecutionResult;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(root.join("docs").join("a.md"), "a").unwrap();
        std::fs::write(root.join("docs").join("b.md"), "b").unwrap();

        let mut ledger = crate::scope_coverage::ScopeCoverageLedger::new();
        let result = ToolExecutionResult::new_ok("c1", "shell", "docs/a.md\ndocs/b.md\n");
        update_scope_coverage_from_tool(&mut ledger, "shell", "{}", &result, root, true);

        assert_eq!(ledger.total(), 2);
        assert!(crate::tool_loop::coverage::scope_coverage_blocks_finalization(true, &ledger));
    }
}
