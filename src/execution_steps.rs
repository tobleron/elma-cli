use crate::execution::ExecutionState;
use crate::*;

#[derive(Debug, Clone)]
struct CommandCompatibilityFacts {
    primary_bin: String,
    command_exists: bool,
    command_lookup: String,
    os_family: String,
    shell_path: String,
}

fn primary_shell_command(cmd: &str) -> String {
    let mut rest = cmd.trim();
    while !rest.is_empty() {
        let token = rest.split_whitespace().next().unwrap_or("").trim();
        if token.is_empty() {
            break;
        }
        rest = rest[token.len()..].trim_start();
        let stripped = token.trim_matches(|c| c == '"' || c == '\'');
        if stripped.eq_ignore_ascii_case("env") {
            continue;
        }
        let looks_like_assignment = stripped.contains('=')
            && !stripped.starts_with('/')
            && stripped
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic() || c == '_')
                .unwrap_or(false);
        if looks_like_assignment {
            continue;
        }
        return stripped.rsplit('/').next().unwrap_or(stripped).to_string();
    }
    String::new()
}

fn probe_command_compatibility(cmd: &str, workdir: &Path) -> CommandCompatibilityFacts {
    let primary_bin = primary_shell_command(cmd);
    let os_family = std::env::consts::OS.to_string();
    let shell_path = std::env::var("SHELL").unwrap_or_default();
    if primary_bin.is_empty() {
        return CommandCompatibilityFacts {
            primary_bin,
            command_exists: true,
            command_lookup: String::new(),
            os_family,
            shell_path,
        };
    }

    let probe = Command::new("sh")
        .current_dir(workdir)
        .arg("-lc")
        .arg(format!("command -v -- {}", shell_quote(&primary_bin)))
        .output();

    match probe {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            CommandCompatibilityFacts {
                primary_bin,
                command_exists: output.status.success(),
                command_lookup: if !stdout.is_empty() { stdout } else { stderr },
                os_family,
                shell_path,
            }
        }
        Err(error) => CommandCompatibilityFacts {
            primary_bin,
            command_exists: false,
            command_lookup: error.to_string(),
            os_family,
            shell_path,
        },
    }
}

fn shell_output_indicates_command_not_found(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("command not found")
        || lower.contains("not recognized as an internal or external command")
}

fn command_is_unavailable(facts: &CommandCompatibilityFacts, output: &str) -> bool {
    (!facts.primary_bin.trim().is_empty() && !facts.command_exists)
        || shell_output_indicates_command_not_found(output)
}

fn unavailable_summary(facts: &CommandCompatibilityFacts, output: &str) -> String {
    let mut parts = vec![format!(
        "command_unavailable: {} on {}",
        if facts.primary_bin.trim().is_empty() {
            "unknown-command"
        } else {
            facts.primary_bin.trim()
        },
        facts.os_family.trim()
    )];
    if !facts.command_lookup.trim().is_empty() {
        parts.push(format!("lookup: {}", facts.command_lookup.trim()));
    }
    if !output.trim().is_empty() {
        parts.push(output.trim().to_string());
    }
    parts.join("\n")
}

