//! @efficiency-role: scenario-spec
//!
//! Calibration summary report generation.

use crate::tune::{RuntimeAggregation, TuneResources};
use crate::*;

pub(crate) fn write_tune_reports(
    args: &Args,
    model_cfg_dir: &PathBuf,
    model_id: &str,
    base_url: &str,
    resources: &TuneResources,
    aggregation: RuntimeAggregation,
    emit_progress: bool,
) -> Result<()> {
    let total = aggregation.scenario_results.len();
    let summary = CalibrationSummary {
        total_cases: total,
        speech_act: calibration_metric(aggregation.speech_correct, total),
        workflow: calibration_metric(aggregation.workflow_correct, total),
        mode: calibration_metric(aggregation.mode_correct, aggregation.mode_total),
        route: calibration_metric(aggregation.route_correct, total),
        program_parse: calibration_metric(aggregation.program_parse_correct, total),
        program_shape: calibration_metric(aggregation.program_shape_correct, total),
        program_policy: calibration_metric(aggregation.program_policy_correct, total),
        program_consistency: calibration_metric(aggregation.program_consistency_correct, total),
        execution: calibration_metric(aggregation.execution_correct, aggregation.execution_total),
        critic: calibration_metric(aggregation.critic_correct, aggregation.critic_total),
        response: calibration_metric(aggregation.response_correct, aggregation.response_total),
        scope: calibration_metric(aggregation.scope_correct, aggregation.scope_total),
        compaction: calibration_metric(aggregation.compaction_correct, aggregation.compaction_total),
        classification: calibration_metric(
            aggregation.classification_correct,
            aggregation.classification_total,
        ),
        claim_check: calibration_metric(
            aggregation.claim_check_correct,
            aggregation.claim_check_total,
        ),
        presentation: calibration_metric(
            aggregation.presentation_correct,
            aggregation.presentation_total,
        ),
        all_ok: calibration_metric(aggregation.all_ok_correct, total),
        certified: total > 0
            && calibration_metric(aggregation.speech_correct, total).accuracy >= 0.80
            && calibration_metric(aggregation.workflow_correct, total).accuracy >= 0.85
            && calibration_metric(aggregation.mode_correct, aggregation.mode_total).accuracy >= 0.80
            && calibration_metric(aggregation.route_correct, total).accuracy >= 0.85
            && calibration_metric(aggregation.program_parse_correct, total).accuracy >= 0.95
            && calibration_metric(aggregation.program_shape_correct, total).accuracy >= 0.85
            && calibration_metric(aggregation.program_policy_correct, total).accuracy >= 0.95
            && calibration_metric(aggregation.program_consistency_correct, total).accuracy >= 0.80
            && calibration_metric(aggregation.execution_correct, aggregation.execution_total).accuracy >= 0.80
            && calibration_metric(aggregation.critic_correct, aggregation.critic_total).accuracy >= 0.80
            && calibration_metric(aggregation.response_correct, aggregation.response_total).accuracy >= 0.80
            && calibration_metric(aggregation.scope_correct, aggregation.scope_total).accuracy >= 0.75
            && calibration_metric(aggregation.compaction_correct, aggregation.compaction_total).accuracy >= 0.75
            && calibration_metric(aggregation.classification_correct, aggregation.classification_total).accuracy >= 0.70
            && calibration_metric(aggregation.claim_check_correct, aggregation.claim_check_total).accuracy >= 0.75
            && calibration_metric(aggregation.presentation_correct, aggregation.presentation_total).accuracy >= 0.80,
        certification_rule: "speech_act>=0.80 workflow>=0.85 mode>=0.80 route>=0.85 parse>=0.95 shape>=0.85 policy>=0.95 consistency>=0.80 execution>=0.80 critic>=0.80 response>=0.80 scope>=0.75 compaction>=0.75 classification>=0.70 claim_check>=0.75 presentation>=0.80".to_string(),
    };
    let report = CalibrationReport {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        supports_logprobs: resources.supports_logprobs,
        n_probs: resources.n_probs,
        summary,
        speech_act_confusions: build_confusions(&aggregation.speech_pairs),
        workflow_confusions: build_confusions(&aggregation.workflow_pairs),
        mode_confusions: build_confusions(&aggregation.mode_pairs),
        route_confusions: build_confusions(&aggregation.route_pairs),
        scenarios: aggregation.scenario_results.clone(),
    };
    let report_path = model_cfg_dir.join("calibration_report.json");
    save_calibration_report(&report_path, &report)?;
    trace(
        args,
        &format!("tune_calibration_report_saved={}", report_path.display()),
    );

    let efficiency_total = aggregation.efficiency_scenarios.len();
    let task_success_sum = aggregation
        .efficiency_scenarios
        .iter()
        .map(|scenario| if scenario.task_success { 1.0 } else { 0.0 })
        .sum::<f64>();
    let grounding_sum = aggregation
        .efficiency_scenarios
        .iter()
        .filter_map(|scenario| scenario.grounding_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let grounding_total = aggregation
        .efficiency_scenarios
        .iter()
        .filter(|scenario| scenario.grounding_ok.is_some())
        .count();
    let scope_sum = aggregation
        .efficiency_scenarios
        .iter()
        .filter_map(|scenario| scenario.scope_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let scope_metric_total = aggregation
        .efficiency_scenarios
        .iter()
        .filter(|scenario| scenario.scope_ok.is_some())
        .count();
    let compaction_sum = aggregation
        .efficiency_scenarios
        .iter()
        .filter_map(|scenario| scenario.compaction_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let compaction_metric_total = aggregation
        .efficiency_scenarios
        .iter()
        .filter(|scenario| scenario.compaction_ok.is_some())
        .count();
    let classification_sum = aggregation
        .efficiency_scenarios
        .iter()
        .filter_map(|scenario| scenario.classification_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let classification_metric_total = aggregation
        .efficiency_scenarios
        .iter()
        .filter(|scenario| scenario.classification_ok.is_some())
        .count();
    let claim_check_sum = aggregation
        .efficiency_scenarios
        .iter()
        .filter_map(|scenario| scenario.claim_check_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let claim_check_metric_total = aggregation
        .efficiency_scenarios
        .iter()
        .filter(|scenario| scenario.claim_check_ok.is_some())
        .count();
    let presentation_sum = aggregation
        .efficiency_scenarios
        .iter()
        .filter_map(|scenario| scenario.presentation_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let presentation_metric_total = aggregation
        .efficiency_scenarios
        .iter()
        .filter(|scenario| scenario.presentation_ok.is_some())
        .count();
    let tool_economy_sum = aggregation
        .efficiency_scenarios
        .iter()
        .map(|scenario| scenario.tool_economy_score)
        .sum::<f64>();

    let efficiency_summary = EfficiencySummary {
        total_cases: efficiency_total,
        task_success_rate: efficiency_metric_from_score(task_success_sum, efficiency_total),
        grounding_rate: efficiency_metric_from_score(grounding_sum, grounding_total),
        scope_precision: efficiency_metric_from_score(scope_sum, scope_metric_total),
        compaction_rate: efficiency_metric_from_score(compaction_sum, compaction_metric_total),
        classification_rate: efficiency_metric_from_score(
            classification_sum,
            classification_metric_total,
        ),
        claim_check_rate: efficiency_metric_from_score(claim_check_sum, claim_check_metric_total),
        presentation_rate: efficiency_metric_from_score(
            presentation_sum,
            presentation_metric_total,
        ),
        tool_economy: efficiency_metric_from_score(tool_economy_sum, efficiency_total),
        overall_efficiency: (0.30
            * efficiency_metric_from_score(task_success_sum, efficiency_total).score)
            + (0.20 * efficiency_metric_from_score(grounding_sum, grounding_total).score)
            + (0.15 * efficiency_metric_from_score(scope_sum, scope_metric_total).score)
            + (0.05 * efficiency_metric_from_score(compaction_sum, compaction_metric_total).score)
            + (0.05
                * efficiency_metric_from_score(
                    classification_sum,
                    classification_metric_total,
                )
                .score)
            + (0.10
                * efficiency_metric_from_score(claim_check_sum, claim_check_metric_total).score)
            + (0.05
                * efficiency_metric_from_score(presentation_sum, presentation_metric_total).score)
            + (0.10 * efficiency_metric_from_score(tool_economy_sum, efficiency_total).score),
    };
    let efficiency_report = EfficiencyReport {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        summary: efficiency_summary,
        scenarios: aggregation.efficiency_scenarios,
    };
    let efficiency_path = model_cfg_dir.join("efficiency_report.json");
    save_efficiency_report(&efficiency_path, &efficiency_report)?;
    trace(
        args,
        &format!("tune_efficiency_report_saved={}", efficiency_path.display()),
    );
    if emit_progress {
        calibration_progress(
            args,
            &format!(
                "calibration finished for {model_id}: score {:.3}, certified={}",
                score_calibration_report(&report),
                report.summary.certified
            ),
        );
    }
    Ok(())
}
