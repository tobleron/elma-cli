//! @efficiency-role: service-orchestrator
//!
//! Execution Steps Shell - Execution and Post-processing

use crate::execution_steps_compat::*;
use crate::*;
use std::collections::HashSet;
use std::sync::{LazyLock, Mutex};

/// Track executed command patterns to prevent retry loops.
static EXECUTED_COMMANDS: LazyLock<Mutex<HashSet<String>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

/// Normalize a command for deduplication: lowercase, collapse whitespace, strip trailing pipes/sorts.
fn normalize_command_pattern(cmd: &str) -> String {
    let mut normalized = cmd.to_lowercase();
    // Collapse whitespace
    let mut result = String::new();
    let mut prev_space = false;
    for c in normalized.chars() {
        if c.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            result.push(c);
            prev_space = false;
        }
    }

    // For find commands, normalize aggressively to a single canonical form.
    // The model keeps generating variations (grep -v vs ! -path, -maxdepth, head -N vs head -M,
    // -exec stat vs | while read stat, xargs ls -lt, etc.) — all are the same "list files" intent.
    if result.starts_with("find ") {
        // Extract the file type being searched for
        let file_type = if result.contains("-name \"*.md\"") || result.contains("-name '*.md'") {
            "md"
        } else if result.contains("-name \"*.toml\"") || result.contains("-name '*.toml'") {
            "toml"
        } else if result.contains("-name \"*.json\"") || result.contains("-name '*.json'") {
            "json"
        } else if result.contains("-name \"*.rs\"") || result.contains("-name '*.rs'") {
            "rs"
        } else if result.contains("-name \"*.txt\"") || result.contains("-name '*.txt'") {
            "txt"
        } else {
            "other"
        };

        // Detect if stat/date/size info is being requested (any form)
        let has_stat = result.contains("stat ")
            || result.contains("-printf \"%t")
            || result.contains("-printf \"%y")
            || result.contains("-printf \"%m")
            || result.contains("ls -lt")
            || result.contains("ls -la")
            || result.contains("-printf \"%s");

        // Detect if output is being counted
        let has_wc = result.contains("| wc");

        // Detect if output is being truncated (head/tail)
        let has_head = result.contains("| head") || result.contains("|head");

        let mut key = format!("find_{}", file_type);
        if has_stat {
            key.push_str("_stat");
        }
        if has_wc {
            key.push_str("_count");
        }
        if has_head {
            key.push_str("_truncated");
        }
        return key;
    }

    // Strip trailing sort/head/tail for pattern matching (these are output formatters, not core logic)
    let stripped = result
        .split(" | ")
        .next()
        .unwrap_or(&result)
        .trim()
        .to_string();
    // Also strip the first pipe chain for find/grep commands
    let core = stripped
        .split(" | grep")
        .next()
        .unwrap_or(&stripped)
        .split(" | sort")
        .next()
        .unwrap_or(&stripped)
        .split(" | head")
        .next()
        .unwrap_or(&stripped)
        .split(" | tail")
        .next()
        .unwrap_or(&stripped)
        .split(" | wc")
        .next()
        .unwrap_or(&stripped)
        .trim()
        .to_string();
    core
}

/// Check if a command pattern has already been executed. Returns true if it's a duplicate.
fn is_duplicate_command(cmd: &str) -> bool {
    let pattern = normalize_command_pattern(cmd);
    let mut cache = EXECUTED_COMMANDS.lock().unwrap_or_else(|e| e.into_inner());
    if cache.contains(&pattern) {
        return true;
    }
    cache.insert(pattern);
    false
}

/// Clear the executed commands cache (for testing).
pub(crate) fn clear_executed_commands_cache() {
    let mut cache = EXECUTED_COMMANDS.lock().unwrap_or_else(|e| e.into_inner());
    cache.clear();
}

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

    // Prevent duplicate command execution (retry loop guard)
    if is_duplicate_command(&cmd) {
        let msg = format!(
            "SKIPPED: This command pattern was already executed in this session. Do not retry the same command — use the previous output or try a different approach."
        );
        trace(
            args,
            &format!(
                "shell_skipped_duplicate={}",
                normalize_command_pattern(&cmd)
            ),
        );
        state.step_results.push(StepResult {
            id: sid,
            kind,
            purpose: purpose_str,
            depends_on,
            success_condition,
            ok: false,
            summary: msg.clone(),
            command: Some(cmd),
            raw_output: Some(msg),
            exit_code: Some(-1),
            output_bytes: Some(0),
            truncated: false,
            timed_out: false,
            artifact_path: None,
            artifact_kind: None,
            outcome_status: None,
            outcome_reason: Some("DUPLICATE_COMMAND".to_string()),
        });
        return Ok(());
    }

    let path = write_shell_action(&session.artifacts_dir, &cmd)?;
    trace(args, &format!("shell_saved={}", path.display()));
    shell_command_trace(args, &cmd);
    let compatibility = probe_command_compatibility(&cmd, workdir);
    let mut shell_result = run_shell_persistent(&cmd, workdir).await?;
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
    let out_path = write_shell_output(&session.artifacts_dir, &output_path_base, &shell_preview)?;
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

    // Task 283: Flush shell result to session transcript and artifacts (clone before move)
    let flush_kind = kind.clone();
    let flush_sid = sid.clone();
    let flush_output = output.clone();
    let flush_ok = code == 0;

    state.step_results.push(StepResult {
        id: sid,
        kind,
        purpose: purpose_str,
        depends_on,
        success_condition,
        ok: flush_ok,
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

    crate::session_flush::flush_tool_result(
        &session.root,
        &flush_sid,
        &flush_kind,
        &flush_output,
        flush_ok,
    );

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
    let out_path = write_shell_output(&session.artifacts_dir, output_path_base, &shell_preview)?;
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
                let out_path =
                    write_shell_output(&session.artifacts_dir, output_path_base, output)?;
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
    let repair_path = write_shell_action(&session.artifacts_dir, &repaired)?;
    trace(args, &format!("shell_saved={}", repair_path.display()));
    shell_command_trace(args, &repaired);

    let shell_result = run_shell_persistent(&repaired, workdir).await?;
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

/// Reserve an artifact path under the artifacts directory.
fn reserve_artifact_path(
    artifacts_dir: &PathBuf,
    kind: &str,
    ext: &str,
) -> Result<(String, PathBuf)> {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let id = format!("{}_{}", kind, ts);
    let path = artifacts_dir.join(format!("{}.{}", id, ext));
    Ok((id, path))
}

/// Append a record to the artifact manifest (session.json).
fn append_artifact_manifest_record(session_root: &PathBuf, record: &ArtifactRecord) -> Result<()> {
    let path = session_root.join("session.json");
    let mut session_data: serde_json::Value = if path.exists() {
        let raw = std::fs::read_to_string(&path)?;
        serde_json::from_str(&raw).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if session_data.get("artifacts").is_none() {
        session_data["artifacts"] = serde_json::json!([]);
    }
    let artifacts = session_data.get_mut("artifacts").unwrap();
    if let Some(arr) = artifacts.as_array_mut() {
        arr.push(serde_json::json!({
            "artifact_id": record.artifact_id,
            "source_step_id": record.source_step_id,
            "kind": record.kind,
            "path": record.path,
            "bytes_written": record.bytes_written,
            "truncated": record.truncated,
            "timed_out": record.timed_out,
            "created_unix_s": record.created_unix_s,
        }));
    }
    let json = serde_json::to_string_pretty(&session_data)?;
    std::fs::write(&path, json)?;
    Ok(())
}
