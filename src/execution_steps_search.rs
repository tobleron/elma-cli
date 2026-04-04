//! @efficiency-role: util-pure
//! Search Step Execution

use crate::*;

#[allow(clippy::too_many_arguments)]
pub(crate) async fn handle_search_step(
    args: &Args,
    session: &SessionPaths,
    workdir: &PathBuf,
    sid: String,
    kind: &str,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    query: &str,
    paths: Vec<String>,
    state: &mut ExecutionState,
) -> Result<()> {
    trace(
        args,
        &format!("step id={} type=search purpose={}", sid, purpose),
    );
    trace(args, &format!("search query={} paths={:?}", query, paths));

    let search_paths = if paths.is_empty() {
        vec![workdir.clone()]
    } else {
        paths
            .iter()
            .map(|p| {
                if p.starts_with('/') {
                    PathBuf::from(p)
                } else {
                    workdir.join(p)
                }
            })
            .collect()
    };

    let mut cmd = std::process::Command::new("rg");
    cmd.arg("--line-number")
        .arg("--color")
        .arg("never")
        .arg("--max-count")
        .arg("100")
        .arg(query);

    for path in &search_paths {
        if path.exists() {
            cmd.arg(path);
        }
    }

    let output = cmd.output();

    let (ok, output_text, exit_code) = match output {
        Ok(out) => {
            let text = String::from_utf8_lossy(&out.stdout).to_string();
            let code = Some(out.status.code().unwrap_or(0));
            let ok = out.status.success() || !text.is_empty();
            (ok, text, code)
        }
        Err(e) => {
            let fallback_text = format!("search_failed: {}", e);
            (false, fallback_text, None)
        }
    };

    state.artifacts.insert(sid.clone(), output_text.clone());

    trace(
        args,
        &format!("search_ok={} bytes={}", ok, output_text.len()),
    );

    state.step_results.push(StepResult {
        id: sid,
        kind: kind.to_string(),
        purpose,
        depends_on,
        success_condition,
        ok,
        summary: if ok {
            format!("found {} matches", output_text.lines().count())
        } else {
            format!("no_matches: {}", query)
        },
        command: Some(format!("rg {}", query)),
        raw_output: Some(output_text.clone()),
        exit_code,
        output_bytes: Some((output_text.len() as u64)),
        truncated: false,
        timed_out: false,
        artifact_path: None,
        artifact_kind: Some("search_results".to_string()),
        outcome_status: None,
        outcome_reason: None,
    });

    Ok(())
}
