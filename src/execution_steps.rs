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

const MAX_SUMMARIZE_INPUT_CHARS: usize = 30_000;

fn truncate_for_summarization(text: &str) -> (String, bool) {
    let trimmed = text.trim();
    if trimmed.len() <= MAX_SUMMARIZE_INPUT_CHARS {
        return (trimmed.to_string(), false);
    }
    let mut truncated = trimmed[..MAX_SUMMARIZE_INPUT_CHARS].to_string();
    let line_count = truncated.lines().count();
    truncated.push_str(&format!(
        "\n\n[input truncated from ~{} lines to ~{} lines for summarization]",
        trimmed.lines().count(),
        line_count
    ));
    (truncated, true)
}

pub(crate) fn normalize_selected_items_against_evidence(
    items: Vec<String>,
    _instructions: &str,
    evidence: &str,
) -> Vec<String> {
    let evidence_lines: Vec<_> = evidence
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    items
        .into_iter()
        .map(|item| normalize_single_item(item.trim(), &evidence_lines))
        .collect()
}

fn normalize_single_item(item: &str, evidence_lines: &[&str]) -> String {
    let mut trimmed = item.trim_matches(|c: char| {
        matches!(
            c,
            '"' | '\'' | '`' | ',' | ';' | ':' | ')' | ']' | '}' | '*'
        )
    });
    if trimmed.ends_with('.') && !trimmed.ends_with("..") && trimmed.matches('.').count() > 1 {
        trimmed = trimmed.trim_end_matches('.');
    }
    let matches: Vec<_> = evidence_lines
        .iter()
        .filter(|line| is_relative_path_match(line, trimmed))
        .copied()
        .collect();
    if matches.len() == 1 {
        return matches[0].to_string();
    }
    if matches.len() > 1 {
        let mut sorted = matches;
        sorted.sort_by_key(|a| a.matches('/').count());
        return sorted[0].to_string();
    }
    if evidence_lines.iter().any(|line| *line == trimmed) {
        return trimmed.to_string();
    }
    trimmed.to_string()
}

fn is_relative_path_match(line: &str, trimmed: &str) -> bool {
    let len_diff = line.len().saturating_sub(trimmed.len());
    line.ends_with(trimmed)
        && len_diff > 0
        && line.as_bytes().get(len_diff - 1) == Some(&b'/')
        && !line[..len_diff - 1].is_empty()
        && line[..len_diff - 1]
            .chars()
            .all(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == '_' || c == '-')
}

fn budget_evidence(evidence: &str) -> String {
    evidence
        .lines()
        .take(120)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(12_000)
        .collect()
}

fn skip_selection(reason: &str) -> SelectionOutput {
    SelectionOutput {
        items: Vec::new(),
        reason: reason.into(),
    }
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
    if evidence.trim().is_empty() {
        return Ok(skip_selection(
            "Upstream evidence is empty; skipping selection to prevent hallucination.",
        ));
    }
    if evidence.contains("EXECUTION FAILED") || evidence.contains("command not found") {
        return Ok(skip_selection(
            "Upstream execution failed; skipping selection to prevent hallucination.",
        ));
    }
    let evidence = budget_evidence(evidence);
    let context = IntelContext::new(
        objective.into(),
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
            profile.name = "rename_suggester".into();
            if let Some(prompt) = canonical_system_prompt("rename_suggester") {
                profile.system_prompt = prompt.into();
            }
            profile.temperature = profile.temperature.clamp(0.2, 0.35);
            profile.max_tokens = profile.max_tokens.min(120);
            let output = RenameSuggesterUnit::new(profile)
                .execute_with_fallback(&context)
                .await?;
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
            let output = SelectorUnit::new(selector_cfg.clone())
                .execute_with_fallback(&context)
                .await?;
            let mut selection: SelectionOutput = serde_json::from_value(output.data)
                .map_err(|e| anyhow::anyhow!("Failed to parse selector output: {}", e))?;
            selection.items =
                normalize_selected_items_against_evidence(selection.items, instructions, &evidence);
            Ok(selection)
        }
    }
}

