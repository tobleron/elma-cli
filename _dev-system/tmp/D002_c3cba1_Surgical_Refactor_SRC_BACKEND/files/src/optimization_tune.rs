//! @efficiency-role: service-orchestrator
//!
//! Model Optimization - Tuning Module
//!
//! Contains the main optimize_model function for parameter tuning.

use crate::app_bootstrap_profiles::load_profiles;
use crate::*;

// ---------------------------------------------------------------------------
// Private helpers for optimize_model
// ---------------------------------------------------------------------------

async fn eval_stability(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    dir: &PathBuf,
    model_id: &str,
) -> Result<StabilitySummary> {
    evaluate_candidate_stability(args, client, chat_url, base_url, dir, model_id).await
}

/// Update beam, stagnation counter, and best score after a stage.
fn update_beam_state(
    beam: &mut Vec<SearchCandidate>,
    best_stage_score: &mut f64,
    stagnant_rounds: &mut usize,
    stage_scores: Vec<SearchCandidate>,
    beam_width: usize,
) {
    *beam = select_top_search_beam(stage_scores, beam_width);
    if let Some(top) = beam.first() {
        if top.score - *best_stage_score < 0.02 {
            *stagnant_rounds += 1;
        } else {
            *stagnant_rounds = 0;
            *best_stage_score = top.score;
        }
    }
}

/// Common body for stages 2-4: iterate beam parents and variants,
/// evaluate each candidate, and optionally short-circuit in quick mode.
struct SearchStage<'a> {
    variants: &'a [&'a str],
    default_variant: &'a str,
    default_threshold: f64,
    stage_label: &'a str,
}

impl<'a> SearchStage<'a> {
    fn router() -> Self {
        Self {
            variants: &["router_strict", "router_soft"],
            default_variant: "router_soft",
            default_threshold: 0.85,
            stage_label: "stage 2/5: routing params",
        }
    }
    fn orchestration() -> Self {
        Self {
            variants: &["orch_conservative", "orch_balanced", "orch_creative"],
            default_variant: "orch_balanced",
            default_threshold: 0.80,
            stage_label: "stage 3/5: workflow orchestration",
        }
    }
    fn response() -> Self {
        Self {
            variants: &["response_stable", "response_balanced", "response_creative"],
            default_variant: "response_balanced",
            default_threshold: 0.80,
            stage_label: "stage 4/5: response quality",
        }
    }
}

