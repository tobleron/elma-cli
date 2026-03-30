//! @efficiency-role: service-orchestrator
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
    sid: String,
    kind: String,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    cmd: String,
    state: &mut ExecutionState,
) -> Result<()> {
    let cmd = match resolve_command_placeholders(&normalize_shell_cmd(&cmd), &state.artifacts) {
        Ok(cmd) => cmd,
        Err(error) => {
            trace(args, &format!("shell_template_error id={sid} error={error}"));
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

    // Display status message before execution
    if let Ok(status) = generate_status_message_once(
        client,
        chat_url,
        &Profile {
            version: 1,
            name: "status_message_generator".to_string(),
            base_url: "".to_string(),
            model: "".to_string(),
            temperature: 0.3,
            top_p: 0.95,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 64,
            timeout_s: 30,
            system_prompt: "Generate an ultra-concise status message explaining what Elma is doing now. Return JSON: {\"status\":\"one line, max 10 words\"}".to_string(),
        },
        "executing",
        &kind,
        &purpose,
    ).await {
        show_status_message(args, &status);
    }

    let (cmd, execution_mode, artifact_kind, _ask_hint, should_halt, _halt_summary) = preflight_shell_command(
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
