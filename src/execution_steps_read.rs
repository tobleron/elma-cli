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
    path: &str,
    state: &mut ExecutionState,
) -> Result<()> {
    trace(
        args,
        &format!("step id={} type=read purpose={}", sid, purpose),
    );
    trace(args, &format!("read path={}", path));

    let full_path = if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        workdir.join(path)
    };

    if !full_path.exists() {
        state.step_results.push(StepResult {
            id: sid,
            kind: kind.to_string(),
            purpose,
            depends_on,
            success_condition,
            ok: false,
            summary: format!("file_not_found: {}", path),
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

    match crate::document_adapter::read_file_smart(&full_path) {
        Ok((content, header)) => {
            let full_content = format!("{}\n{}", header, content);
            state.artifacts.insert(sid.clone(), full_content.clone());

            trace(
                args,
                &format!("read_ok bytes={}", file_size(full_content.len() as u64)),
            );

            state.step_results.push(StepResult {
                id: sid,
                kind: kind.to_string(),
                purpose,
                depends_on,
                success_condition,
                ok: true,
                summary: format!("read {}", file_size(full_content.len() as u64)),
                command: None,
                raw_output: Some(full_content),
                exit_code: None,
                output_bytes: None,
                truncated: false,
                timed_out: false,
                artifact_path: None,
                artifact_kind: Some("document".to_string()),
                outcome_status: None,
                outcome_reason: None,
            });
        }
        Err(e) => {
            let error_msg = format!("Failed to read {}: {}", path, e);
            state.step_results.push(StepResult {
                id: sid,
                kind: kind.to_string(),
                purpose,
                depends_on,
                success_condition,
                ok: false,
                summary: error_msg.clone(),
                command: None,
                raw_output: Some(error_msg),
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
        }
    }

    Ok(())
}
