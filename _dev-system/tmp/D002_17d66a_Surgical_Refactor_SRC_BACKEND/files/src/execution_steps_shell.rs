//! @efficiency-role: service-orchestrator
//!
//! Execution Steps - Shell Step Handling

use crate::execution_steps_compat::*;
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
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
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
            id: sid,
            kind,
            purpose,
            depends_on,
            success_condition,
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
            id: sid,
            kind,
            purpose,
            depends_on,
            success_condition,
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
            &purpose,
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
                            &purpose,
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
                state.artifacts.insert(
                    sid.clone(),
                    user_reason.to_string(),
                );
                state.step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: false,
                    summary: format!(
                        "preflight_rejected: {}",
                        user_reason
                    ),
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
        }
    }
    if execution_mode.eq_ignore_ascii_case("ASK") {
        let ask_reason = if ask_hint.trim().is_empty() {
            "The shell action needs narrowing before execution.".to_string()
        } else {
            ask_hint
        };
        state.final_reply = Some(
            format!(
                "Explain briefly that the shell action was not executed yet. Reason: {}. Ask one concise clarifying question or offer a compatible alternative if that would help.",
                ask_reason
            ),
        );
        state.halt = true;
        state.artifacts.insert(
            sid.clone(),
            ask_reason.clone(),
        );
        state.step_results.push(StepResult {
            id: sid,
            kind,
            purpose,
            depends_on,
            success_condition,
            ok: false,
            summary: format!("preflight_requires_clarification: {ask_reason}"),
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
            let shell_preview = if let Some(path) = shell_result.artifact_path.as_ref() {
                format!("{}\n[artifact: {}]", output.trim_end(), path.display())
            } else {
                output.clone()
            };
            let out_path =
                write_shell_output(&session.shell_dir, &output_path_base, &shell_preview)?;
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
            trace(args, &format!("exec_exit_code={code}"));
            if emit_shell_output || code != 0 {
                println!("elma> exit_code={code}\n{shell_preview}");
            }
            state.final_reply = Some(unavailable_reply_instructions(&compatibility));
            state.halt = true;
            state.artifacts.insert(format!("{sid}:raw"), output.clone());
            state.artifacts.insert(sid.clone(), output.clone());
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: false,
                summary: unavailable_summary(&compatibility, &output),
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
                artifact_kind: shell_result.artifact_kind.clone(),
                outcome_status: None,
                outcome_reason: None,
            });
            return Ok(());
        }
        if let Some(repair_cfg) = command_repair_cfg {
            if let Ok(repair) = repair_command_once(
                client, chat_url, repair_cfg, objective, &purpose, &cmd, &output,
            )
            .await
            {
                let repaired = normalize_shell_cmd(repair.cmd.trim());
                if !repaired.is_empty()
                    && repaired != cmd
                    && program_safety_check(&repaired)
                    && (!readonly_only || command_is_readonly(&repaired))
                {
                    if let Some(guard_cfg) = task_semantics_guard_cfg {
                        if let Ok(guard) = guard_repair_semantics_once(
                            client,
                            chat_url,
                            guard_cfg,
                            objective,
                            &purpose,
                            &cmd,
                            &repaired,
                            &output,
                        )
                        .await
                        {
                            trace(
                                args,
                                &format!(
                                    "repair_semantics id={sid} status={} reason={}",
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
                                let out_path =
                                    write_shell_output(&session.shell_dir, &output_path_base, &output)?;
                                trace(args, &format!("shell_output_saved={}", out_path.display()));
                                trace(args, &format!("exec_exit_code={code}"));
                                if emit_shell_output || code != 0 {
                                    println!("elma> exit_code={code}\n{output}");
                                }
                                state.artifacts.insert(format!("{sid}:raw"), output.clone());
                                state.artifacts.insert(sid.clone(), output.clone());
                                state.step_results.push(StepResult {
                                    id: sid,
                                    kind,
                                    purpose,
                                    depends_on,
                                    success_condition,
                                    ok: false,
                                    summary: format!(
                                        "repair_rejected: {}\n{}",
                                        repaired,
                                        summarize_shell_output(&output)
                                    ),
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
                                    artifact_kind: shell_result.artifact_kind.clone(),
                                    outcome_status: None,
                                    outcome_reason: None,
                                });
                                return Ok(());
                            }
                        }
                    }
                    trace(
                        args,
                        &format!(
                            "command_repair id={sid} reason={} cmd={}",
                            repair.reason.trim(),
                            repaired.replace('\n', " ")
                        ),
                    );
                    operator_trace(args, "repairing a failed shell command");
                    let repair_path = write_shell_action(&session.shell_dir, &repaired)?;
                    trace(args, &format!("shell_saved={}", repair_path.display()));
                    shell_command_trace(args, &repaired);
                    shell_result = run_shell_one_liner(
                        &repaired,
                        workdir,
                        artifact_reservation
                            .as_ref()
                            .map(|(_, path)| (path, artifact_kind.as_str())),
                    )?;
                    code = shell_result.exit_code;
                    output = shell_result.inline_text.clone();
                    repaired_cmd = Some(repaired);
                    let repaired_compatibility =
                        probe_command_compatibility(repaired_cmd.as_deref().unwrap_or(&cmd), workdir);
                    if code != 0 && command_is_unavailable(&repaired_compatibility, &output) {
                        let shell_preview = if let Some(path) = shell_result.artifact_path.as_ref() {
                            format!("{}\n[artifact: {}]", output.trim_end(), path.display())
                        } else {
                            output.clone()
                        };
                        let out_path = write_shell_output(
                            &session.shell_dir,
                            &output_path_base,
                            &shell_preview,
                        )?;
                        trace(args, &format!("shell_output_saved={}", out_path.display()));
                        trace(
                            args,
                            &format!(
                                "command_unavailable id={sid} bin={} os={} shell={}",
                                repaired_compatibility.primary_bin.trim(),
                                repaired_compatibility.os_family.trim(),
                                repaired_compatibility.shell_path.trim()
                            ),
                        );
                        trace(args, &format!("exec_exit_code={code}"));
                        if emit_shell_output || code != 0 {
                            println!("elma> exit_code={code}\n{shell_preview}");
                        }
                        state.final_reply = Some(unavailable_reply_instructions(
                            &repaired_compatibility,
                        ));
                        state.halt = true;
                        state.artifacts.insert(format!("{sid}:raw"), output.clone());
                        state.artifacts.insert(sid.clone(), output.clone());
                        state.step_results.push(StepResult {
                            id: sid,
                            kind,
                            purpose,
                            depends_on,
                            success_condition,
                            ok: false,
                            summary: unavailable_summary(&repaired_compatibility, &output),
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
                            artifact_kind: shell_result.artifact_kind.clone(),
                            outcome_status: None,
                            outcome_reason: None,
                        });
                        return Ok(());
                    }
                }
            }
        }
    }

    if let Some((artifact_id, artifact_path)) = &artifact_reservation {
        if let Some(actual_path) = &shell_result.artifact_path {
            let record = ArtifactRecord {
                artifact_id: artifact_id.clone(),
                source_step_id: sid.clone(),
                kind: shell_result
                    .artifact_kind
                    .clone()
                    .unwrap_or_else(|| artifact_kind.clone()),
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

    let shell_preview = if let Some(path) = shell_result.artifact_path.as_ref() {
        format!(
            "{}\n[artifact: {}]",
            output.trim_end(),
            path.display()
        )
    } else {
        output.clone()
    };
    let out_path = write_shell_output(&session.shell_dir, &output_path_base, &shell_preview)?;
    trace(args, &format!("shell_output_saved={}", out_path.display()));
    trace(args, &format!("exec_exit_code={code}"));
    if emit_shell_output || code != 0 {
        println!("elma> exit_code={code}\n{shell_preview}");
    }
    state.artifacts.insert(format!("{sid}:raw"), output.clone());

    let mut compact_summary = summarize_shell_output(&output);
    if let Some(compactor_cfg) = evidence_compactor_cfg {
        if let Ok(compact) = compact_evidence_once(
            client,
            chat_url,
            compactor_cfg,
            objective,
            &purpose,
            scope,
            repaired_cmd.as_deref().unwrap_or(&cmd),
            &output,
        )
        .await
        {
            let compact_text = summarize_evidence_compact(&compact);
            if !compact_text.trim().is_empty() {
                compact_summary = compact_text.clone();
                state.artifacts.insert(sid.clone(), compact_text);
            }
        }
    }
    if !state.artifacts.contains_key(&sid) {
        if let Some(path) = shell_result.artifact_path.as_ref() {
            state.artifacts.insert(
                sid.clone(),
                format!("{}\nartifact: {}", output.trim_end(), path.display()),
            );
        } else {
            state.artifacts.insert(sid.clone(), output.clone());
        }
    }
    if let Some(classifier_cfg) = artifact_classifier_cfg {
        if should_classify_artifacts(complexity, formula) {
            if let Ok(classification) = classify_artifacts_once(
                client,
                chat_url,
                classifier_cfg,
                objective,
                scope,
                state.artifacts.get(&sid).map(String::as_str).unwrap_or(&output),
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
        purpose,
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
