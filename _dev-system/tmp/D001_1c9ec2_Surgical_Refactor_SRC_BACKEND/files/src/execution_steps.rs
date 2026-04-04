//! @efficiency-role: orchestrator
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

fn gather_artifacts(
    depends_on: &[String],
    artifacts: &std::collections::HashMap<String, String>,
) -> String {
    depends_on
        .iter()
        .filter_map(|dep| artifacts.get(dep))
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn normalize_selected_items_against_evidence(
    items: Vec<String>,
    _instructions: &str,
    evidence: &str,
) -> Vec<String> {
    let evidence_lines = evidence
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>();
    items
        .into_iter()
        .map(|item| normalize_single_item(item.trim(), &evidence_lines))
        .collect()
}

fn normalize_single_item(trimmed: &str, evidence_lines: &[&str]) -> String {
    let mut trimmed = trimmed.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | ';' | ':' | ')' | ']' | '}' | '*'
        )
    });
    if trimmed.ends_with('.') && !trimmed.ends_with("..") && trimmed.matches('.').count() > 1 {
        trimmed = trimmed.trim_end_matches('.');
    }
    let matches = evidence_lines
        .iter()
        .filter(|line| is_relative_path_match(line, trimmed))
        .copied()
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        return matches[0].to_string();
    }
    if matches.len() > 1 {
        let mut sorted = matches.clone();
        sorted.sort_by_key(|a| a.matches('/').count());
        return sorted[0].to_string();
    }
    if evidence_lines.iter().any(|line| *line == trimmed) {
        return trimmed.to_string();
    }
    trimmed.to_string()
}

fn is_relative_path_match(line: &str, trimmed: &str) -> bool {
    let ends_with = line.ends_with(trimmed);
    if !ends_with {
        return false;
    }
    let len_diff = line.len().saturating_sub(trimmed.len());
    if len_diff == 0 {
        return false;
    }
    let separator_ok = line.as_bytes().get(len_diff - 1) == Some(&b'/');
    if !separator_ok {
        return false;
    }
    let prefix = &line[..len_diff - 1];
    !prefix.is_empty()
        && prefix
            .chars()
            .all(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-')
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
        let mut text = evidence.lines().take(120).collect::<Vec<_>>().join("\n");
        if text.chars().count() > 12_000 {
            text = text.chars().take(12_000).collect();
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

fn mk_chat_req(cfg: &Profile, system: String, user: String) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: user,
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    }
}

async fn chat_once_get_text(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<String> {
    let resp = chat_once(client, chat_url, req).await?;
    Ok(resp
        .choices
        .get(0)
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default()
        .trim()
        .to_string())
}

fn mk_step_result(
    id: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    ok: bool,
    summary: String,
    command: Option<String>,
    raw_output: Option<String>,
    exit_code: Option<i32>,
    output_bytes: Option<u64>,
    truncated: bool,
    timed_out: bool,
    artifact_path: Option<String>,
    artifact_kind: Option<String>,
) -> StepResult {
    StepResult {
        id: id.to_string(),
        kind: kind.to_string(),
        purpose,
        depends_on,
        success_condition,
        ok,
        summary,
        command,
        raw_output,
        exit_code,
        output_bytes,
        truncated,
        timed_out,
        artifact_path,
        artifact_kind,
        outcome_status: None,
        outcome_reason: None,
    }
}

fn mk_simple_step_result(
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    summary: String,
    artifact_kind: Option<String>,
) -> StepResult {
    mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        summary,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        artifact_kind,
    )
}

async fn handle_select_step(
    client: &reqwest::Client,
    selector_cfg: &Profile,
    objective: &str,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    instructions: String,
    unit_type: Option<String>,
    state: &mut ExecutionState,
    args: &Args,
) -> Result<()> {
    let evidence = gather_artifacts(&depends_on, &state.artifacts);
    let selection = select_items_via_unit(
        client,
        selector_cfg,
        unit_type.as_deref(),
        objective,
        &purpose,
        &instructions,
        &evidence,
    )
    .await?;
    let items: Vec<_> = selection
        .items
        .into_iter()
        .map(|i| i.trim().to_string())
        .filter(|i| !i.is_empty())
        .collect();
    let selection_text = items.join("\n");
    state
        .artifacts
        .insert(sid.to_string(), selection_text.clone());
    trace(
        args,
        &format!(
            "selection id={sid} count={} items={:?} reason={}",
            items.len(),
            items,
            selection.reason.trim()
        ),
    );
    let summary = if items.is_empty() {
        format!("selection_empty: {}", selection.reason.trim())
    } else {
        format!(
            "selected {} item(s)\n{}",
            items.len(),
            preview_text(&selection_text, 8)
        )
    };
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        !items.is_empty(),
        summary,
        None,
        Some(selection_text),
        None,
        None,
        false,
        false,
        None,
        Some("selection".to_string()),
    ));
    if items.is_empty() {
        state.halt = true;
    }
    Ok(())
}

