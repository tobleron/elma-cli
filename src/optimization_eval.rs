//! @efficiency-role: service-orchestrator
//!
//! Model Optimization - Evaluation Module
//!
//! Contains helper functions for baseline evaluation and stability checking.

use crate::*;

pub(crate) fn make_baseline_report(
    name: &str,
    source: &str,
    candidate: &CandidateScore,
    stability: StabilitySummary,
) -> BaselineAnchorReport {
    let adjusted_score = candidate.score - stability.penalty;
    BaselineAnchorReport {
        name: name.to_string(),
        source: source.to_string(),
        raw_score: candidate.score,
        adjusted_score,
        certified: candidate.report.summary.certified,
        hard_rejected: candidate.hard_rejected,
        stability,
    }
}

pub(crate) fn choose_preferred_baseline(
    baselines: &[BaselineAnchorReport],
) -> Result<BaselineAnchorReport> {
    let runtime = baselines
        .iter()
        .find(|b| b.source == "runtime_default")
        .cloned();
    let best = baselines
        .iter()
        .cloned()
        .max_by(|a, b| {
            a.adjusted_score
                .partial_cmp(&b.adjusted_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .context("no baseline reports available")?;
    if let Some(runtime) = runtime {
        if (best.adjusted_score - runtime.adjusted_score) < ACTIVATION_MARGIN {
            return Ok(runtime);
        }
    }
    Ok(best)
}

pub(crate) async fn evaluate_candidate_stability(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<StabilitySummary> {
    let mut quick_args = args.clone();
    quick_args.tune_mode = "quick".to_string();
    let runs = 3usize;
    let mut scores = Vec::with_capacity(runs);
    for _ in 0..runs {
        let result = evaluate_candidate_dir(
            &quick_args,
            client,
            chat_url,
            base_url,
            candidate_dir,
            model_id,
            false,
        )
        .await?;
        scores.push(result.score);
    }
    let mean_score = scores.iter().sum::<f64>() / runs as f64;
    let min_score = scores
        .iter()
        .copied()
        .fold(f64::INFINITY, |acc, v| acc.min(v));
    let max_score = scores
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, |acc, v| acc.max(v));
    let variance = scores
        .iter()
        .map(|v| {
            let d = *v - mean_score;
            d * d
        })
        .sum::<f64>()
        / runs as f64;
    let stddev = variance.sqrt();
    let penalty = stddev.min(0.05);
    Ok(StabilitySummary {
        runs,
        mean_score,
        min_score,
        max_score,
        stddev,
        penalty,
    })
}