pub(crate) fn mk_chat_req(cfg: &Profile, system: String, user: String) -> ChatCompletionRequest {
    chat_request_system_user(cfg, &system, &user, ChatRequestOptions::default())
}

pub(crate) async fn chat_once_get_text(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<String> {
    Ok(chat_once(client, chat_url, req)
        .await?
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
) -> StepResult {
    StepResult {
        id: id.into(),
        kind: kind.into(),
        purpose,
        depends_on,
        success_condition,
        ok,
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
    }
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
    state.artifacts.insert(sid.into(), selection_text.clone());
    trace(
        args,
        &format!(
            "selection id={sid} count={} items={:?} reason={}",
            items.len(),
            items,
            selection.reason.trim()
        ),
    );
    let ok = !items.is_empty();
    let summary = if !ok {
        format!("selection_empty: {}", selection.reason.trim())
    } else {
        format!(
            "selected {} item(s)\n{}",
            items.len(),
            preview_text(&selection_text, 8)
        )
    };
    let mut result = mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        ok,
        summary,
    );
    result.raw_output = Some(selection_text);
    result.artifact_kind = Some("selection".into());
    state.step_results.push(result);
    if !ok {
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
    let (text, truncated) = truncate_for_summarization(&text);
    let req = mk_chat_req(
        summarizer_cfg,
        summarizer_cfg.system_prompt.clone(),
        format!("Instructions:\n{}\n\nText:\n{}", instructions.trim(), text),
    );
    let sum_text = chat_once_get_text(client, chat_url, &req).await?;
    state.artifacts.insert(sid.into(), sum_text.clone());
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        !sum_text.is_empty(),
        sum_text,
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
    let master =
        std::fs::read_to_string(session.artifacts_dir.join("_master.md")).unwrap_or_default();
    let req = mk_chat_req(
        planner_cfg,
        planner_cfg.system_prompt.clone(),
        format!("Goal:\n{goal}\n\nMaster plan (_master.md):\n{master}"),
    );
    let text = chat_once_get_text(client, chat_url, &req).await?;
    let plan_path = write_plan_file(&session.artifacts_dir, &(text.trim().to_string() + "\n"))?;
    append_master_link(&session.artifacts_dir, &plan_path, &goal)?;
    trace(args, &format!("plan_saved={}", plan_path.display()));
    let trimmed = text.trim().to_string();
    state.artifacts.insert(sid.into(), trimmed.clone());
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        format!(
            "saved {}\n{}",
            plan_path.display(),
            preview_text(&trimmed, 8)
        ),
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
    let path = session.artifacts_dir.join("_master.md");
    std::fs::write(
        &path,
        squash_blank_lines(text.trim()).trim().to_string() + "\n",
    )
    .with_context(|| format!("write {}", path.display()))?;
    trace(args, &format!("masterplan_saved={}", path.display()));
    let trimmed = text.trim().to_string();
    state.artifacts.insert(sid.into(), trimmed.clone());
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        format!("saved {}\n{}", path.display(), preview_text(&trimmed, 8)),
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
    let path = write_decision(&session.artifacts_dir, &word)?;
    trace(args, &format!("decision_saved={}", path.display()));
    state.artifacts.insert(sid.into(), word.clone());
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        word,
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
    state.artifacts.insert(sid.into(), instructions);
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        "reply".into(),
    ));
}

fn handle_respond_step(
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    instructions: String,
    state: &mut ExecutionState,
) {
    state.final_reply = Some(instructions.clone());
    state.artifacts.insert(sid.into(), instructions);
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        "respond".into(),
    ));
}

fn handle_explore_step(
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    objective: String,
    state: &mut ExecutionState,
) {
    state.artifacts.insert(sid.into(), objective.clone());
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        true,
        "explore".into(),
    ));
}

