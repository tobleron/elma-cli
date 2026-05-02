//! @efficiency-role: util-pure
//!
//! App Chat - Helper Functions

use crate::app::AppRuntime;
use crate::*;

/// Task 014: Truncate output to prevent infinite token repetition
/// Maximum 2000 characters, truncated at last complete sentence.
fn truncate_output(text: &str) -> String {
    const MAX_CHARS: usize = 2000;

    if text.len() <= MAX_CHARS {
        return text.to_string();
    }

    // Truncate at MAX_CHARS using char indices to avoid UTF-8 boundary issues
    let truncated: String = text.chars().take(MAX_CHARS).collect();

    // Find last sentence boundary (., !, ?, or newline)
    let last_boundary = truncated
        .char_indices()
        .rfind(|(_, c)| matches!(c, '.' | '!' | '?' | '\n'));

    let result = match last_boundary {
        Some((pos, '\n')) => truncated[..pos].to_string(),
        Some((pos, _)) => truncated[..=pos].to_string(),
        None => truncated.to_string(),
    };

    format!("{} (truncated)", result.trim())
}

pub(crate) fn print_final_output(
    args: &Args,
    ctx_max: Option<u64>,
    final_usage_total: Option<u64>,
    final_text: &str,
    effort_timer: Option<&crate::ui_effort::EffortTimer>,
) {
    // Task 014: Truncate output to prevent infinite repetition bugs
    let truncated_text = truncate_output(final_text);
    print_elma_message(args, &truncated_text);

    // Intel unit failure count (warning color — model reliability tracking)
    let total_failures = crate::ui_state::get_total_intel_failures();
    if total_failures > 0 {
        let counts = crate::ui_state::get_intel_failure_counts();
        let summary: Vec<String> = counts
            .iter()
            .map(|(unit, count)| format!("{}×{}", unit, count))
            .collect();
        let line = format!(
            "intel: {} failures ({})",
            total_failures,
            summary.join(", ")
        );
        eprintln!(
            "{}",
            if args.no_color {
                line
            } else {
                error_red(&line)
            }
        );
    }

    // Task 133: Status bar is handled by TUI — this function is a no-op
    // when running in TUI mode. Kept for compatibility with non-TUI paths.
    let _ = (args, ctx_max, final_usage_total, final_text, effort_timer);
}

pub(crate) fn refresh_runtime_workspace(runtime: &mut AppRuntime) -> Result<()> {
    runtime.ws = gather_workspace_context(&runtime.repo);
    runtime.ws_brief = gather_workspace_brief(&runtime.repo);
    runtime.system_content = rebuild_system_content(
        &runtime.profiles.elma_cfg.system_prompt,
        &runtime.ws,
        &runtime.ws_brief,
        &runtime.guidance,
        &runtime.model_id,
        runtime.chat_url.as_str(),
    );
    if let Some(system_message) = runtime.messages.first_mut() {
        if system_message.role == "system" {
            system_message.content = runtime.system_content.clone();
        }
    }
    persist_runtime_workspace_intel(
        &runtime.args,
        &runtime.session,
        &runtime.ws,
        &runtime.ws_brief,
    )?;
    Ok(())
}

pub(crate) fn rebuild_system_content(
    base_prompt: &str,
    ws: &str,
    ws_brief: &str,
    guidance: &GuidanceSnapshot,
    model_id: &str,
    base_url: &str,
) -> String {
    let mut system_content = base_prompt.to_string();
    if !model_id.trim().is_empty() || !base_url.trim().is_empty() {
        system_content.push_str("\n\nRUNTIME CONTEXT:\n");
        if !model_id.trim().is_empty() {
            system_content.push_str(&format!("model_id: {}\n", model_id.trim()));
        }
        if !base_url.trim().is_empty() {
            system_content.push_str(&format!("base_url: {}\n", base_url.trim()));
        }
    }
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let guidance_text = guidance.render_for_system_prompt();
    if !guidance_text.is_empty() {
        system_content.push_str("\n\nPROJECT GUIDANCE:\n");
        system_content.push_str(&guidance_text);
    }
    system_content
}

pub(crate) fn persist_runtime_workspace_intel(
    args: &Args,
    session: &SessionPaths,
    ws: &str,
    ws_brief: &str,
) -> Result<()> {
    use crate::session_write::mutate_session_doc;
    let _ = mutate_session_doc(&session.root, |doc| {
        if doc.get("runtime").is_none() {
            doc["runtime"] = serde_json::json!({});
        }
        if !ws.is_empty() {
            doc["runtime"]["workspace"] = serde_json::json!(ws);
        }
        if !ws_brief.is_empty() {
            doc["runtime"]["workspace_brief"] = serde_json::json!(ws_brief);
        }
    });

    // Legacy writes for backward compatibility
    if !ws.is_empty() {
        let path = session.root.join("workspace.txt");
        let _ = std::fs::write(&path, ws.trim().to_string() + "\n");
        trace(args, &format!("workspace_context_saved={}", path.display()));
    }
    if !ws_brief.is_empty() {
        let path = session.root.join("workspace_brief.txt");
        let _ = std::fs::write(&path, ws_brief.trim().to_string() + "\n");
        trace(args, &format!("workspace_brief_saved={}", path.display()));
    }
    Ok(())
}

