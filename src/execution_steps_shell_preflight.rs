//! @efficiency-role: service-orchestrator
//!
//! Execution Steps Shell - Preflight Phase

use crate::execution_steps_compat::*;
use crate::*;

/// Handle command preflight checks and validation
/// Returns: (finalized_cmd, execution_mode, artifact_kind, ask_hint, should_halt, halt_summary)
#[allow(clippy::too_many_arguments)]
pub(crate) async fn preflight_shell_command(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    command_repair_cfg: Option<&Profile>,
    command_preflight_cfg: Option<&Profile>,
    task_semantics_guard_cfg: Option<&Profile>,
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    objective: &str,
    purpose: &str,
    readonly_only: bool,
    sid: &str,
    cmd: String,
    state: &mut ExecutionState,
) -> Result<(String, String, String, String, bool, Option<String>)> {
    let mut cmd = cmd;
    let mut execution_mode = "INLINE".to_string();
    let mut artifact_kind = "shell_output".to_string();
    let mut ask_hint = String::new();

    let compatibility = probe_command_compatibility(&cmd, workdir);
    if let Some(preflight_cfg) = command_preflight_cfg {
        if let Ok(preflight) = preflight_command_once(
            client,
            chat_url,
            preflight_cfg,
            objective,
            purpose,
            scope,
            complexity,
            formula,
            &cmd,
            &compatibility.os_family,
            &compatibility.shell_path,
            &compatibility.primary_bin,
            compatibility.command_exists,
            &compatibility.command_lookup,
        )
        .await
        {
            trace(
                args,
                &format!(
                    "preflight id={sid} status={} reason={}",
                    preflight.status.trim(),
                    preflight.reason.trim()
                ),
            );
            if !preflight.execution_mode.trim().is_empty() {
                execution_mode = preflight.execution_mode.trim().to_uppercase();
            }
            if !preflight.artifact_kind.trim().is_empty() {
                artifact_kind = preflight.artifact_kind.trim().to_string();
            }
            if execution_mode.eq_ignore_ascii_case("ASK") {
                ask_hint = if preflight.question.trim().is_empty() {
                    preflight.reason.trim().to_string()
                } else {
                    preflight.question.trim().to_string()
                };
            }
            if preflight.status.eq_ignore_ascii_case("revise") {
                let revised = normalize_shell_cmd(preflight.cmd.trim());
                if !revised.is_empty()
                    && revised != cmd
                    && program_safety_check(&revised)
                    && (!readonly_only || command_is_readonly(&revised))
                {
                    let mut accepted = true;
                    if let Some(guard_cfg) = task_semantics_guard_cfg {
                        if let Ok(guard) = guard_repair_semantics_once(
                            client,
                            chat_url,
                            guard_cfg,
                            objective,
                            purpose,
                            &cmd,
                            &revised,
                            "",
                        )
                        .await
                        {
                            trace(
                                args,
                                &format!(
                                    "preflight_semantics id={sid} status={} reason={}",
                                    guard.status.trim(),
                                    guard.reason.trim()
                                ),
                            );
                            accepted = guard.status.eq_ignore_ascii_case("accept");
                        }
                    }
                    if accepted {
                        cmd = revised;
                    }
                }
            } else if preflight.status.eq_ignore_ascii_case("reject") {
                let user_reason = if preflight.question.trim().is_empty() {
                    preflight.reason.trim()
                } else {
                    preflight.question.trim()
                };
                state.final_reply = Some(
                    format!(
                        "Explain briefly that the requested shell action was not executed. Reason: {}. Ask one concise follow-up only if it helps narrow the request or choose a compatible alternative.",
                        user_reason
                    ),
                );
                state.halt = true;
                state.artifacts.insert(sid.to_string(), user_reason.to_string());
                state.step_results.push(StepResult {
                    id: sid.to_string(),
                    kind: "shell".to_string(),
                    purpose: purpose.to_string(),
                    depends_on: vec![],
                    success_condition: String::new(),
                    ok: false,
                    summary: format!("preflight_rejected: {}", user_reason),
                    command: Some(cmd.clone()),
                    raw_output: None,
                    exit_code: None,
                    output_bytes: None,
                    truncated: false,
                    timed_out: false,
                    artifact_path: None,
                    artifact_kind: None,
                    outcome_status: None,
                    outcome_reason: None,
                });
                return Ok((cmd, execution_mode, artifact_kind, ask_hint, true, Some(format!("preflight_rejected: {}", user_reason))));
            }
        }
    }

    if execution_mode.eq_ignore_ascii_case("ASK") {
        let ask_reason = if ask_hint.trim().is_empty() {
            "The shell action needs narrowing before execution.".to_string()
        } else {
            ask_hint.clone()
        };
        state.final_reply = Some(
            format!(
                "Explain briefly that the shell action was not executed yet. Reason: {}. Ask one concise clarifying question or offer a compatible alternative if that would help.",
                ask_reason
            ),
        );
        state.halt = true;
        state.artifacts.insert(sid.to_string(), ask_reason.clone());
        state.step_results.push(StepResult {
            id: sid.to_string(),
            kind: "shell".to_string(),
            purpose: purpose.to_string(),
            depends_on: vec![],
            success_condition: String::new(),
            ok: false,
            summary: format!("preflight_requires_clarification: {ask_reason}"),
            command: Some(cmd.clone()),
            raw_output: None,
            exit_code: None,
            output_bytes: None,
            truncated: false,
            timed_out: false,
            artifact_path: None,
            artifact_kind: None,
            outcome_status: None,
            outcome_reason: None,
        });
        return Ok((cmd, execution_mode, artifact_kind, ask_hint, true, Some(format!("preflight_requires_clarification: {ask_reason}"))));
    }

    Ok((cmd, execution_mode, artifact_kind, ask_hint, false, None))
}