fn unavailable_reply_instructions(facts: &CommandCompatibilityFacts) -> String {
    format!(
        "Explain briefly that the command `{}` is not available in this environment (os: {}, shell: {}). Do not retry the same command. If the user's underlying goal could be met with a platform-appropriate alternative, mention that and ask whether to use it.",
        if facts.primary_bin.trim().is_empty() {
            "the requested command"
        } else {
            facts.primary_bin.trim()
        },
        facts.os_family.trim(),
        if facts.shell_path.trim().is_empty() {
            "unknown"
        } else {
            facts.shell_path.trim()
        }
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_program_step(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    planner_cfg: &Profile,
    planner_master_cfg: &Profile,
    decider_cfg: &Profile,
    selector_cfg: &Profile,
    summarizer_cfg: &Profile,
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
    step: Step,
    state: &mut ExecutionState,
) -> Result<()> {
    let sid = step_id(&step).to_string();
    let kind = step_kind(&step).to_string();
    let purpose = step_purpose(&step);
    let depends_on = step_depends_on(&step);
    let success_condition = step_success_condition(&step);
    trace(
        args,
        &format!(
            "step id={sid} type={kind} purpose={} depends_on={}",
            purpose,
            if depends_on.is_empty() {
                "-".to_string()
            } else {
                depends_on.join(",")
            }
        ),
    );
    if !matches!(step, Step::Reply { .. }) {
        operator_trace(args, &purpose);
    }

    match step {
        Step::Shell { id: _, cmd, .. } => {
            handle_shell_step(
                args,
                client,
                chat_url,
                session,
                workdir,
                command_repair_cfg,
                command_preflight_cfg,
                task_semantics_guard_cfg,
                evidence_compactor_cfg,
                artifact_classifier_cfg,
                scope,
                complexity,
                formula,
                objective,
                emit_shell_output,
                readonly_only,
                sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                cmd,
                state,
            )
            .await?;
        }
        Step::Select {
            id: _,
            instructions,
            ..
        } => {
            let evidence = depends_on
                .iter()
                .filter_map(|dep| state.artifacts.get(dep))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            let selection = select_items_once(
                client,
                chat_url,
                selector_cfg,
                objective,
                &purpose,
                &instructions,
                &evidence,
            )
            .await?;
            let items = selection
                .items
                .into_iter()
                .map(|item| item.trim().to_string())
                .filter(|item| !item.is_empty())
                .collect::<Vec<_>>();
            let selection_text = items.join("\n");
            state.artifacts.insert(sid.clone(), selection_text.clone());
            trace(
                args,
                &format!(
                    "selection id={sid} count={} reason={}",
                    items.len(),
                    selection.reason.trim()
                ),
            );
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: !items.is_empty(),
                summary: if items.is_empty() {
                    format!("selection_empty: {}", selection.reason.trim())
                } else {
                    format!(
                        "selected {} item(s)\n{}",
                        items.len(),
                        preview_text(&selection_text, 8)
                    )
                },
                command: None,
                raw_output: Some(selection_text),
                exit_code: None,
                output_bytes: None,
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: Some("selection".to_string()),
                outcome_status: None,
                outcome_reason: None,
            });
            if items.is_empty() {
                state.halt = true;
            }
        }
        Step::Summarize {
            id: _,
            mut text,
            instructions,
            ..
        } => {
            if text.trim().is_empty() && !depends_on.is_empty() {
                text = depends_on
                    .iter()
                    .filter_map(|dep| state.artifacts.get(dep))
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n\n");
            }
            let sum_req = ChatCompletionRequest {
                model: summarizer_cfg.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: summarizer_cfg.system_prompt.clone(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!("Instructions:\n{}\n\nText:\n{}", instructions.trim(), text),
                    },
                ],
                temperature: summarizer_cfg.temperature,
                top_p: summarizer_cfg.top_p,
                stream: false,
                max_tokens: summarizer_cfg.max_tokens,
                n_probs: None,
                repeat_penalty: Some(summarizer_cfg.repeat_penalty),
                reasoning_format: Some(summarizer_cfg.reasoning_format.clone()),
            };
            let sum_resp = chat_once(client, chat_url, &sum_req).await?;
            let sum_text = sum_resp
                .choices
                .get(0)
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default()
                .trim()
                .to_string();
            state.artifacts.insert(sid.clone(), sum_text.clone());
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: !sum_text.is_empty(),
                summary: sum_text,
                command: None,
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
        }
        Step::Edit { id: _, spec, .. } => {
            handle_edit_step(
                args,
                session,
                workdir,
                sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                spec,
                state,
            )?;
        }
        Step::Plan { id: _, goal, .. } => {
            let req = ChatCompletionRequest {
                model: planner_cfg.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: planner_cfg.system_prompt.clone(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!(
                            "Goal:\n{goal}\n\nMaster plan (_master.md):\n{}",
                            std::fs::read_to_string(session.plans_dir.join("_master.md"))
                                .unwrap_or_default()
                        ),
                    },
                ],
                temperature: planner_cfg.temperature,
                top_p: planner_cfg.top_p,
                stream: false,
                max_tokens: planner_cfg.max_tokens,
                n_probs: None,
                repeat_penalty: Some(planner_cfg.repeat_penalty),
                reasoning_format: Some(planner_cfg.reasoning_format.clone()),
            };
            let resp = chat_once(client, chat_url, &req).await?;
            let text = resp
                .choices
                .get(0)
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let plan_path = write_plan_file(&session.plans_dir, &(text.trim().to_string() + "\n"))?;
            append_master_link(&session.plans_dir, &plan_path, &goal)?;
            trace(args, &format!("plan_saved={}", plan_path.display()));
            state.artifacts.insert(sid.clone(), text.trim().to_string());
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: true,
                summary: format!("saved {}\n{}", plan_path.display(), preview_text(text.trim(), 8)),
                command: None,
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
        }
        Step::MasterPlan { id: _, goal, .. } => {
            let req = ChatCompletionRequest {
                model: planner_master_cfg.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: planner_master_cfg.system_prompt.clone(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!("Goal:\n{goal}\n\nUpdate the master plan."),
                    },
                ],
                temperature: planner_master_cfg.temperature,
                top_p: planner_master_cfg.top_p,
                stream: false,
                max_tokens: planner_master_cfg.max_tokens,
                n_probs: None,
                repeat_penalty: Some(planner_master_cfg.repeat_penalty),
                reasoning_format: Some(planner_master_cfg.reasoning_format.clone()),
            };
            let resp = chat_once(client, chat_url, &req).await?;
            let text = resp
                .choices
                .get(0)
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let path = session.plans_dir.join("_master.md");
            std::fs::write(&path, squash_blank_lines(text.trim()).trim().to_string() + "\n")
                .with_context(|| format!("write {}", path.display()))?;
            trace(args, &format!("masterplan_saved={}", path.display()));
            state.artifacts.insert(sid.clone(), text.trim().to_string());
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: true,
                summary: format!("saved {}\n{}", path.display(), preview_text(text.trim(), 8)),
                command: None,
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
        }
        Step::Decide { id: _, prompt, .. } => {
            let req = ChatCompletionRequest {
                model: decider_cfg.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: decider_cfg.system_prompt.clone(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: prompt,
                    },
                ],
                temperature: decider_cfg.temperature,
                top_p: decider_cfg.top_p,
                stream: false,
                max_tokens: decider_cfg.max_tokens,
                n_probs: None,
                repeat_penalty: Some(decider_cfg.repeat_penalty),
                reasoning_format: Some(decider_cfg.reasoning_format.clone()),
            };
            let resp = chat_once(client, chat_url, &req).await?;
            let word = resp
                .choices
                .get(0)
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let word = word
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .to_string();
            let path = write_decision(&session.decisions_dir, &word)?;
            trace(args, &format!("decision_saved={}", path.display()));
            state.artifacts.insert(sid.clone(), word.clone());
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: true,
                summary: word,
                command: None,
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
        }
        Step::Reply {
            id: _,
            instructions,
            ..
        } => {
            state.final_reply = Some(instructions.clone());
            state.artifacts.insert(sid.clone(), instructions);
            state.step_results.push(StepResult {
                id: sid,
                kind,
                purpose,
                depends_on,
                success_condition,
                ok: true,
                summary: "reply".to_string(),
                command: None,
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
        }
    }

    Ok(())
}

