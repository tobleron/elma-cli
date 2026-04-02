use crate::*;

pub(crate) async fn evaluate_routing_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("json_outputter.toml")) {
        set_json_outputter_profile(Some(cfg));
    }
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("final_answer_extractor.toml")) {
        set_final_answer_extractor_profile(Some(cfg));
    }
    evaluation_routing::evaluate_routing_suite_impl(args, client, chat_url, candidate_dir, model_id)
        .await
}

pub(crate) async fn evaluate_workflow_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("json_outputter.toml")) {
        set_json_outputter_profile(Some(cfg));
    }
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("final_answer_extractor.toml")) {
        set_final_answer_extractor_profile(Some(cfg));
    }
    evaluation_workflow::evaluate_workflow_suite_impl(
        args,
        client,
        chat_url,
        candidate_dir,
        model_id,
    )
    .await
}

pub(crate) async fn evaluate_response_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("json_outputter.toml")) {
        set_json_outputter_profile(Some(cfg));
    }
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("final_answer_extractor.toml")) {
        set_final_answer_extractor_profile(Some(cfg));
    }
    evaluation_response::evaluate_response_suite_impl(
        args,
        client,
        chat_url,
        candidate_dir,
        model_id,
    )
    .await
}

pub(crate) async fn evaluate_candidate_dir(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    candidate_dir: &PathBuf,
    model_id: &str,
    emit_progress: bool,
) -> Result<CandidateScore> {
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("json_outputter.toml")) {
        set_json_outputter_profile(Some(cfg));
    }
    if let Ok(cfg) = load_agent_config(&candidate_dir.join("final_answer_extractor.toml")) {
        set_final_answer_extractor_profile(Some(cfg));
    }
    sync_profile_dir_base_url_and_model(candidate_dir, base_url, model_id)?;
    let tune_cfg = load_agent_config(&candidate_dir.join("intention_tune.toml"))?;
    tune_model(
        args,
        client,
        chat_url,
        base_url,
        candidate_dir,
        model_id,
        &tune_cfg,
        emit_progress,
    )
    .await?;
    let report = load_calibration_report(&candidate_dir.join("calibration_report.json"))?;
    let efficiency_report =
        load_efficiency_report(&candidate_dir.join("efficiency_report.json")).ok();
    let efficiency_score = efficiency_report
        .as_ref()
        .map(score_efficiency_report)
        .unwrap_or(0.0);
    let score = (0.75 * score_calibration_report(&report)) + (0.25 * efficiency_score);
    let hard_rejected = hard_rejects_calibration_report(&report);
    Ok(CandidateScore {
        name: candidate_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "candidate".to_string()),
        dir: candidate_dir.clone(),
        report: report.clone(),
        score,
        hard_rejected,
        variance: 0.0, // Task 009: Will be calculated in repeated evaluations
        std_dev: 0.0,
        parse_failure_count: report.summary.all_ok.total - report.summary.all_ok.correct,
        latency_avg_ms: 0.0, // Task 009: Will be measured in repeated evaluations
    })
}

pub(crate) fn make_candidate_dir(run_root: &Path, name: &str) -> Result<PathBuf> {
    let safe = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let dir = run_root.join("candidates").join(safe);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    Ok(dir)
}

pub(crate) fn select_top_beam(
    candidates: Vec<CandidateScore>,
    beam_width: usize,
) -> Vec<CandidateScore> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut out = Vec::new();
    for candidate in sorted {
        if candidate.hard_rejected {
            continue;
        }
        out.push(candidate);
        if out.len() >= beam_width {
            break;
        }
    }
    out
}

pub(crate) fn select_top_search_beam(
    candidates: Vec<SearchCandidate>,
    beam_width: usize,
) -> Vec<SearchCandidate> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut out = Vec::new();
    for candidate in sorted {
        if candidate.hard_rejected {
            continue;
        }
        out.push(candidate);
        if out.len() >= beam_width {
            break;
        }
    }
    out
}
