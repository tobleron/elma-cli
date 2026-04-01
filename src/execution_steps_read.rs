//! Read Step Execution

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
    trace(args, &format!("step id={} type=read purpose={}", sid, purpose));
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

    let content = std::fs::read_to_string(&full_path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path, e))?;

    state.artifacts.insert(sid.clone(), content.clone());

    trace(args, &format!("read_ok bytes={}", content.len()));

    state.step_results.push(StepResult {
        id: sid,
        kind: kind.to_string(),
        purpose,
        depends_on,
        success_condition,
        ok: true,
        summary: format!("read {} bytes", content.len()),
        command: None,
        raw_output: Some(content),
        exit_code: None,
        output_bytes: None,
        truncated: false,
        timed_out: false,
        artifact_path: None,
        artifact_kind: Some("file".to_string()),
        outcome_status: None,
        outcome_reason: None,
    });

    Ok(())
}
