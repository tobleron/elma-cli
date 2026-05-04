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

    let path = resolve_tool_path(workdir, &spec.path)?;
    let policy = crate::workspace_policy::WorkspacePolicy::new(workdir);
    if let Some(msg) = policy.blocked_message(&path, "edit") {
        state.step_results.push(StepResult {
            id: sid.clone(),
            kind: kind.clone(),
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
        return Ok(());
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::collections::HashMap;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn setup_test_env() -> (TempDir, PathBuf, PathBuf) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let workdir = temp_dir.path().to_path_buf();
        let project_tmp = workdir.join("project_tmp");
        fs::create_dir_all(&project_tmp).expect("Failed to create project_tmp");

        let dummy_files = vec![
            ("dummy.txt", "initial content here"),
            ("code.rs", "fn main() {\n    println!(\"Hello\");\n}"),
            ("config.toml", "[section]\nkey = \"value\""),
            ("empty.txt", ""),
        ];
        for (name, content) in dummy_files {
            let path = project_tmp.join(name);
            fs::write(&path, content).expect(&format!("Failed to write {}", name));
        }

        (temp_dir, workdir, project_tmp)
    }

    fn mock_args() -> Args {
        Args::try_parse_from(&["test"]).expect("Failed to parse Args")
    }

    fn mock_session_paths(workdir: &PathBuf) -> SessionPaths {
        let artifacts_dir = workdir.join("artifacts");
        fs::create_dir_all(&artifacts_dir).expect("Failed to create artifacts dir");
        SessionPaths {
            root: workdir.clone(),
            artifacts_dir,
        }
    }

    fn mock_execution_state() -> ExecutionState {
        ExecutionState {
            step_results: Vec::new(),
            final_reply: None,
            artifacts: HashMap::new(),
            auto_snapshot_id: Some("test_snapshot".to_string()),
            halt: false,
        }
    }

    fn to_relative_path(workdir: &PathBuf, absolute_path: &PathBuf) -> String {
        absolute_path.strip_prefix(workdir)
            .unwrap_or(absolute_path)
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn test_write_file_new() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("new_file.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "write_file".to_string(),
            content: "hello world".to_string(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_1".to_string(), "test".to_string(),
            "Write new file".to_string(), vec![],
            "File should be written".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(step_result.ok);
        assert!(step_result.summary.contains("wrote"));
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn test_write_file_overwrite() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("dummy.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "write_file".to_string(),
            content: "overwritten content".to_string(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_2".to_string(), "test".to_string(),
            "Overwrite existing file".to_string(), vec![],
            "File should be overwritten".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(step_result.ok);
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "overwritten content");
    }

    #[test]
    fn test_append_text_existing() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("dummy.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "append_text".to_string(),
            content: "\nappended line".to_string(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_3".to_string(), "test".to_string(),
            "Append to existing file".to_string(), vec![],
            "Content should be appended".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(step_result.ok);
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("initial content here"));
        assert!(content.contains("appended line"));
    }

    #[test]
    fn test_append_text_new_file() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("brand_new.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "append_text".to_string(),
            content: "new file content".to_string(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_4".to_string(), "test".to_string(),
            "Append to new file".to_string(), vec![],
            "New file should be created".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(step_result.ok);
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new file content");
    }

    #[test]
    fn test_replace_text_success() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("dummy.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "replace_text".to_string(),
            content: String::new(),
            find: "initial".to_string(),
            replace: "updated".to_string(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_5".to_string(), "test".to_string(),
            "Replace text".to_string(), vec![],
            "Text should be replaced".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(step_result.ok);
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("updated content here"));
        assert!(!content.contains("initial content here"));
    }

    #[test]
    fn test_replace_text_no_match() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("dummy.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "replace_text".to_string(),
            content: String::new(),
            find: "nonexistent".to_string(),
            replace: "something".to_string(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_6".to_string(), "test".to_string(),
            "Replace with no match".to_string(), vec![],
            "Should fail".to_string(),
            spec, &mut state,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("found no matches"));
    }

    #[test]
    fn test_replace_text_empty_find() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("dummy.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "replace_text".to_string(),
            content: String::new(),
            find: String::new(),
            replace: "something".to_string(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_7".to_string(), "test".to_string(),
            "Replace with empty find".to_string(), vec![],
            "Should fail".to_string(),
            spec, &mut state,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires non-empty find"));
    }

    #[test]
    fn test_unsupported_operation() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        let file_path = project_tmp.join("test.txt");
        let spec = EditSpec {
            path: to_relative_path(&workdir, &file_path),
            operation: "invalid_op".to_string(),
            content: String::new(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_8".to_string(), "test".to_string(),
            "Unsupported operation".to_string(), vec![],
            "Should fail".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(!step_result.ok);
        assert!(step_result.summary.contains("unsupported edit operation"));
    }

    #[test]
    fn test_absolute_path_within_workspace() {
        let (_temp_dir, workdir, project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        // Use absolute path within workspace - should be accepted now
        let file_path = project_tmp.join("absolute_test.txt");
        let abs_path = file_path.to_string_lossy().to_string();
        let spec = EditSpec {
            path: abs_path,
            operation: "write_file".to_string(),
            content: "absolute path works".to_string(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_abs".to_string(), "test".to_string(),
            "Absolute path test".to_string(), vec![],
            "Should succeed with absolute path".to_string(),
            spec, &mut state,
        );

        assert!(result.is_ok());
        let step_result = state.step_results.first().unwrap();
        assert!(step_result.ok);
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "absolute path works");
    }

    #[test]
    fn test_absolute_path_outside_workspace() {
        let (_temp_dir, workdir, _project_tmp) = setup_test_env();
        let args = mock_args();
        let session = mock_session_paths(&workdir);
        let mut state = mock_execution_state();

        // Use absolute path outside workspace - should be rejected
        let spec = EditSpec {
            path: "/etc/passwd".to_string(),
            operation: "write_file".to_string(),
            content: "should fail".to_string(),
            find: String::new(),
            replace: String::new(),
        };

        let result = handle_edit_step(
            &args, &session, &workdir,
            "step_outside".to_string(), "test".to_string(),
            "Outside workspace".to_string(), vec![],
            "Should fail".to_string(),
            spec, &mut state,
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("outside workspace"));
    }

    #[test]
    fn test_edit_operation_is_supported() {
        assert!(edit_operation_is_supported("write_file"));
        assert!(edit_operation_is_supported("replace_text"));
        assert!(edit_operation_is_supported("append_text"));
        assert!(!edit_operation_is_supported("invalid"));
        assert!(!edit_operation_is_supported(""));
    }
}
