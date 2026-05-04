//! @efficiency-role: util-pure
//! Read Step Execution

use crate::format::file_size;
use crate::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_read_step(
    args: &Args,
    session: &SessionPaths,
    workdir: &PathBuf,
    sid: String,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    path: Option<&str>,
    paths: Option<&[String]>,
    state: &mut ExecutionState,
) -> Result<()> {
    let read_paths: Vec<String> = if let Some(p) = path {
        vec![p.to_string()]
    } else if let Some(ps) = paths {
        ps.to_vec()
    } else {
        state.step_results.push(StepResult {
            id: sid.clone(),
            kind: kind.to_string(),
            purpose,
            depends_on,
            success_condition,
            ok: false,
            summary: "read: no path or paths provided".to_string(),
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
        state.halt = true;
        return Ok(());
    };

    let is_multi = read_paths.len() > 1;
    let mut all_content = String::new();
    let mut errors: Vec<String> = Vec::new();

    for (i, tp) in read_paths.iter().enumerate() {
        trace(
            args,
            &format!("step id={} type=read purpose={}", sid, purpose),
        );
        trace(args, &format!("read path={}", tp));

        let full_path = match resolve_tool_path(workdir, tp) {
            Ok(p) => {
                let policy = crate::workspace_policy::WorkspacePolicy::new(workdir);
                if let Some(msg) = policy.blocked_message(&p, "read") {
                    state.step_results.push(StepResult {
                        id: sid.clone(),
                        kind: kind.to_string(),
                        purpose: purpose.clone(),
                        depends_on: depends_on.clone(),
                        success_condition: success_condition.clone(),
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
                p
            }
            Err(e) => {
                state.step_results.push(StepResult {
                    id: sid.clone(),
                    kind: kind.to_string(),
                    purpose: purpose.clone(),
                    depends_on: depends_on.clone(),
                    success_condition: success_condition.clone(),
                    ok: false,
                    summary: format!("path error: {}", e),
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
                state.halt = true;
                return Ok(());
            }
        };

        if !full_path.exists() {
            let err = format!("file_not_found: {}", tp);
            if is_multi {
                errors.push(err.clone());
                all_content.push_str(&format!("\n### File {}: ERROR — {}\n", i + 1, tp));
                continue;
            } else {
                state.step_results.push(StepResult {
                    id: sid,
                    kind: kind.to_string(),
                    purpose,
                    depends_on,
                    success_condition,
                    ok: false,
                    summary: err,
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
                state.halt = true;
                return Ok(());
            }
        }

        match crate::document_adapter::read_file_smart(&full_path) {
            Ok((content, header)) => {
                let file_content = if is_multi {
                    format!("### File {}: {}\n{}\n\n{}", i + 1, tp, header, content)
                } else {
                    format!("{}\n{}", header, content)
                };
                all_content.push_str(&file_content);
                if i < read_paths.len() - 1 {
                    all_content.push_str("\n\n");
                }
            }
            Err(e) => {
                let err = format!("Failed to read {}: {}", tp, e);
                if is_multi {
                    errors.push(err.clone());
                    all_content.push_str(&format!(
                        "\n### File {}: ERROR — {}
\n",
                        i + 1,
                        tp
                    ));
                } else {
                    state.step_results.push(StepResult {
                        id: sid,
                        kind: kind.to_string(),
                        purpose,
                        depends_on,
                        success_condition,
                        ok: false,
                        summary: err.clone(),
                        command: None,
                        raw_output: Some(err),
                        exit_code: None,
                        output_bytes: None,
                        truncated: false,
                        timed_out: false,
                        artifact_path: None,
                        artifact_kind: None,
                        outcome_status: None,
                        outcome_reason: None,
                    });
                    state.halt = true;
                    return Ok(());
                }
            }
        }
    }

    let ok = errors.is_empty();
    let summary = if is_multi {
        if ok {
            format!(
                "read {} files ({})",
                read_paths.len(),
                file_size(all_content.len() as u64)
            )
        } else {
            format!(
                "read {}/{} files ({}), errors: {}",
                read_paths.len() - errors.len(),
                read_paths.len(),
                file_size(all_content.len() as u64),
                errors.join("; ")
            )
        }
    } else {
        format!("read {}", file_size(all_content.len() as u64))
    };

    state.artifacts.insert(sid.clone(), all_content.clone());

    trace(
        args,
        &format!("read_ok bytes={}", file_size(all_content.len() as u64)),
    );

    state.step_results.push(StepResult {
        id: sid,
        kind: kind.to_string(),
        purpose,
        depends_on,
        success_condition,
        ok,
        summary,
        command: None,
        raw_output: Some(all_content),
        exit_code: None,
        output_bytes: None,
        truncated: false,
        timed_out: false,
        artifact_path: None,
        artifact_kind: Some("document".to_string()),
        outcome_status: None,
        outcome_reason: None,
    });

    if !ok {
        state.halt = true;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ExecutionState;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn make_state() -> ExecutionState {
        ExecutionState {
            step_results: Vec::new(),
            final_reply: None,
            artifacts: HashMap::new(),
            auto_snapshot_id: None,
            halt: false,
        }
    }

    fn make_session(root: &PathBuf) -> SessionPaths {
        let artifacts_dir = root.join("artifacts");
        std::fs::create_dir_all(&artifacts_dir).unwrap();
        SessionPaths {
            root: root.clone(),
            artifacts_dir,
        }
    }

    #[tokio::test]
    async fn test_read_single_file() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        // Create a test file
        let test_file = workdir.join("hello.txt");
        std::fs::write(&test_file, "Hello, world!\n").unwrap();

        let mut state = make_state();
        handle_read_step(
            &args,
            &session,
            &workdir,
            "step-1".into(),
            "read",
            "read a file".into(),
            vec![],
            "file must be readable".into(),
            Some("hello.txt"),
            None,
            &mut state,
        )
        .await
        .unwrap();

        assert!(!state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(state.step_results[0].ok);
        assert!(state.step_results[0]
            .raw_output
            .as_ref()
            .unwrap()
            .contains("Hello, world!"));
        assert!(!state.step_results[0]
            .raw_output
            .as_ref()
            .unwrap()
            .contains("### File"));
    }

    #[tokio::test]
    async fn test_read_multiple_files() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        std::fs::write(&workdir.join("a.txt"), "content a\n").unwrap();
        std::fs::write(&workdir.join("b.txt"), "content b\n").unwrap();

        let mut state = make_state();
        handle_read_step(
            &args,
            &session,
            &workdir,
            "step-2".into(),
            "read",
            "read multiple files".into(),
            vec![],
            "files must exist".into(),
            None,
            Some(&["a.txt".into(), "b.txt".into()]),
            &mut state,
        )
        .await
        .unwrap();

        assert!(!state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(state.step_results[0].ok);
        let output = state.step_results[0].raw_output.as_ref().unwrap();
        assert!(output.contains("### File 1: a.txt"));
        assert!(output.contains("### File 2: b.txt"));
        assert!(output.contains("content a"));
        assert!(output.contains("content b"));
    }

    #[tokio::test]
    async fn test_read_partial_failure_in_multi() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        std::fs::write(&workdir.join("exists.txt"), "here\n").unwrap();
        // nonexistent.txt does not exist

        let mut state = make_state();
        handle_read_step(
            &args,
            &session,
            &workdir,
            "step-3".into(),
            "read",
            "read with partial failure".into(),
            vec![],
            "report partial failure".into(),
            None,
            Some(&["exists.txt".into(), "nonexistent.txt".into()]),
            &mut state,
        )
        .await
        .unwrap();

        assert!(state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(!state.step_results[0].ok);
        let output = state.step_results[0].raw_output.as_ref().unwrap();
        assert!(output.contains("### File 1: exists.txt"));
        assert!(output.contains("here"));
        assert!(output.contains("### File 2: ERROR"));
        assert!(output.contains("nonexistent.txt"));
    }

    #[tokio::test]
    async fn test_read_single_file_not_found_halt() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        let mut state = make_state();
        handle_read_step(
            &args,
            &session,
            &workdir,
            "step-4".into(),
            "read",
            "read missing file".into(),
            vec![],
            "file should exist".into(),
            Some("no_such_file.txt"),
            None,
            &mut state,
        )
        .await
        .unwrap();

        assert!(state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(!state.step_results[0].ok);
        assert!(state.step_results[0].summary.contains("file_not_found"));
    }

    #[tokio::test]
    async fn test_read_no_path_provided() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        let mut state = make_state();
        handle_read_step(
            &args,
            &session,
            &workdir,
            "step-5".into(),
            "read",
            "read with no path".into(),
            vec![],
            "must provide path".into(),
            None,
            None,
            &mut state,
        )
        .await
        .unwrap();

        assert!(state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(!state.step_results[0].ok);
        assert!(state.step_results[0]
            .summary
            .contains("no path or paths provided"));
    }

    #[tokio::test]
    async fn test_read_many_files_regression() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        // Create 20 files
        let mut file_paths: Vec<String> = Vec::new();
        for i in 0..20 {
            let name = format!("file_{:02}.txt", i);
            std::fs::write(&workdir.join(&name), format!("content {}\n", i)).unwrap();
            file_paths.push(name);
        }

        let mut state = make_state();
        handle_read_step(
            &args,
            &session,
            &workdir,
            "step-6".into(),
            "read",
            "read 20 files".into(),
            vec![],
            "all files readable".into(),
            None,
            Some(&file_paths),
            &mut state,
        )
        .await
        .unwrap();

        assert!(!state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(state.step_results[0].ok);
        let output = state.step_results[0].raw_output.as_ref().unwrap();
        // Verify headers for all 20 files
        for i in 0..20 {
            let expected_header = format!("### File {}: file_{:02}.txt", i + 1, i);
            assert!(
                output.contains(&expected_header),
                "missing header: {}",
                expected_header
            );
            assert!(
                output.contains(&format!("content {}", i)),
                "missing content for file {}",
                i
            );
        }
    }
}