fn handle_edit_step(
    args: &Args,
    session: &SessionPaths,
    workdir: &PathBuf,
    sid: String,
    kind: String,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    spec: EditSpec,
    state: &mut ExecutionState,
) -> Result<()> {
    let operation = spec.operation.trim();
    if !edit_operation_is_supported(operation) {
        state.step_results.push(StepResult {
            id: sid,
            kind,
            purpose,
            depends_on,
            success_condition,
            ok: false,
            summary: format!("unsupported edit operation: {}", spec.operation.trim()),
            command: None,
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

    let snapshot_id = if let Some(existing) = state.auto_snapshot_id.clone() {
        Some(existing)
    } else {
        let reason = if purpose.trim().is_empty() {
            format!("automatic pre-edit snapshot for {}", spec.path.trim())
        } else {
            format!("automatic pre-edit snapshot before {}", purpose.trim())
        };
        match create_workspace_snapshot(session, workdir, &reason, true) {
            Ok(snapshot) => {
                trace(
                    args,
                    &format!(
                        "snapshot_saved id={} path={} files={} automatic={}",
                        snapshot.snapshot_id,
                        snapshot.snapshot_dir.display(),
                        snapshot.file_count,
                        snapshot.automatic
                    ),
                );
                state.auto_snapshot_id = Some(snapshot.snapshot_id.clone());
                Some(snapshot.snapshot_id)
            }
            Err(error) => {
                state.halt = true;
                state.step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: false,
                    summary: format!("snapshot_failed: {error}"),
                    command: None,
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
    };

    let path = resolve_workspace_edit_path(workdir, &spec.path)?;
    let parent = path
        .parent()
        .context("edit target has no parent directory")?;
    std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;

    let action_summary = match operation {
        "write_file" => {
            std::fs::write(&path, spec.content.as_bytes())
                .with_context(|| format!("write {}", path.display()))?;
            format!("wrote {}", path.display())
        }
        "append_text" => {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .with_context(|| format!("open {}", path.display()))?;
            file.write_all(spec.content.as_bytes())
                .with_context(|| format!("append {}", path.display()))?;
            format!("appended {}", path.display())
        }
        "replace_text" => {
            let original =
                std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
            if spec.find.is_empty() {
                anyhow::bail!("replace_text requires non-empty find");
            }
            let replaced = original.replace(&spec.find, &spec.replace);
            if replaced == original {
                anyhow::bail!("replace_text found no matches in {}", path.display());
            }
            std::fs::write(&path, replaced.as_bytes())
                .with_context(|| format!("write {}", path.display()))?;
            format!("updated {}", path.display())
        }
        _ => unreachable!(),
    };
    let summary = if let Some(snapshot_id) = snapshot_id.as_deref() {
        format!("{action_summary} (snapshot {snapshot_id})")
    } else {
        action_summary
    };

    trace(
        args,
        &format!(
            "edit_saved path={} operation={}",
            path.display(),
            operation
        ),
    );
    state.artifacts.insert(
        sid.clone(),
        format!(
            "{}\noperation: {}\npath: {}{}",
            summary,
            operation,
            path.display(),
            snapshot_id
                .as_deref()
                .map(|id| format!("\nsnapshot: {id}"))
                .unwrap_or_default()
        ),
    );
    state.step_results.push(StepResult {
        id: sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        ok: true,
        summary,
        command: None,
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
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_shell_step(
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
                    output_path_base = repair_path;
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
