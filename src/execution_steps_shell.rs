//! @efficiency-role: orchestrator
//!
//! Execution Steps - Shell Step Handling (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - execution_steps_shell_preflight: Preflight checks and validation
//! - execution_steps_shell_exec: Execution and post-processing

use crate::execution_steps_compat::*;
use crate::execution_steps_shell_exec::*;
use crate::execution_steps_shell_preflight::*;
use crate::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_shell_step(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    status_message_cfg: &Profile,
    command_repair_cfg: Option<&Profile>,
    command_preflight_cfg: Option<&Profile>,
    task_semantics_guard_cfg: Option<&Profile>,
    evidence_compactor_cfg: Option<&Profile>,
    artifact_classifier_cfg: Option<&Profile>,
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    objective: &str,
    emit_shell_output: bool,
    readonly_only: bool,
    is_destructive: bool,
    sid: String,
    kind: String,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    cmd: String,
    state: &mut ExecutionState,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<()> {
    let cmd = match resolve_command_placeholders(&normalize_shell_cmd(&cmd), &state.artifacts) {
        Ok(cmd) => cmd,
        Err(error) => {
            trace(
                args,
                &format!("shell_template_error id={sid} error={error}"),
            );
            state.halt = true;
            state.step_results.push(StepResult {
                id: sid.clone(),
                kind: kind.clone(),
                purpose: purpose.clone(),
                depends_on: depends_on.clone(),
                success_condition: success_condition.clone(),
                ok: false,
                summary: format!("template_resolution_failed: {error}"),
                command: Some(cmd),
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
            return Ok(());
        }
    };
    if !program_safety_check(&cmd) {
        trace(
            args,
            &format!("step_blocked id={sid} cmd={}", cmd.replace('\n', " ")),
        );
        state.step_results.push(StepResult {
            id: sid.clone(),
            kind: kind.clone(),
            purpose: purpose.clone(),
            depends_on: depends_on.clone(),
            success_condition: success_condition.clone(),
            ok: false,
            summary: "blocked_by_policy".to_string(),
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
        return Ok(());
    }
    if readonly_only && !command_is_readonly(&cmd) {
        trace(
            args,
            &format!(
                "step_skipped_readonly_only id={sid} cmd={}",
                cmd.replace('\n', " ")
            ),
        );
        state.step_results.push(StepResult {
            id: sid.clone(),
            kind: kind.clone(),
            purpose: purpose.clone(),
            depends_on: depends_on.clone(),
            success_condition: success_condition.clone(),
            ok: false,
            summary: "skipped_by_calibration_policy".to_string(),
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
        return Ok(());
    }

    if !permission_gate::check_permission(args, &cmd, is_destructive, tui.as_deref_mut()).await {
        trace(
            args,
            &format!("step_denied id={sid} cmd={}", cmd.replace('\n', " ")),
        );
        state.step_results.push(StepResult {
            id: sid.clone(),
            kind: kind.clone(),
            purpose: purpose.clone(),
            depends_on: depends_on.clone(),
            success_condition: success_condition.clone(),
            ok: false,
            summary: "denied_by_user".to_string(),
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
        return Ok(());
    }

    // Display status message before execution
    let status_unit = StatusMessageUnit::new(status_message_cfg.clone());
    if let Ok(context) = IntelContext::new(
        "executing".to_string(),
        neutral_route_decision(),
        String::new(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("current_action", "executing")
    .and_then(|ctx| ctx.with_extra("step_type", &kind))
    .and_then(|ctx| ctx.with_extra("step_purpose", &purpose))
    {
        if let Ok(output) = status_unit.execute_with_fallback(&context).await {
            if let Some(status) = output.get_str("status") {
                show_status_message(args, status);
            }
        }
    }

    let (cmd, execution_mode, artifact_kind, _ask_hint, should_halt, _halt_summary) =
        preflight_shell_command(
            args,
            client,
            chat_url,
            session,
            workdir,
            command_repair_cfg,
            command_preflight_cfg,
            task_semantics_guard_cfg,
            scope,
            complexity,
            formula,
            objective,
            &purpose,
            readonly_only,
            &sid,
            cmd,
            state,
        )
        .await?;

    if should_halt {
        return Ok(());
    }

    execute_and_process_shell(
        args,
        client,
        chat_url,
        session,
        workdir,
        command_repair_cfg,
        task_semantics_guard_cfg,
        evidence_compactor_cfg,
        artifact_classifier_cfg,
        scope,
        complexity,
        formula,
        objective,
        &purpose,
        emit_shell_output,
        readonly_only,
        sid,
        kind,
        purpose.clone(),
        depends_on,
        success_condition,
        cmd,
        execution_mode,
        artifact_kind,
        state,
    )
    .await
}
