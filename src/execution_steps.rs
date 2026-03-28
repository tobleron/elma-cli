use crate::execution::ExecutionState;
use crate::*;

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
    summarizer_cfg: &Profile,
    command_repair_cfg: Option<&Profile>,
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
                .and_then(|c| {
                    c.message
                        .content
                        .clone()
                        .or(c.message.reasoning_content.clone())
                })
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
            });
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
                .and_then(|c| {
                    c.message
                        .content
                        .clone()
                        .or(c.message.reasoning_content.clone())
                })
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
                .and_then(|c| {
                    c.message
                        .content
                        .clone()
                        .or(c.message.reasoning_content.clone())
                })
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
                .and_then(|c| {
                    c.message
                        .content
                        .clone()
                        .or(c.message.reasoning_content.clone())
                })
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
            });
        }
    }

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
    let cmd = normalize_shell_cmd(&cmd);
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
        });
        return Ok(());
    }

    let path = write_shell_action(&session.shell_dir, &cmd)?;
    trace(args, &format!("shell_saved={}", path.display()));
    shell_command_trace(args, &cmd);
    let (mut code, mut output) = run_shell_one_liner(&cmd, workdir)?;
    let mut output_path_base = path.clone();
    let mut repaired_cmd: Option<String> = None;

    if code != 0 {
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
                    let (repair_code, repair_output) = run_shell_one_liner(&repaired, workdir)?;
                    code = repair_code;
                    output = repair_output;
                    repaired_cmd = Some(repaired);
                }
            }
        }
    }

    let out_path = write_shell_output(&session.shell_dir, &output_path_base, &output)?;
    trace(args, &format!("shell_output_saved={}", out_path.display()));
    trace(args, &format!("exec_exit_code={code}"));
    if emit_shell_output || code != 0 {
        println!("elma> exit_code={code}\n{output}");
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
        state.artifacts.insert(sid.clone(), output.clone());
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
    });
    Ok(())
}