pub(crate) async fn optimize_model(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
) -> Result<CandidateScore> {
    let run_id = new_tune_run_id()?;
    let run_root = model_tune_runs_dir(model_cfg_dir).join(&run_id);
    std::fs::create_dir_all(run_root.join("candidates"))
        .with_context(|| format!("mkdir {}", run_root.display()))?;
    snapshot_active_profile_set(model_cfg_dir, &run_root.join("live_before"))?;

    let profiles = load_profiles(model_cfg_dir)?;
    let prompt_hashes = crate::tune::compute_all_prompt_hashes(&profiles);

    save_tune_run_manifest(
        &run_root.join("run_manifest.toml"),
        &TuneRunManifest {
            version: 1,
            run_id: run_id.clone(),
            model: model_id.to_string(),
            mode: "tune".to_string(),
            started_unix_s: now_unix_s()?,
            activated: false,
            final_score: 0.0,
            certified: false,
            activation_reason: String::new(),
            baseline_score: 0.0,
            prompt_hashes,
        },
    )?;

    // Stage 0: JSON Temperature Tuning
    calibration_progress(
        args,
        &format!("tune stage 0/5: JSON temperature tuning for {model_id}",),
    );
    let json_tuning_result =
        run_json_temperature_tuning(args, client, chat_url, model_cfg_dir, model_id, true).await?;
    save_json_tuning_report(model_cfg_dir, &json_tuning_result)?;
    apply_json_tuning_temperature(model_cfg_dir, json_tuning_result.recommended_temperature)?;

    calibration_progress(
        args,
        &format!(
            "  JSON tuning complete: optimal_temp={:.2}, recommended_temp={:.2}, score={:.3}",
            json_tuning_result.optimal_temperature,
            json_tuning_result.recommended_temperature,
            json_tuning_result
                .results_by_temp
                .iter()
                .find(|r| r.temperature == json_tuning_result.recommended_temperature)
                .map(|r| r.weighted_score)
                .unwrap_or(0.0)
        ),
    );

    let shipped_src_dir = ensure_baseline_profile_set(model_cfg_dir, base_url, model_id)?;
    let shipped_baseline_dir = make_candidate_dir(&run_root, "00_shipped_baseline")?;
    copy_profile_set(&shipped_src_dir, &shipped_baseline_dir)?;
    sync_profile_dir_base_url_and_model(&shipped_baseline_dir, base_url, model_id)?;

    let active_baseline_dir = make_candidate_dir(&run_root, "00_active_baseline")?;
    snapshot_active_profile_set(model_cfg_dir, &active_baseline_dir)?;

    let runtime_defaults =
        fetch_runtime_generation_defaults(client, &Url::parse(base_url)?).await?;
    let runtime_baseline_dir = if let Some(defaults) = runtime_defaults.as_ref() {
        let dir = make_candidate_dir(&run_root, "00_runtime_default_baseline")?;
        copy_profile_set(&shipped_src_dir, &dir)?;
        apply_runtime_generation_defaults(&dir, defaults)?;
        sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
        Some(dir)
    } else {
        None
    };

    // Stage 1: Protected baselines
    calibration_progress(
        args,
        &format!(
            "tune stage 1/5: protected baselines on {} corpus for {model_id}",
            args.tune_mode
        ),
    );
    let mut protected_search = Vec::new();

    let (score, hard_rejected, _note) =
        evaluate_routing_suite(args, client, chat_url, model_cfg_dir, model_id).await?;
    protected_search.push(SearchCandidate {
        name: "00_active_baseline_with_json_tuning".to_string(),
        dir: model_cfg_dir.clone(),
        score,
        hard_rejected,
        variance: 0.0,
        std_dev: 0.0,
    });
    calibration_progress(
        args,
        &format!(
            "  active baseline (with JSON tuning temps): score={:.3}, hard_rejected={}",
            score, hard_rejected
        ),
    );

    let (score, hard_rejected, note) =
        evaluate_routing_suite(args, client, chat_url, &shipped_baseline_dir, model_id).await?;
    save_stage_score_note(&shipped_baseline_dir, "stage1_routing", &note)?;
    protected_search.push(SearchCandidate {
        name: "00_shipped_baseline".to_string(),
        dir: shipped_baseline_dir.clone(),
        score,
        hard_rejected,
        variance: 0.0,
        std_dev: 0.0,
    });
    if let Some(dir) = runtime_baseline_dir.as_ref() {
        let (score, hard_rejected, note) =
            evaluate_routing_suite(args, client, chat_url, dir, model_id).await?;
        save_stage_score_note(dir, "stage1_routing", &note)?;
        protected_search.push(SearchCandidate {
            name: "00_runtime_default_baseline".to_string(),
            dir: dir.clone(),
            score,
            hard_rejected,
            variance: 0.0,
            std_dev: 0.0,
        });
    }

    let beam_width = 3usize;
    let mut beam = select_top_search_beam(protected_search, beam_width);
    if beam.is_empty() {
        anyhow::bail!("All protected baselines were hard-rejected during stage 1 for {model_id}.");
    }
    let mut best_search = beam[0].clone();
    let mut best_stage_score = best_search.score;
    let mut stagnant_rounds = 0usize;
    if let Some(top) = beam.first() {
        if top.score - best_stage_score < 0.02 {
            stagnant_rounds += 1;
        } else {
            best_stage_score = top.score;
        }
    }

    // Stages 2-4: parameter sweep using shared loop body
    let stages = [
        SearchStage::router(),
        SearchStage::orchestration(),
        SearchStage::response(),
    ];
    for stage in stages {
        if stagnant_rounds >= 2 {
            break;
        }
        let mut stage_scores = Vec::new();
        let quick_mode = args.tune_mode == "quick";
        let mut default_passed = false;

        calibration_progress(args, &format!("tune {} for {model_id}", stage.stage_label));

        for parent in &beam {
            for &variant in stage.variants {
                if quick_mode && variant != stage.default_variant && !default_passed {
                    continue;
                }

                let prefix = match stage.stage_label {
                    "stage 2/5: routing params" => "20",
                    "stage 3/5: workflow orchestration" => "30",
                    _ => "40",
                };
                let dir = make_candidate_dir(
                    &run_root,
                    &format!("{prefix}_{}_{}", parent.name, variant),
                )?;
                copy_profile_set(&parent.dir, &dir)?;

                match stage.stage_label {
                    "stage 2/5: routing params" => apply_router_param_variant(&dir, variant)?,
                    "stage 3/5: workflow orchestration" => {
                        apply_orchestrator_param_variant(&dir, variant)?
                    }
                    _ => apply_response_param_variant(&dir, variant)?,
                }

                validate_tuning_mutations(&parent.dir, &dir)
                    .with_context(|| format!("variant '{variant}' violated tuning boundaries"))?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;

                let (score, hard_rejected, note) = match stage.stage_label {
                    "stage 2/5: routing params" => {
                        evaluate_routing_suite(args, client, chat_url, &dir, model_id).await?
                    }
                    "stage 3/5: workflow orchestration" => {
                        evaluate_workflow_suite(args, client, chat_url, &dir, model_id).await?
                    }
                    _ => evaluate_response_suite(args, client, chat_url, &dir, model_id).await?,
                };
                save_stage_score_note(&dir, stage.stage_label, &note)?;

                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                    variance: 0.0,
                    std_dev: 0.0,
                };

                if quick_mode
                    && variant == stage.default_variant
                    && !hard_rejected
                    && score >= stage.default_threshold
                {
                    default_passed = true;
                    if candidate.score > best_search.score {
                        best_search = candidate.clone();
                    }
                    stage_scores.push(candidate);
                    calibration_progress(
                        args,
                        &format!(
                            "  {} default passed (score={:.3}), skipping other variants",
                            stage.stage_label, score
                        ),
                    );
                    break;
                }

                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage_scores.push(candidate);
            }
        }

        update_beam_state(
            &mut beam,
            &mut best_stage_score,
            &mut stagnant_rounds,
            stage_scores,
            beam_width,
        );
        if beam.is_empty() {
            anyhow::bail!(
                "All candidates were hard-rejected during {} for {model_id}.",
                stage.stage_label
            );
        }
        if let Some(top) = beam.first() {
            if top.score > best_search.score {
                best_search = top.clone();
            }
        }
    }

    // Final validation
    let search_winner = beam.first().cloned().unwrap_or_else(|| best_search.clone());
    calibration_progress(
        args,
        &format!("tune final validation: {}", search_winner.name),
    );
    let best_overall = evaluate_candidate_dir(
        args,
        client,
        chat_url,
        base_url,
        &search_winner.dir,
        model_id,
        false,
    )
    .await?;

    calibration_progress(
        args,
        &format!("tune protected baseline validation for {model_id}"),
    );
    let active_baseline_full = evaluate_candidate_dir(
        args,
        client,
        chat_url,
        base_url,
        &active_baseline_dir,
        model_id,
        false,
    )
    .await?;
    let shipped_baseline_full = evaluate_candidate_dir(
        args,
        client,
        chat_url,
        base_url,
        &shipped_baseline_dir,
        model_id,
        false,
    )
    .await?;
    let runtime_baseline_full = if let Some(dir) = runtime_baseline_dir.as_ref() {
        Some(evaluate_candidate_dir(args, client, chat_url, base_url, dir, model_id, false).await?)
    } else {
        None
    };

    let winner_dir = run_root.join("winner");
    snapshot_active_profile_set(&search_winner.dir, &winner_dir)?;
    snapshot_active_profile_set(
        &active_baseline_dir,
        &run_root.join("active_baseline_evaluated"),
    )?;
    snapshot_active_profile_set(
        &shipped_baseline_dir,
        &run_root.join("shipped_baseline_evaluated"),
    )?;
    if let Some(dir) = runtime_baseline_dir.as_ref() {
        snapshot_active_profile_set(dir, &run_root.join("runtime_default_baseline_evaluated"))?;
    }

    // Stability checks
    calibration_progress(
        args,
        &format!("tune stability check: {}", search_winner.name),
    );
    let winner_stability = eval_stability(
        args,
        client,
        chat_url,
        base_url,
        &search_winner.dir,
        model_id,
    )
    .await?;
    let active_stability = eval_stability(
        args,
        client,
        chat_url,
        base_url,
        &active_baseline_dir,
        model_id,
    )
    .await?;
    let shipped_stability = eval_stability(
        args,
        client,
        chat_url,
        base_url,
        &shipped_baseline_dir,
        model_id,
    )
    .await?;
    let runtime_stability = if let Some(dir) = runtime_baseline_dir.as_ref() {
        Some(eval_stability(args, client, chat_url, base_url, dir, model_id).await?)
    } else {
        None
    };

    // Activation decision
    let winner_adjusted = best_overall.score - winner_stability.penalty;
    let mut baseline_reports = vec![
        make_baseline_report(
            "active_live",
            "active_live",
            &active_baseline_full,
            active_stability,
        ),
        make_baseline_report(
            "shipped_baseline",
            "shipped_baseline",
            &shipped_baseline_full,
            shipped_stability,
        ),
    ];
    if let (Some(full), Some(stability)) = (runtime_baseline_full.as_ref(), runtime_stability) {
        baseline_reports.push(make_baseline_report(
            "runtime_default",
            "runtime_default",
            full,
            stability,
        ));
    }
    let preferred_baseline = choose_preferred_baseline(&baseline_reports)?;
    let baseline_score = preferred_baseline.adjusted_score;
    let (should_activate, raw_reason) = activation_reason(
        winner_adjusted,
        baseline_score,
        best_overall.report.summary.certified,
    );
    let reason = format!(
        "{} | selected={:.4} raw={:.4} stability_penalty={:.4} | baseline={} adjusted={:.4}",
        raw_reason,
        winner_adjusted,
        best_overall.score,
        winner_stability.penalty,
        preferred_baseline.name,
        preferred_baseline.adjusted_score
    );

    let (activation_src, activation_dir, final_score, final_certified, selected_name): (
        &str,
        &PathBuf,
        f64,
        bool,
        String,
    ) = if should_activate {
        calibration_progress(
            args,
            &format!(
                "tune activating winner ({}) -- {}",
                search_winner.name, reason
            ),
        );
        (
            "tune",
            &search_winner.dir,
            best_overall.score,
            best_overall.report.summary.certified,
            search_winner.name.clone(),
        )
    } else {
        calibration_progress(args, &format!("tune preferring baseline -- {}", reason));
        let selected_dir = match preferred_baseline.source.as_str() {
            "runtime_default" => runtime_baseline_dir
                .as_ref()
                .context("runtime default baseline dir missing")?,
            "shipped_baseline" => &shipped_baseline_dir,
            _ => &active_baseline_dir,
        };
        let selected_certified = match preferred_baseline.source.as_str() {
            "runtime_default" => runtime_baseline_full
                .as_ref()
                .map(|c| c.report.summary.certified)
                .unwrap_or(false),
            "shipped_baseline" => shipped_baseline_full.report.summary.certified,
            _ => active_baseline_full.report.summary.certified,
        };
        (
            "tune_baseline_preferred",
            selected_dir,
            preferred_baseline.raw_score,
            selected_certified,
            preferred_baseline.name.clone(),
        )
    };

    let decision = TuneDecisionReport {
        version: 1,
        model: model_id.to_string(),
        selected_name: selected_name.clone(),
        selected_source: activation_src.to_string(),
        selected_raw_score: final_score,
        selected_adjusted_score: if should_activate {
            winner_adjusted
        } else {
            preferred_baseline.adjusted_score
        },
        protected_baseline_name: preferred_baseline.name.clone(),
        protected_baseline_adjusted_score: preferred_baseline.adjusted_score,
        activation_reason: reason.clone(),
        baselines: baseline_reports.clone(),
    };
    save_json_pretty(&run_root.join("baseline_report.json"), &baseline_reports)?;
    save_json_pretty(&run_root.join("activation_summary.json"), &decision)?;

    activate_profile_set(
        model_cfg_dir,
        activation_dir,
        base_url,
        model_id,
        activation_src,
        Some(run_id.clone()),
        final_score,
        final_certified,
        &reason,
        baseline_score,
    )?;

    let profiles = load_profiles(model_cfg_dir)?;
    let prompt_hashes = crate::tune::compute_all_prompt_hashes(&profiles);

    save_tune_run_manifest(
        &run_root.join("run_manifest.toml"),
        &TuneRunManifest {
            version: 1,
            run_id,
            model: model_id.to_string(),
            mode: "tune".to_string(),
            started_unix_s: now_unix_s()?,
            activated: should_activate,
            final_score,
            certified: final_certified,
            activation_reason: reason,
            baseline_score,
            prompt_hashes,
        },
    )?;

    if should_activate {
        Ok(best_overall)
    } else {
        match preferred_baseline.source.as_str() {
            "runtime_default" => {
                runtime_baseline_full.context("runtime default baseline score missing")
            }
            "shipped_baseline" => Ok(shipped_baseline_full),
            _ => Ok(active_baseline_full),
        }
    }
}