fn handle_write_step(
    args: &Args,
    workdir: &PathBuf,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    path: String,
    content: String,
    state: &mut ExecutionState,
) {
    if std::path::Path::new(&path).is_absolute() {
        state.step_results.push(StepResult {
            id: sid.to_string(),
            kind: kind.to_string(),
            purpose,
            depends_on,
            success_condition,
            ok: false,
            summary: format!("absolute_path_not_allowed: {} — use workspace-relative path", path),
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
        return;
    }
    let full_path = workdir.join(&path);
    let policy = crate::workspace_policy::WorkspacePolicy::new(workdir);
    if let Some(msg) = policy.blocked_message(&full_path, "write") {
        state.step_results.push(StepResult {
            id: sid.to_string(),
            kind: kind.to_string(),
            purpose: purpose.clone(),
            depends_on,
            success_condition,
            ok: false,
            summary: msg,
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
        return;
    }
    let ok = std::fs::create_dir_all(full_path.parent().unwrap_or(workdir)).is_ok()
        && std::fs::write(&full_path, &content).is_ok();
    trace(args, &format!("write_step path={} ok={}", path, ok));
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        ok,
        "write".into(),
    ));
}

fn handle_delete_step(
    args: &Args,
    workdir: &PathBuf,
    sid: &str,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    path: String,
    state: &mut ExecutionState,
) {
    if std::path::Path::new(&path).is_absolute() {
        state.step_results.push(StepResult {
            id: sid.to_string(),
            kind: kind.to_string(),
            purpose,
            depends_on,
            success_condition,
            ok: false,
            summary: format!("absolute_path_not_allowed: {} — use workspace-relative path", path),
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
        return;
    }
    let full_path = workdir.join(&path);
    let policy = crate::workspace_policy::WorkspacePolicy::new(workdir);
    if let Some(msg) = policy.blocked_message(&full_path, "delete") {
        state.step_results.push(StepResult {
            id: sid.to_string(),
            kind: kind.to_string(),
            purpose: purpose.clone(),
            depends_on,
            success_condition,
            ok: false,
            summary: msg,
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
        return;
    }
    let ok =
        std::fs::remove_file(&full_path).is_ok() || std::fs::remove_dir_all(&full_path).is_ok();
    trace(args, &format!("delete_step path={} ok={}", path, ok));
    state.step_results.push(mk_step_result(
        sid,
        kind,
        purpose,
        depends_on,
        success_condition,
        ok,
        "delete".into(),
    ));
}

#[allow(clippy::too_many_arguments)]
async fn summarize_batch_content(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    batch_content: &str,
    summary_prompt: &str,
    objective: &str,
) -> Result<String> {
    let system_prompt = format!(
        "You are a structured analysis summarizer. Your task: read the following content \
         from multiple sources and produce a detailed summary focused on the objective: \"{}\". \n\
         Include: key information found, relationships between items, and relevance to \
         the objective. Be thorough — this summary may be the only representation of \
         these items for later analysis. \n\
         Output format: plain text paragraphs, no markdown headings.",
        objective
    );
    let user_message = format!(
        "{}\n\n## Item contents\n{}",
        summary_prompt, batch_content
    );
    let req = mk_chat_req(cfg, system_prompt, user_message);
    let sum_text = chat_once_get_text(client, chat_url, &req).await?;
    Ok(sum_text)
}

#[allow(clippy::too_many_arguments)]
async fn handle_batch_step(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    summarizer_cfg: &Profile,
    batches: &[BatchGroup],
    objective: &str,
    state: &mut ExecutionState,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<()> {
    let mut batch_summaries: Vec<String> = Vec::new();
    let mut total_items_processed: usize = 0;
    let mut failures: Vec<String> = Vec::new();

    for batch in batches {
        if let Some(ref mut t) = tui {
            t.set_coordinator_status(
                format!("Batch {}/{}: processing {} items...",
                    batch.batch_number, batches.len(), batch.item_uris.len()),
                true,
            );
            let _ = t.pump_ui();
        }

        let mut batch_content = String::new();
        for (i, kind) in batch.item_kinds.iter().enumerate() {
            let item_uri = &batch.item_uris[i];

            let content_result = match kind {
                ItemKind::FilePath(path) => {
                    let full_path = if path.starts_with('/') {
                        std::path::PathBuf::from(path)
                    } else {
                        workdir.join(path)
                    };
                    match std::fs::read_to_string(&full_path) {
                        Ok(c) => {
                            let tokens = crate::token_counter::count_tokens(&c);
                            Ok(format!("\n=== ITEM: {} ({} tokens, source=file) ===\n{}\n",
                                path, tokens, c))
                        }
                        Err(e) => Err(format!("File read error: {} — {}", path, e)),
                    }
                }
                ItemKind::ShellOutput { command_hash, offset_bytes, length_bytes } => {
                    let artifact_key = format!("shell_output_{}", command_hash);
                    if let Some(output) = state.artifacts.get(&artifact_key) {
                        let start = *offset_bytes as usize;
                        let end = (*offset_bytes + *length_bytes) as usize;
                        let segment = output.get(start..end.min(output.len())).unwrap_or("");
                        Ok(format!("\n=== ITEM: shell://{} (bytes {}-{}) ===\n{}\n",
                            command_hash, offset_bytes, offset_bytes + length_bytes, segment))
                    } else {
                        Err(format!("Shell output artifact not found: {}", command_hash))
                    }
                }
                ItemKind::SearchPage { query, file, start_line: _, match_count } => {
                    let search_cmd = std::process::Command::new("rg")
                        .args(["-n", "-C", "2", query, file])
                        .output();
                    match search_cmd {
                        Ok(out) => {
                            let text = String::from_utf8_lossy(&out.stdout);
                            Ok(format!("\n=== ITEM: search://{}@{} ({} matches) ===\n{}\n",
                                query, file, match_count, text))
                        }
                        Err(e) => Err(format!("Search error: {}@{} — {}", query, file, e)),
                    }
                }
                ItemKind::TextBlock { source_label } => {
                    let artifact_key = format!("text_block_{}", source_label);
                    if let Some(text) = state.artifacts.get(&artifact_key) {
                        Ok(format!("\n=== ITEM: text://{} ===\n{}\n", source_label, text))
                    } else {
                        Err(format!("Text block artifact not found: {}", source_label))
                    }
                }
            };

            match content_result {
                Ok(content) => {
                    batch_content.push_str(&content);
                    total_items_processed += 1;
                }
                Err(err_msg) => {
                    failures.push(format!("{} [{}]", err_msg, item_uri));
                    batch_content.push_str(&format!("\n=== ITEM: {} (ERROR: {}) ===\n", item_uri, err_msg));
                }
            }
        }

        let mut summary_prompt = batch.summary_prompt.clone();

        if batch.depends_on_previous && !batch_summaries.is_empty() {
            summary_prompt.push_str(&format!(
                "\n\nThis is batch {}/{}.\n", batch.batch_number, batches.len()
            ));
            summary_prompt.push_str("\n## Previous batch findings (for context, do not repeat)\n");
            for (i, prior) in batch_summaries.iter().enumerate() {
                let token_count = crate::token_counter::count_tokens(prior);
                let display = if token_count > 500 {
                    let cutoff = prior.char_indices()
                        .nth(prior.len() / 4 * 3)
                        .map(|(i, _)| i)
                        .unwrap_or(prior.len());
                    format!("{}... (truncated, {} total tokens)", &prior[..cutoff], token_count)
                } else {
                    prior.clone()
                };
                summary_prompt.push_str(&format!(
                    "### Batch {} summary ({})\n{}\n\n", i + 1, token_count, display
                ));
            }
            summary_prompt.push_str(
                "Use the above context to avoid repeating findings. \
                 Focus on new information and connections across batches. \
                 Build cumulative understanding toward the objective."
            );
        }

        let summary = summarize_batch_content(
            client, chat_url, summarizer_cfg,
            &batch_content, &summary_prompt, objective,
        ).await?;

        batch_summaries.push(summary.clone());

        let artifact_key = format!("batch_summary_{}", batch.batch_number);
        state.artifacts.insert(artifact_key, summary);

        if let Some(ref mut t) = tui {
            t.push_meta_event(
                "BATCH",
                &format!(
                    "batch {}/{} complete: {} items processed, {} failures",
                    batch.batch_number, batches.len(),
                    batch.item_uris.len(), failures.len()
                ),
            );
            let _ = t.pump_ui();
        }
    }

    let mut aggregated = String::new();
    aggregated.push_str(&format!(
        "## Batch Processing Results: {} items across {} batches\n\n",
        total_items_processed, batches.len()
    ));

    if !failures.is_empty() {
        aggregated.push_str("### Warnings\n");
        for f in &failures {
            aggregated.push_str(&format!("- {}\n", f));
        }
        aggregated.push('\n');
    }

    for (i, summary) in batch_summaries.iter().enumerate() {
        aggregated.push_str(&format!("### Batch {}\n{}\n\n", i + 1, summary));
    }

    state.artifacts.insert("aggregated_summary".to_string(), aggregated.clone());

    let success = failures.is_empty() || failures.len() < batches.iter().map(|b| b.item_uris.len()).sum::<usize>() / 2;

    state.step_results.push(StepResult {
        id: format!("batch_{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos() & 0xFFFF_FFFF),
        kind: "batch".to_string(),
        purpose: format!("Process {} items in {} batches", total_items_processed, batches.len()),
        depends_on: vec![],
        success_condition: "All batches completed with usable summaries".to_string(),
        ok: success,
        summary: aggregated,
        command: None,
        raw_output: None,
        exit_code: None,
        output_bytes: None,
        truncated: false,
        timed_out: false,
        artifact_path: None,
        artifact_kind: None,
        outcome_status: Some(if success { "completed" } else { "partial" }.to_string()),
        outcome_reason: if !failures.is_empty() {
            Some(format!("{} item acquisition failures", failures.len()))
        } else {
            None
        },
    });

    Ok(())
}

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
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<()> {
    if let Some(t) = tui.as_deref_mut() {
        let _ = t.pump_ui();
    }
    let sid = step_id(&step).to_string();
    let kind = step_kind(&step).to_string();
    let purpose = step_purpose(&step);
    let depends_on = step_depends_on(&step);
    let success_condition = step_success_condition(&step);
    let dep_str = if depends_on.is_empty() {
        "-".into()
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
    let is_tool_step = !matches!(step, Step::Reply { .. } | Step::Respond { .. });
    if is_tool_step {
        operator_trace(args, &purpose);
        if let Some(ref mut t) = tui {
            t.handle_ui_event(crate::claude_ui::UiEvent::ToolStarted {
                name: kind.clone(),
                command: purpose.clone(),
            });
            t.set_coordinator_status(purpose.clone(), true);
        }
    }

    match step {
        Step::Read { path, paths, .. } => {
            handle_read_step(
                args,
                session,
                workdir,
                sid,
                &kind,
                purpose,
                depends_on,
                success_condition,
                path.as_deref(),
                paths.as_deref(),
                state,
            )
            .await?
        }
        Step::Search { query, paths, .. } => {
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
            .await?
        }
        Step::Shell { cmd, common, .. } => {
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
                common.is_destructive,
                sid,
                kind.clone(),
                purpose,
                depends_on,
                success_condition,
                cmd,
                state,
                tui.as_deref_mut(),
            )
            .await?
        }
        Step::Select {
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
            .await?
        }
        Step::Summarize {
            mut text,
            instructions,
            ..
        } => {
            if text.trim().is_empty() && !depends_on.is_empty() {
                text = gather_artifacts(&depends_on, &state.artifacts);
            }
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
            .await?
        }
        Step::Edit { spec, .. } => handle_edit_step(
            args,
            session,
            workdir,
            sid,
            kind.clone(),
            purpose,
            depends_on,
            success_condition,
            spec,
            state,
        )?,
        Step::Plan { goal, .. } => {
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
            .await?
        }
        Step::MasterPlan { goal, .. } => {
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
            .await?
        }
        Step::Decide { prompt, .. } => {
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
            .await?
        }
        Step::Reply { instructions, .. } => handle_reply_step(
            &sid,
            &kind,
            purpose,
            depends_on,
            success_condition,
            instructions,
            state,
        ),
        Step::Respond { instructions, .. } => handle_respond_step(
            &sid,
            &kind,
            purpose,
            depends_on,
            success_condition,
            instructions,
            state,
        ),
        Step::Explore { objective, .. } => handle_explore_step(
            &sid,
            &kind,
            purpose,
            depends_on,
            success_condition,
            objective,
            state,
        ),
        Step::Write { path, content, .. } => handle_write_step(
            args,
            workdir,
            &sid,
            &kind,
            purpose,
            depends_on,
            success_condition,
            path,
            content,
            state,
        ),
        Step::Delete { ref path, .. } => handle_delete_step(
            args,
            workdir,
            &sid,
            &kind,
            purpose,
            depends_on,
            success_condition,
            path.clone(),
            state,
        ),
        Step::Batch { ref batches, .. } => {
            handle_batch_step(
                args,
                client,
                chat_url,
                session,
                workdir,
                summarizer_cfg,
                batches,
                objective,
                state,
                tui.as_deref_mut(),
            ).await?
        }
    }

    // Task 287: Add evidence ledger entry for legacy execution path
    if let Some(result) = state.step_results.last() {
        let is_evidence_step = !matches!(
            kind.as_str(),
            "reply" | "respond" | "plan" | "masterplan" | "decide" | "select" | "summarize"
        );
        if result.ok && is_evidence_step {
            let source = match kind.as_str() {
                "shell" => {
                    let cmd = result.command.clone().unwrap_or_default();
                    crate::evidence_ledger::EvidenceSource::Shell {
                        command: cmd,
                        exit_code: result.exit_code.unwrap_or(0),
                    }
                }
                "read" => {
                    let path = result
                        .summary
                        .split_whitespace()
                        .next()
                        .unwrap_or("")
                        .to_string();
                    crate::evidence_ledger::EvidenceSource::Read { path }
                }
                "search" => crate::evidence_ledger::EvidenceSource::Tool {
                    name: "search".to_string(),
                    input: result.summary.chars().take(100).collect(),
                },
                "write" | "edit" | "delete" => {
                    if let Some(path) = result.summary.split_whitespace().next() {
                        crate::evidence_ledger::with_session_ledger(|ledger| {
                            ledger.mark_path_modified(path);
                        });
                    }
                    crate::evidence_ledger::EvidenceSource::Tool {
                        name: kind.clone(),
                        input: result.summary.chars().take(100).collect(),
                    }
                }
                _ => crate::evidence_ledger::EvidenceSource::Tool {
                    name: kind.clone(),
                    input: result.summary.chars().take(100).collect(),
                },
            };
            if let Some(raw) = &result.raw_output {
                crate::evidence_ledger::with_session_ledger(|ledger| {
                    ledger.add_entry(source, raw);
                });
            }
        }
    }

    if is_tool_step {
        if let Some(ref mut t) = tui {
            let last_result = state.step_results.last();
            let success = last_result.map(|r| r.ok).unwrap_or(false);
            // Use raw_output for full content, fallback to summary if raw is missing
            let output = last_result
                .and_then(|r| r.raw_output.clone())
                .unwrap_or_else(|| last_result.map(|r| r.summary.clone()).unwrap_or_default());
            t.handle_ui_event(crate::claude_ui::UiEvent::ToolFinished {
                name: kind.clone(),
                success,
                output,
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
        let evidence = "_stress_testing/_opencode_for_testing/main.go\n_stress_testing/_opencode_for_testing/cmd/root.go";
        let instructions = "Choose exactly one most likely primary entry point. Return the exact relative path only.";
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
        let instructions = "Choose exactly one most likely primary entry point. Return the exact relative path only.";
        let items = normalize_selected_items_against_evidence(
            vec!["main.go".to_string()],
            instructions,
            evidence,
        );
        assert_eq!(items, vec!["_stress_testing/_opencode_for_testing/main.go"]);
    }
}