pub(crate) async fn maybe_save_formula_memory(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    memory_gate_cfg: &Profile,
    model_id: &str,
    model_cfg_dir: &PathBuf,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    scope: &ScopePlan,
    program: &Program,
    step_results: &[StepResult],
    reasoning_clean: bool,
) -> Result<()> {
    if !formula.memory_id.trim().is_empty() {
        let reuse_success = reasoning_clean && step_results.iter().all(|result| result.ok);
        let artifact_mode_capable = step_results
            .iter()
            .any(|result| result.artifact_path.is_some());
        if let Ok(Some(record)) = record_formula_memory_reuse(
            model_cfg_dir,
            formula.memory_id.trim(),
            reuse_success,
            artifact_mode_capable,
        ) {
            trace(
                args,
                &format!(
                    "formula_memory_reuse id={} status={} success_count={} failure_count={} disabled={}",
                    record.id,
                    if reuse_success { "success" } else { "failure" },
                    record.success_count,
                    record.failure_count,
                    record.disabled
                ),
            );
        }
        return Ok(());
    }

    if !reasoning_clean {
        trace(
            args,
            "memory_gate_status=skip reason=unclean_reasoning_fallback",
        );
        return Ok(());
    }
    if request_requires_workspace_evidence(route_decision, complexity, formula)
        && !step_results_have_workspace_evidence(step_results)
    {
        trace(
            args,
            "memory_gate_status=skip reason=missing_workspace_evidence",
        );
        return Ok(());
    }
    if step_results.iter().all(|result| result.ok)
        && !route_decision.route.eq_ignore_ascii_case("CHAT")
    {
        let gate = gate_formula_memory_once(
            client,
            chat_url,
            memory_gate_cfg,
            line,
            route_decision,
            complexity,
            formula,
            scope,
            program,
            step_results,
        )
        .await
        .unwrap_or_else(|_| MemoryGateVerdict {
            status: "skip".to_string(),
            reason: "memory_gate_error".to_string(),
        });
        trace(
            args,
            &format!("memory_gate_status={} reason={}", gate.status, gate.reason),
        );
        if !gate.status.eq_ignore_ascii_case("save") {
            return Ok(());
        }
        let now = now_unix_s()?;
        let active_run_id = load_active_manifest(&model_active_manifest_path(model_cfg_dir))
            .ok()
            .and_then(|m| m.active_run_id)
            .unwrap_or_default();
        let record = FormulaMemoryRecord {
            id: format!("fm_{now}"),
            created_unix_s: now,
            model_id: model_id.to_string(),
            active_run_id,
            user_message: line.to_string(),
            route: route_decision.route.clone(),
            complexity: complexity.complexity.clone(),
            formula: if formula.primary.trim().is_empty() {
                complexity.suggested_pattern.clone()
            } else {
                formula.primary.clone()
            },
            objective: program.objective.clone(),
            title: if !scope.objective.trim().is_empty() {
                scope.objective.clone()
            } else {
                line.to_string()
            },
            program_signature: program_signature(program),
            last_success_unix_s: now,
            last_failure_unix_s: 0,
            success_count: 1,
            failure_count: 0,
            disabled: false,
            artifact_mode_capable: step_results
                .iter()
                .any(|result| result.artifact_path.is_some()),
        };
        if let Ok(path) = save_formula_memory(model_cfg_dir, &record) {
            trace(args, &format!("formula_memory_saved={}", path.display()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_output_short_text() {
        let text = "Hello, this is a short message.";
        let result = truncate_output(text);
        assert_eq!(result, text);
        assert!(!result.contains("(truncated)"));
    }

    #[test]
    fn test_truncate_output_long_text() {
        // Create text longer than 2000 chars
        let text = "A".repeat(3000) + ". The end.";
        let result = truncate_output(&text);
        assert!(result.len() < 3000);
        assert!(result.contains("(truncated)"));
    }

    #[test]
    fn test_truncate_output_at_sentence() {
        // Text that should truncate at sentence boundary
        let text = "First sentence. Second sentence. ".repeat(100);
        let result = truncate_output(&text);
        assert!(result.contains("(truncated)"));
        // Should end at a sentence boundary
        assert!(result.ends_with(". (truncated)") || result.ends_with("(truncated)"));
    }

    #[test]
    fn test_truncate_output_infinite_repetition() {
        // Simulate the S001 bug with repeated tokens
        let text = "<font color='blue'>...".repeat(500);
        let result = truncate_output(&text);
        assert!(result.len() <= 2020); // 2000 + " (truncated)"
        assert!(result.contains("(truncated)"));
    }
}