async fn handle_summarize_step(
    client: &reqwest::Client,
    chat_url: &Url,
    summarizer_cfg: &Profile,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    mut text: String,
    instructions: String,
    state: &mut ExecutionState,
) -> Result<()> {
    if text.trim().is_empty() && !depends_on.is_empty() {
        text = gather_artifacts(&depends_on, &state.artifacts);
    }
    let sum_req = mk_chat_req(
        &summarizer_cfg,
        summarizer_cfg.system_prompt.clone(),
        format!("Instructions:\n{}\n\nText:\n{}", instructions.trim(), text),
    );
    let sum_text = chat_once_get_text(client, chat_url, &sum_req).await?;
    state.artifacts.insert(sid.to_string(), sum_text.clone());
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        !sum_text.is_empty(),
        sum_text,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        None,
    ));
    Ok(())
}

async fn handle_plan_step(
    client: &reqwest::Client,
    chat_url: &Url,
    planner_cfg: &Profile,
    session: &SessionPaths,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    goal: String,
    state: &mut ExecutionState,
    args: &Args,
) -> Result<()> {
    let master = std::fs::read_to_string(session.plans_dir.join("_master.md")).unwrap_or_default();
    let req = mk_chat_req(
        &planner_cfg,
        planner_cfg.system_prompt.clone(),
        format!("Goal:\n{goal}\n\nMaster plan (_master.md):\n{master}"),
    );
    let text = chat_once_get_text(client, chat_url, &req).await?;
    let plan_path = write_plan_file(&session.plans_dir, &(text.trim().to_string() + "\n"))?;
    append_master_link(&session.plans_dir, &plan_path, &goal)?;
    trace(args, &format!("plan_saved={}", plan_path.display()));
    let trimmed = text.trim().to_string();
    state.artifacts.insert(sid.to_string(), trimmed.clone());
    state.step_results.push(mk_simple_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        format!(
            "saved {}\n{}",
            plan_path.display(),
            preview_text(&trimmed, 8)
        ),
        None,
    ));
    Ok(())
}

async fn handle_master_plan_step(
    client: &reqwest::Client,
    chat_url: &Url,
    planner_master_cfg: &Profile,
    session: &SessionPaths,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    goal: String,
    state: &mut ExecutionState,
    args: &Args,
) -> Result<()> {
    let req = mk_chat_req(
        &planner_master_cfg,
        planner_master_cfg.system_prompt.clone(),
        format!("Goal:\n{goal}\n\nUpdate the master plan."),
    );
    let text = chat_once_get_text(client, chat_url, &req).await?;
    let path = session.plans_dir.join("_master.md");
    std::fs::write(
        &path,
        squash_blank_lines(text.trim()).trim().to_string() + "\n",
    )
    .with_context(|| format!("write {}", path.display()))?;
    trace(args, &format!("masterplan_saved={}", path.display()));
    let trimmed = text.trim().to_string();
    state.artifacts.insert(sid.to_string(), trimmed.clone());
    state.step_results.push(mk_simple_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        format!("saved {}\n{}", path.display(), preview_text(&trimmed, 8)),
        None,
    ));
    Ok(())
}

async fn handle_decide_step(
    client: &reqwest::Client,
    chat_url: &Url,
    decider_cfg: &Profile,
    session: &SessionPaths,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    prompt: String,
    state: &mut ExecutionState,
    args: &Args,
) -> Result<()> {
    let req = mk_chat_req(&decider_cfg, decider_cfg.system_prompt.clone(), prompt);
    let word = chat_once_get_text(client, chat_url, &req)
        .await?
        .trim()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    let path = write_decision(&session.decisions_dir, &word)?;
    trace(args, &format!("decision_saved={}", path.display()));
    state.artifacts.insert(sid.to_string(), word.clone());
    state.step_results.push(mk_simple_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        word,
        None,
    ));
    Ok(())
}

fn handle_reply_step(
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    instructions: String,
    state: &mut ExecutionState,
) {
    state.final_reply = Some(instructions.clone());
    state.artifacts.insert(sid.to_string(), instructions);
    state.step_results.push(mk_simple_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        "reply".to_string(),
        None,
    ));
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
    let dep_str = if depends_on.is_empty() {
        "-".to_string()
    } else {
        depends_on.join(",")
    };
    trace(
        args,
        &format!(
            "step id={sid} type={kind} purpose={} depends_on={}",
            purpose, dep_str
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
            handle_select_step(
                client,
                selector_cfg,
                objective,
                &sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                instructions,
                common.unit_type,
                state,
                args,
            )
            .await?;
        }
        Step::Summarize {
            id: _,
            mut text,
            instructions,
            ..
        } => {
            handle_summarize_step(
                client,
                chat_url,
                summarizer_cfg,
                &sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                text,
                instructions,
                state,
            )
            .await?;
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
            handle_plan_step(
                client,
                chat_url,
                planner_cfg,
                session,
                &sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                goal,
                state,
                args,
            )
            .await?;
        }
        Step::MasterPlan { id: _, goal, .. } => {
            handle_master_plan_step(
                client,
                chat_url,
                planner_master_cfg,
                session,
                &sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                goal,
                state,
                args,
            )
            .await?;
        }
        Step::Decide { id: _, prompt, .. } => {
            handle_decide_step(
                client,
                chat_url,
                decider_cfg,
                session,
                &sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                prompt,
                state,
                args,
            )
            .await?;
        }
        Step::Reply {
            id: _,
            instructions,
            ..
        } => {
            handle_reply_step(
                &sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                instructions,
                state,
            );
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
