//! @efficiency-role: util-pure
//! Observe Step Execution
//!
//! Metadata-only inspection without consuming file contents.

use crate::format::file_size;
use crate::*;

fn normalize_target_path(workdir: &PathBuf, target: &str) -> PathBuf {
    if target.starts_with('/') {
        PathBuf::from(target)
    } else {
        workdir.join(target)
    }
}

fn system_time_unix_secs(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs())
}

fn describe_file_type(path: &Path, metadata: &std::fs::Metadata) -> String {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        if let Ok(target) = std::fs::read_link(path) {
            return format!("symlink -> {}", target.display());
        }
        return "symlink".to_string();
    }
    if file_type.is_dir() {
        return describe_directory(path, metadata);
    }

    let mut parts = vec![format!("file size={}", file_size(metadata.len()))];
    parts.push(format!("readonly={}", metadata.permissions().readonly()));
    if let Ok(modified) = metadata.modified() {
        if let Some(ts) = system_time_unix_secs(modified) {
            parts.push(format!("modified={}", ts));
        }
    }
    if let Ok(created) = metadata.created() {
        if let Some(ts) = system_time_unix_secs(created) {
            parts.push(format!("created={}", ts));
        }
    }
    if let Ok(accessed) = metadata.accessed() {
        if let Some(ts) = system_time_unix_secs(accessed) {
            parts.push(format!("accessed={}", ts));
        }
    }
    parts.join(" ")
}

fn describe_directory(path: &Path, metadata: &std::fs::Metadata) -> String {
    let mut file_count = 0usize;
    let mut dir_count = 0usize;
    let mut sample_entries = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if sample_entries.len() < 20 {
                sample_entries.push(entry.file_name().to_string_lossy().to_string());
            }
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    dir_count += 1;
                } else {
                    file_count += 1;
                }
            } else {
                file_count += 1;
            }
        }
    }

    let mut parts = vec![format!("dir entries={}", file_count + dir_count)];
    parts.push(format!("files={}", file_count));
    parts.push(format!("dirs={}", dir_count));
    parts.push(format!("readonly={}", metadata.permissions().readonly()));
    if !sample_entries.is_empty() {
        parts.push(format!("sample=[{}]", sample_entries.join(", ")));
    }
    parts.join(" ")
}

fn observe_target(path: &Path) -> Result<String> {
    let metadata =
        std::fs::symlink_metadata(path).with_context(|| format!("metadata {}", path.display()))?;
    let kind = describe_file_type(path, &metadata);
    Ok(format!("{}: {}", path.display(), kind))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_observe_step(
    args: &Args,
    _session: &SessionPaths,
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
    let observe_paths: Vec<String> = if let Some(p) = path {
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
            summary: "observe: no path or paths provided".to_string(),
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

    let is_multi = observe_paths.len() > 1;
    let mut all_reports = String::new();
    let mut errors = Vec::new();

    for (idx, target) in observe_paths.iter().enumerate() {
        trace(
            args,
            &format!("step id={} type=observe purpose={}", sid, purpose),
        );
        trace(args, &format!("observe target={}", target));

        let full_path = normalize_target_path(workdir, target);
        match observe_target(&full_path) {
            Ok(report) => {
                if is_multi {
                    all_reports.push_str(&format!(
                        "### Target {}: {}\n{}\n",
                        idx + 1,
                        target,
                        report
                    ));
                } else {
                    all_reports.push_str(&report);
                }
                if idx < observe_paths.len() - 1 {
                    all_reports.push_str("\n");
                }
            }
            Err(err) => {
                let text = format!("observe_failed: {}", err);
                if is_multi {
                    errors.push(text.clone());
                    all_reports.push_str(&format!("### Target {}: ERROR — {}\n", idx + 1, target));
                    if idx < observe_paths.len() - 1 {
                        all_reports.push_str("\n");
                    }
                } else {
                    state.step_results.push(StepResult {
                        id: sid,
                        kind: kind.to_string(),
                        purpose,
                        depends_on,
                        success_condition,
                        ok: false,
                        summary: text.clone(),
                        command: None,
                        raw_output: Some(text),
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
                "observed {} targets ({})",
                observe_paths.len(),
                file_size(all_reports.len() as u64)
            )
        } else {
            format!(
                "observed {}/{} targets ({}), errors: {}",
                observe_paths.len() - errors.len(),
                observe_paths.len(),
                file_size(all_reports.len() as u64),
                errors.join("; ")
            )
        }
    } else {
        format!("observed {}", file_size(all_reports.len() as u64))
    };

    let output_bytes = all_reports.len() as u64;
    state.artifacts.insert(sid.clone(), all_reports.clone());

    trace(
        args,
        &format!("observe_ok={} bytes={}", ok, file_size(output_bytes)),
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
        raw_output: Some(all_reports),
        exit_code: None,
        output_bytes: Some(output_bytes),
        truncated: false,
        timed_out: false,
        artifact_path: None,
        artifact_kind: Some("metadata".to_string()),
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
    async fn test_observe_single_file_metadata() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        let test_file = workdir.join("hello.txt");
        std::fs::write(&test_file, "Hello, world!\n").unwrap();

        let mut state = make_state();
        handle_observe_step(
            &args,
            &session,
            &workdir,
            "step-1".into(),
            "observe",
            "inspect metadata".into(),
            vec![],
            "metadata must be available".into(),
            Some("hello.txt"),
            None,
            &mut state,
        )
        .await
        .unwrap();

        assert!(!state.halt);
        assert_eq!(state.step_results.len(), 1);
        assert!(state.step_results[0].ok);
        let raw = state.step_results[0].raw_output.as_ref().unwrap();
        assert!(raw.contains("hello.txt"));
        assert!(raw.contains("file size="));
        assert!(!raw.contains("Hello, world!"));
    }

    #[tokio::test]
    async fn test_observe_directory_metadata() {
        let tmp = TempDir::new().unwrap();
        let workdir = tmp.path().to_path_buf();
        let session = make_session(&workdir.join("session"));
        let args = Args::parse_from(["elma-cli"]);

        let dir = workdir.join("dir");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::create_dir_all(dir.join("nested")).unwrap();

        let mut state = make_state();
        handle_observe_step(
            &args,
            &session,
            &workdir,
            "step-1".into(),
            "observe",
            "inspect directory metadata".into(),
            vec![],
            "metadata must be available".into(),
            Some("dir"),
            None,
            &mut state,
        )
        .await
        .unwrap();

        let raw = state.step_results[0].raw_output.as_ref().unwrap();
        assert!(raw.contains("dir"));
        assert!(raw.contains("entries="));
        assert!(raw.contains("sample="));
    }
}
