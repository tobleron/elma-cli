//! @efficiency-role: service-orchestrator
//!
//! Execution Steps Shell - Execution and Post-processing

use crate::execution_steps_compat::*;
use crate::*;

async fn compact_evidence_via_unit(
    client: &reqwest::Client,
    compactor_cfg: &Profile,
    objective: &str,
    purpose: &str,
    scope: &ScopePlan,
    cmd: &str,
    output: &str,
) -> Result<EvidenceCompact> {
    let unit = EvidenceCompactorUnit::new(compactor_cfg.clone());
    let context = IntelContext::new(
        objective.to_string(),
        neutral_route_decision(),
        output.to_string(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("objective", objective)?
    .with_extra("purpose", purpose)?
    .with_extra("scope", scope)?
    .with_extra("cmd", cmd)?
    .with_extra("output", output)?;
    let intel_output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(intel_output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse evidence compact output: {}", e))
}

async fn classify_artifacts_via_unit(
    client: &reqwest::Client,
    classifier_cfg: &Profile,
    objective: &str,
    scope: &ScopePlan,
    evidence: &str,
) -> Result<ArtifactClassification> {
    let unit = ArtifactClassifierUnit::new(classifier_cfg.clone());
    let context = IntelContext::new(
        objective.to_string(),
        neutral_route_decision(),
        evidence.to_string(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("objective", objective)?
    .with_extra("scope", scope)?
    .with_extra("evidence", evidence)?;
    let intel_output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(intel_output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse artifact classification: {}", e))
}

async fn repair_command_via_unit(
    client: &reqwest::Client,
    repair_cfg: &Profile,
    objective: &str,
    purpose: &str,
    cmd: &str,
    output: &str,
) -> Result<CommandRepair> {
    let unit = CommandRepairUnit::new(repair_cfg.clone());
    let context = IntelContext::new(
        cmd.to_string(),
        neutral_route_decision(),
        summarize_shell_output(output),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("objective", objective)?
    .with_extra("purpose", purpose)?
    .with_extra("output", output)?;
    let intel_output = unit.execute_with_fallback(&context).await?;
    serde_json::from_value(intel_output.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse command repair output: {}", e))
}

/// Execute shell command and handle results
#[allow(clippy::too_many_arguments)]
pub(crate) async fn execute_and_process_shell(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    command_repair_cfg: Option<&Profile>,
    task_semantics_guard_cfg: Option<&Profile>,
    evidence_compactor_cfg: Option<&Profile>,
    artifact_classifier_cfg: Option<&Profile>,
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    objective: &str,
    purpose: &str,
    emit_shell_output: bool,
    readonly_only: bool,
    sid: String,
    kind: String,
    purpose_str: String,
    depends_on: Vec<String>,
    success_condition: String,
    cmd: String,
    execution_mode: String,
    artifact_kind: String,
    state: &mut ExecutionState,
) -> Result<()> {
    let artifact_reservation = if execution_mode.eq_ignore_ascii_case("ARTIFACT") {
        Some(reserve_artifact_path(
            &session.artifacts_dir,
            &artifact_kind,
            "txt",
        )?)
    } else {
        None
    };

    let path = write_shell_action(&session.shell_dir, &cmd)?;
    trace(args, &format!("shell_saved={}", path.display()));
    shell_command_trace(args, &cmd);
    let compatibility = probe_command_compatibility(&cmd, workdir);
    let mut shell_result = run_shell_one_liner(
        &cmd,
        workdir,
        artifact_reservation
            .as_ref()
            .map(|(_, path)| (path, artifact_kind.as_str())),
    )?;
    let mut code = shell_result.exit_code;
    let mut output = shell_result.inline_text.clone();
    let mut output_path_base = path.clone();
    let mut repaired_cmd: Option<String> = None;

    if code != 0 {
        if command_is_unavailable(&compatibility, &output) {
            return handle_command_unavailable(
                args,
                session,
                &sid,
                &kind,
                &purpose_str,
                &depends_on,
                &success_condition,
                &cmd,
                &output,
                &output_path_base,
                &shell_result,
                &compatibility,
                emit_shell_output,
                state,
            );
        }

        if let Some(repair_result) = try_command_repair(
            args,
            client,
            chat_url,
            session,
            workdir,
            command_repair_cfg,
            task_semantics_guard_cfg,
            objective,
            purpose,
            &cmd,
            &output,
            &output_path_base,
            &shell_result,
            readonly_only,
            &artifact_reservation,
            &artifact_kind,
            emit_shell_output,
            state,
        )
        .await?
        {
            code = repair_result.code;
            output = repair_result.output;
            output_path_base = repair_result.output_path_base;
            repaired_cmd = Some(repair_result.repaired_cmd);
        }
    }

    handle_artifact_recording(
        session,
        &artifact_reservation,
        &shell_result,
        &artifact_kind,
        &sid,
        args,
    )?;

    let shell_preview = if let Some(path) = shell_result.artifact_path.as_ref() {
        format!("{}\n[artifact: {}]", output.trim_end(), path.display())
    } else {
        output.clone()
    };
    let out_path = write_shell_output(&session.shell_dir, &output_path_base, &shell_preview)?;
    trace(args, &format!("shell_output_saved={}", out_path.display()));
    trace(args, &format!("exec_exit_code={code}"));
    // Only print to stdout when TUI is not active (TUI handles this via transcript)
    if !crate::ui_state::is_tui_active() && (emit_shell_output || code != 0) {
        println!("elma> exit_code={code}\n{shell_preview}");
    }
    state.artifacts.insert(format!("{sid}:raw"), output.clone());

    let mut compact_summary = summarize_shell_output(&output);
    if code != 0 {
        compact_summary = format!("EXECUTION FAILED (code {}):\n{}", code, compact_summary);
    } else if let Some(compactor_cfg) = evidence_compactor_cfg {
        if let Ok(compact) = compact_evidence_via_unit(
            client,
            compactor_cfg,
            objective,
            purpose,
            scope,
            repaired_cmd.as_deref().unwrap_or(&cmd),
            &output,
        )
        .await
        {
            let compact_text = summarize_evidence_compact(&compact);
            if !compact_text.trim().is_empty() {
                compact_summary = compact_text.clone();
                // Task 023: Store compact version separately, do NOT overwrite raw output
                // Grounded selection requires the raw path list for normalization.
                state
                    .artifacts
                    .insert(format!("{}:compact", sid), compact_text);
            }
        }
    }

    // Always store the raw/full output in the primary artifact key for grounded selection
    if !state.artifacts.contains_key(&sid) {
        state.artifacts.insert(sid.clone(), output.clone());
    }
    if let Some(classifier_cfg) = artifact_classifier_cfg {
        if should_classify_artifacts(complexity, formula) {
            if let Ok(classification) = classify_artifacts_via_unit(
                client,
                classifier_cfg,
                objective,
                scope,
                state
                    .artifacts
                    .get(&sid)
                    .map(String::as_str)
                    .unwrap_or(&output),
            )
            .await
            {
                let classification_text = summarize_artifact_classification(&classification);
                if !classification_text.trim().is_empty() {
                    state
                        .artifacts
                        .insert(format!("{sid}:classification"), classification_text.clone());
                    compact_summary = format!("{compact_summary}\n{classification_text}");
                }
            }
        }
    }

    state.step_results.push(StepResult {
        id: sid,
        kind,
        purpose: purpose_str,
        depends_on,
        success_condition,
        ok: code == 0,
        summary: if let Some(repaired) = repaired_cmd {
            format!("repaired_cmd: {}\n{}", repaired, compact_summary)
        } else {
            compact_summary
        },
        command: Some(cmd),
        raw_output: Some(output),
        exit_code: Some(code),
        output_bytes: Some(shell_result.bytes_written),
        truncated: shell_result.truncated,
        timed_out: shell_result.timed_out,
        artifact_path: shell_result
            .artifact_path
            .as_ref()
            .map(|p| p.display().to_string()),
        artifact_kind: shell_result.artifact_kind,
        outcome_status: None,
        outcome_reason: None,
    });
    Ok(())
}

struct RepairResult {
    code: i32,
    output: String,
    output_path_base: PathBuf,
    repaired_cmd: String,
}

fn handle_command_unavailable(
    args: &Args,
    session: &SessionPaths,
    sid: &str,
    kind: &str,
    purpose: &str,
    depends_on: &[String],
    success_condition: &str,
    cmd: &str,
    output: &str,
    output_path_base: &PathBuf,
    shell_result: &ShellExecutionResult,
    compatibility: &CommandCompatibilityFacts,
    emit_shell_output: bool,
    state: &mut ExecutionState,
) -> Result<()> {
    let shell_preview = if let Some(path) = shell_result.artifact_path.as_ref() {
        format!("{}\n[artifact: {}]", output.trim_end(), path.display())
    } else {
        output.to_string()
    };
    let out_path = write_shell_output(&session.shell_dir, output_path_base, &shell_preview)?;
    trace(args, &format!("shell_output_saved={}", out_path.display()));
    trace(
        args,
        &format!(
            "command_unavailable id={sid} bin={} os={} shell={}",
            compatibility.primary_bin.trim(),
            compatibility.os_family.trim(),
            compatibility.shell_path.trim()
        ),
    );
    trace(args, &format!("exec_exit_code={}", shell_result.exit_code));
    // Only print to stdout when TUI is not active (TUI handles this via transcript)
    if !crate::ui_state::is_tui_active() && (emit_shell_output || shell_result.exit_code != 0) {
        println!(
            "elma> exit_code={}\n{shell_preview}",
            shell_result.exit_code
        );
    }
    state.final_reply = Some(unavailable_reply_instructions(compatibility));
    state.halt = true;
    state
        .artifacts
        .insert(format!("{sid}:raw"), output.to_string());
    state.artifacts.insert(sid.to_string(), output.to_string());
    state.step_results.push(StepResult {
        id: sid.to_string(),
        kind: kind.to_string(),
        purpose: purpose.to_string(),
        depends_on: depends_on.to_vec(),
        success_condition: success_condition.to_string(),
        ok: false,
        summary: unavailable_summary(compatibility, output),
        command: Some(cmd.to_string()),
        raw_output: Some(output.to_string()),
        exit_code: Some(shell_result.exit_code),
        output_bytes: Some(shell_result.bytes_written),
        truncated: shell_result.truncated,
        timed_out: shell_result.timed_out,
        artifact_path: shell_result
            .artifact_path
            .as_ref()
            .map(|p| p.display().to_string()),
        artifact_kind: shell_result.artifact_kind.clone(),
        outcome_status: None,
        outcome_reason: None,
    });
    Ok(())
}

async fn try_command_repair(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    command_repair_cfg: Option<&Profile>,
    task_semantics_guard_cfg: Option<&Profile>,
    objective: &str,
    purpose: &str,
    cmd: &str,
    output: &str,
    output_path_base: &PathBuf,
    shell_result: &ShellExecutionResult,
    readonly_only: bool,
    artifact_reservation: &Option<(String, PathBuf)>,
    artifact_kind: &str,
    emit_shell_output: bool,
    state: &mut ExecutionState,
) -> Result<Option<RepairResult>> {
    let Some(repair_cfg) = command_repair_cfg else {
        return Ok(None);
    };

    let repair =
        repair_command_via_unit(client, repair_cfg, objective, purpose, cmd, output).await?;

    let repaired = normalize_shell_cmd(repair.cmd.trim());
    if repaired.is_empty()
        || repaired == cmd
        || !program_safety_check(&repaired)
        || (readonly_only && !command_is_readonly(&repaired))
    {
        return Ok(None);
    }

    if let Some(guard_cfg) = task_semantics_guard_cfg {
        if let Ok(guard) = guard_repair_semantics_once(
            client, chat_url, guard_cfg, objective, purpose, cmd, &repaired, output,
        )
        .await
        {
            trace(
                args,
                &format!(
                    "repair_semantics id=shell status={} reason={}",
                    guard.status.trim(),
                    guard.reason.trim()
                ),
            );
            if !guard.status.eq_ignore_ascii_case("accept") {
                state.final_reply = Some(
                    "Explain briefly that the repaired shell command was rejected because it changed the task semantics. Ask the user to narrow or restate the request if needed."
                        .to_string(),
                );
                state.halt = true;
                let out_path = write_shell_output(&session.shell_dir, output_path_base, output)?;
                trace(args, &format!("shell_output_saved={}", out_path.display()));
                trace(args, &format!("exec_exit_code={}", shell_result.exit_code));
                // Only print to stdout when TUI is not active (TUI handles this via transcript)
                if !crate::ui_state::is_tui_active()
                    && (emit_shell_output || shell_result.exit_code != 0)
                {
                    println!("elma> exit_code={}\n{output}", shell_result.exit_code);
                }
                state
                    .artifacts
                    .insert(format!("shell:raw"), output.to_string());
                state
                    .artifacts
                    .insert("shell".to_string(), output.to_string());
                state.step_results.push(StepResult {
                    id: "shell".to_string(),
                    kind: "shell".to_string(),
                    purpose: purpose.to_string(),
                    depends_on: vec![],
                    success_condition: String::new(),
                    ok: false,
                    summary: format!(
                        "repair_rejected: {}\n{}",
                        repaired,
                        summarize_shell_output(output)
                    ),
                    command: Some(cmd.to_string()),
                    raw_output: Some(output.to_string()),
                    exit_code: Some(shell_result.exit_code),
                    output_bytes: Some(shell_result.bytes_written),
                    truncated: shell_result.truncated,
                    timed_out: shell_result.timed_out,
                    artifact_path: shell_result
                        .artifact_path
                        .as_ref()
                        .map(|p| p.display().to_string()),
                    artifact_kind: shell_result.artifact_kind.clone(),
                    outcome_status: None,
                    outcome_reason: None,
                });
                return Err(anyhow::anyhow!("repair_rejected"));
            }
        }
    }

    trace(
        args,
        &format!(
            "command_repair id=shell reason={} cmd={}",
            repair.reason.trim(),
            repaired.replace('\n', " ")
        ),
    );
    operator_trace(args, "repairing a failed shell command");
    let repair_path = write_shell_action(&session.shell_dir, &repaired)?;
    trace(args, &format!("shell_saved={}", repair_path.display()));
    shell_command_trace(args, &repaired);

    let shell_result = run_shell_one_liner(
        &repaired,
        workdir,
        artifact_reservation
            .as_ref()
            .map(|(_, path)| (path, artifact_kind)),
    )?;
    let code = shell_result.exit_code;
    let output = shell_result.inline_text.clone();

    let repaired_compatibility = probe_command_compatibility(&repaired, workdir);
    if code != 0 && command_is_unavailable(&repaired_compatibility, &output) {
        return Err(anyhow::anyhow!("repaired_command_unavailable"));
    }

    Ok(Some(RepairResult {
        code,
        output,
        output_path_base: repair_path,
        repaired_cmd: repaired,
    }))
}

fn handle_artifact_recording(
    session: &SessionPaths,
    artifact_reservation: &Option<(String, PathBuf)>,
    shell_result: &ShellExecutionResult,
    artifact_kind: &str,
    sid: &str,
    args: &Args,
) -> Result<()> {
    if let Some((artifact_id, artifact_path)) = artifact_reservation {
        if let Some(actual_path) = &shell_result.artifact_path {
            let record = ArtifactRecord {
                artifact_id: artifact_id.clone(),
                source_step_id: sid.to_string(),
                kind: shell_result
                    .artifact_kind
                    .clone()
                    .unwrap_or_else(|| artifact_kind.to_string()),
                path: actual_path.display().to_string(),
                bytes_written: shell_result.bytes_written,
                truncated: shell_result.truncated,
                timed_out: shell_result.timed_out,
                created_unix_s: now_unix_s().unwrap_or_default(),
            };
            let _ = append_artifact_manifest_record(&session.artifacts_dir, &record);
            trace(
                args,
                &format!(
                    "artifact_saved id={} path={}",
                    artifact_id,
                    artifact_path.display()
                ),
            );
        }
    }
    Ok(())
}
