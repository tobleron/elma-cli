//! @efficiency-role: domain-logic
//!
//! Execution Steps - Edit Step Handling

use crate::ui::ui_diff::StructuredDiff;
use crate::*;

pub(crate) fn handle_edit_step(
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

    let old_content = std::fs::read_to_string(&path).unwrap_or_default();

    let mut diff_text = String::new();
    let action_summary = match operation {
        "write_file" => {
            std::fs::write(&path, spec.content.as_bytes())
                .with_context(|| format!("write {}", path.display()))?;
            let new_content = spec.content.clone();
            diff_text = if old_content.is_empty() {
                format!("New file: {}\n{}", path.display(), new_content)
            } else {
                let diff_viewer = StructuredDiff::new(
                    &path.display().to_string(),
                    &path.display().to_string(),
                    &old_content,
                    &new_content,
                );
                let diff_lines = diff_viewer.render_ratatui(80);
                diff_lines
                    .iter()
                    .map(|l| {
                        l.spans
                            .iter()
                            .map(|s| s.content.clone())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
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
            let new_content = format!("{}{}", old_content, spec.content);
            diff_text = if old_content.is_empty() {
                format!("New file: {}\n{}", path.display(), new_content)
            } else {
                let diff_viewer = StructuredDiff::new(
                    &path.display().to_string(),
                    &path.display().to_string(),
                    &old_content,
                    &new_content,
                );
                let diff_lines = diff_viewer.render_ratatui(80);
                diff_lines
                    .iter()
                    .map(|l| {
                        l.spans
                            .iter()
                            .map(|s| s.content.clone())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!("appended {}", path.display())
        }
        "replace_text" => {
            if spec.find.is_empty() {
                anyhow::bail!("replace_text requires non-empty find");
            }
            let replaced = old_content.replace(&spec.find, &spec.replace);
            if replaced == old_content {
                anyhow::bail!("replace_text found no matches in {}", path.display());
            }
            std::fs::write(&path, replaced.as_bytes())
                .with_context(|| format!("write {}", path.display()))?;
            let new_content = replaced;
            diff_text = {
                let diff_viewer = StructuredDiff::new(
                    &path.display().to_string(),
                    &path.display().to_string(),
                    &old_content,
                    &new_content,
                );
                let diff_lines = diff_viewer.render_ratatui(80);
                diff_lines
                    .iter()
                    .map(|l| {
                        l.spans
                            .iter()
                            .map(|s| s.content.clone())
                            .collect::<String>()
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
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
        &format!("edit_saved path={} operation={}", path.display(), operation),
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
        raw_output: Some(diff_text),
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
