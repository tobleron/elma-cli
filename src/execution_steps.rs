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

fn normalize_selected_items_against_evidence(
    items: Vec<String>,
    instructions: &str,
    evidence: &str,
) -> Vec<String> {
    let evidence_lines = evidence
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let require_exact_path = instructions
        .to_ascii_lowercase()
        .contains("exact relative path")
        || instructions
            .to_ascii_lowercase()
            .contains("exact file path")
        || instructions
            .to_ascii_lowercase()
            .contains("exact grounded relative file paths");

    items
        .into_iter()
        .map(|item| {
            let mut trimmed = item.trim();
            // Aggressively trim common non-path separators that might be hallucinated/captured
            trimmed = trimmed.trim_matches(|c: char| {
                matches!(
                    c,
                    '"' | '\'' | '`' | ',' | ';' | ':' | ')' | ']' | '}' | '*'
                )
            });
            // Also trim a trailing dot if it's Not part of an extension (e.g. "main.go.")
            if trimmed.ends_with('.')
                && !trimmed.ends_with("..")
                && trimmed.matches('.').count() > 1
            {
                trimmed = trimmed.trim_end_matches('.');
            }

            let matches = evidence_lines
                .iter()
                .filter(|line| {
                    // Suffix match requirement
                    let ends_with = line.ends_with(trimmed);
                    if !ends_with {
                        return false;
                    }

                    // Separator requirement (must be a path component match)
                    let len_diff = line.len().saturating_sub(trimmed.len());
                    if len_diff == 0 {
                        return false;
                    } // Handled by exact match above

                    let separator_ok = line.as_bytes().get(len_diff - 1) == Some(&b'/');
                    if !separator_ok {
                        return false;
                    }

                    // Strict path prefix requirement: no spaces or other non-path characters
                    // to avoid matching human sentences that happens to end with the path.
                    let prefix = &line[..len_diff - 1]; // up to the '/'
                    let is_vaguely_path_like = !prefix.is_empty()
                        && prefix.chars().all(|c| {
                            c.is_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-'
                        });
                    is_vaguely_path_like
                })
                .copied()
                .collect::<Vec<_>>();

            // Prefer restoring grounded full paths before accepting a basename that also
            // appears verbatim in evidence (e.g. `ls` output alongside `rg --files` output).
            if matches.len() == 1 {
                return matches[0].to_string();
            }

            if matches.len() > 1 {
                let mut sorted = matches.clone();
                sorted.sort_by_key(|a| a.matches('/').count());
                return sorted[0].to_string();
            }

            if evidence_lines.iter().any(|line| *line == trimmed) {
                if require_exact_path && !trimmed.contains('/') {
                    return trimmed.to_string();
                }
                return trimmed.to_string();
            }

            trimmed.to_string()
        })
        .collect()
}

async fn select_items_via_unit(
    client: &reqwest::Client,
    selector_cfg: &Profile,
    unit_type: Option<&str>,
    objective: &str,
    purpose: &str,
    instructions: &str,
    evidence: &str,
) -> Result<SelectionOutput> {
    fn budget_selection_evidence(evidence: &str) -> String {
        const MAX_LINES: usize = 120;
        const MAX_CHARS: usize = 12_000;

        let mut text = evidence
            .lines()
            .take(MAX_LINES)
            .collect::<Vec<_>>()
            .join("\n");
        if text.chars().count() > MAX_CHARS {
            text = text.chars().take(MAX_CHARS).collect::<String>();
        }
        text
    }

    if evidence.trim().is_empty() {
        return Ok(SelectionOutput {
            items: Vec::new(),
            reason: "Upstream evidence is empty; skipping selection to prevent hallucination."
                .to_string(),
        });
    }

    if evidence.contains("EXECUTION FAILED") || evidence.contains("command not found") {
        return Ok(SelectionOutput {
            items: Vec::new(),
            reason: "Upstream execution failed; skipping selection to prevent hallucination."
                .to_string(),
        });
    }

    let evidence = budget_selection_evidence(evidence);
    let context = IntelContext::new(
        objective.to_string(),
        neutral_route_decision(),
        evidence.clone(),
        String::new(),
        Vec::new(),
        client.clone(),
    )
    .with_extra("purpose", purpose)?
    .with_extra("instructions", instructions)?
    .with_extra("evidence", &evidence)?;
    match unit_type.unwrap_or("selector") {
        "rename_suggester" => {
            let mut profile = selector_cfg.clone();
            profile.name = "rename_suggester".to_string();
            if let Some(prompt) = canonical_system_prompt("rename_suggester") {
                profile.system_prompt = prompt.to_string();
            }
            profile.temperature = profile.temperature.clamp(0.2, 0.35);
            profile.max_tokens = profile.max_tokens.min(120);
            let unit = RenameSuggesterUnit::new(profile);
            let output = unit.execute_with_fallback(&context).await?;
            let rename: RenameSuggestion = serde_json::from_value(output.data)
                .map_err(|e| anyhow::anyhow!("Failed to parse rename suggester output: {}", e))?;
            Ok(SelectionOutput {
                items: if rename.identifier.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![rename.identifier.trim().to_string()]
                },
                reason: rename.reason,
            })
        }
        _ => {
            let unit = SelectorUnit::new(selector_cfg.clone());
            let output = unit.execute_with_fallback(&context).await?;
            let mut selection: SelectionOutput = serde_json::from_value(output.data)
                .map_err(|e| anyhow::anyhow!("Failed to parse selector output: {}", e))?;
            selection.items =
                normalize_selected_items_against_evidence(selection.items, instructions, &evidence);
            Ok(selection)
        }
    }
}

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
            common,
            ..
        } => {
            let evidence = depends_on
                .iter()
                .filter_map(|dep| state.artifacts.get(dep))
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n");
            let selection = select_items_via_unit(
                client,
                selector_cfg,
                common.unit_type.as_deref(),
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
                    "selection id={sid} count={} items={:?} reason={}",
                    items.len(),
                    items,
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

#[cfg(test)]
mod tests {
    use super::normalize_selected_items_against_evidence;

    #[test]
    fn selector_normalizes_unique_suffix_to_exact_relative_path() {
        let evidence =
            "_stress_testing/_opencode_for_testing/main.go\n_stress_testing/_opencode_for_testing/cmd/root.go";
        let instructions =
            "Choose exactly one most likely primary entry point. Return the exact relative path only.";
        let items = normalize_selected_items_against_evidence(
            vec!["main.go".to_string()],
            instructions,
            evidence,
        );
        assert_eq!(items, vec!["_stress_testing/_opencode_for_testing/main.go"]);
    }

    #[test]
    fn selector_prefers_shallow_grounded_path_when_basename_is_ambiguous() {
        let evidence = "_stress_testing/_opencode_for_testing/main.go\n_stress_testing/_opencode_for_testing/cmd/root.go\n_stress_testing/_opencode_for_testing/cmd/schema/main.go";
        let instructions =
            "Choose exactly one most likely primary entry point. Return the exact relative path only.";
        let items = normalize_selected_items_against_evidence(
            vec!["main.go".to_string()],
            instructions,
            evidence,
        );
        assert_eq!(items, vec!["_stress_testing/_opencode_for_testing/main.go"]);
    }
}
