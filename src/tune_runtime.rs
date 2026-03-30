//! @efficiency-role: scenario-spec
//!
//! Runtime calibration suite execution.

use crate::tune::{RuntimeAggregation, TuneResources};
use crate::*;

pub(crate) async fn run_runtime_calibration(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    resources: &TuneResources,
    emit_progress: bool,
) -> Result<RuntimeAggregation> {
    let manifest = load_tuning_manifest(&args.tune_mode, true)?;
    if manifest.version != 1 {
        anyhow::bail!(
            "Unsupported calibration manifest version {}",
            manifest.version
        );
    }

    let scenario_total = manifest.scenarios.len();
    let mut aggregation = RuntimeAggregation::default();

    for (scenario_index, scenario) in manifest.scenarios.into_iter().enumerate() {
        if emit_progress {
            calibration_progress(
                args,
                &format!(
                    "calibrating {}: runtime suite {}/{} ({})",
                    resources.elma_cfg.model,
                    scenario_index + 1,
                    scenario_total,
                    scenario.file
                ),
            );
        }
        let outcome = tune_scenario::evaluate_runtime_scenario(
            args,
            client,
            chat_url,
            resources,
            scenario,
        )
        .await?;
        aggregation.push(outcome);
    }

    Ok(aggregation)
}
