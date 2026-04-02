//! @efficiency-role: service-orchestrator
//!
//! Execution Steps Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - execution_steps_compat: Command compatibility and probing
//! - execution_steps_edit: Edit step handling
//! - execution_steps_shell: Shell step handling
//! - execution_steps_read: Read step handling
//! - execution_steps_search: Search step handling

use crate::execution::ExecutionState;
use crate::*;

pub(crate) use execution_steps_compat::*;
pub(crate) use execution_steps_edit::*;
pub(crate) use execution_steps_read::*;
pub(crate) use execution_steps_search::*;
pub(crate) use execution_steps_shell::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_program_step(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    status_message_cfg: &Profile,
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
        Step::Read { id: _, path, .. } => {
            handle_read_step(
                args,
                session,
                workdir,
                sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                &path,
                state,
            )
            .await?;
        }
        Step::Search {
            id: _,
            query,
            paths,
            ..
        } => {
            handle_search_step(
                args,
                session,
                workdir,
                sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                &query,
                paths,
                state,
            )
            .await?;
        }
        Step::Shell { id: _, cmd, .. } => {
            handle_shell_step(
                args,
                client,
                chat_url,
                session,
                workdir,
                status_message_cfg,
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
                        content: format!(
                            "Instructions:\n{}\n\nText:\n{}",
                            instructions.trim(),
                            text
                        ),
                    },
                ],
                temperature: summarizer_cfg.temperature,
                top_p: summarizer_cfg.top_p,
                stream: false,
                max_tokens: summarizer_cfg.max_tokens,
                n_probs: None,
                repeat_penalty: Some(summarizer_cfg.repeat_penalty),
                reasoning_format: Some(summarizer_cfg.reasoning_format.clone()),
                grammar: None,
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
                grammar: None,
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
                summary: format!(
                    "saved {}\n{}",
                    plan_path.display(),
                    preview_text(text.trim(), 8)
                ),
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
                grammar: None,
            };
            let resp = chat_once(client, chat_url, &req).await?;
            let text = resp
                .choices
                .get(0)
                .and_then(|c| c.message.content.clone())
                .unwrap_or_default();
            let path = session.plans_dir.join("_master.md");
            std::fs::write(
                &path,
                squash_blank_lines(text.trim()).trim().to_string() + "\n",
            )
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
                grammar: None,
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
