use crate::*;

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
        },
    )?;

    let active_baseline_dir = make_candidate_dir(&run_root, "00_active_baseline")?;
    snapshot_active_profile_set(model_cfg_dir, &active_baseline_dir)?;

    let shipped_src_dir = ensure_baseline_profile_set(model_cfg_dir, base_url, model_id)?;
    let shipped_baseline_dir = make_candidate_dir(&run_root, "00_shipped_baseline")?;
    copy_profile_set(&shipped_src_dir, &shipped_baseline_dir)?;
    sync_profile_dir_base_url_and_model(&shipped_baseline_dir, base_url, model_id)?;

    let runtime_defaults = fetch_runtime_generation_defaults(client, &Url::parse(base_url)?).await?;
    let runtime_baseline_dir = if let Some(defaults) = runtime_defaults.as_ref() {
        let dir = make_candidate_dir(&run_root, "00_runtime_default_baseline")?;
        copy_profile_set(&shipped_src_dir, &dir)?;
        apply_runtime_generation_defaults(&dir, defaults)?;
        sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
        Some(dir)
    } else {
        None
    };

    calibration_progress(
        args,
        &format!(
            "tune stage 1/4: protected baselines on {} corpus for {model_id}",
            args.tune_mode
        ),
    );
    let mut protected_search = Vec::new();
    for (name, dir) in [
        ("00_active_baseline", active_baseline_dir.clone()),
        ("00_shipped_baseline", shipped_baseline_dir.clone()),
    ] {
        let (score, hard_rejected, note) =
            evaluate_routing_suite(args, client, chat_url, &dir, model_id).await?;
        save_stage_score_note(&dir, "stage1_routing", &note)?;
        protected_search.push(SearchCandidate {
            name: name.to_string(),
            dir,
            score,
            hard_rejected,
        });
    }
    if let Some(dir) = runtime_baseline_dir.as_ref() {
        let (score, hard_rejected, note) =
            evaluate_routing_suite(args, client, chat_url, dir, model_id).await?;
        save_stage_score_note(dir, "stage1_routing", &note)?;
        protected_search.push(SearchCandidate {
            name: "00_runtime_default_baseline".to_string(),
            dir: dir.clone(),
            score,
            hard_rejected,
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

    if stagnant_rounds < 2 {
        let router_variants = ["router_strict", "router_soft"];
        let mut stage2_scores = Vec::new();
        calibration_progress(
            args,
            &format!("tune stage 2/4: routing params for {model_id}"),
        );
        for parent in &beam {
            for variant in router_variants {
                let dir =
                    make_candidate_dir(&run_root, &format!("20_{}_{}", parent.name, variant))?;
                copy_profile_set(&parent.dir, &dir)?;
                apply_router_param_variant(&dir, variant)?;
                validate_tuning_mutations(&parent.dir, &dir).with_context(|| {
                    format!("router variant '{variant}' violated tuning boundaries")
                })?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_routing_suite(args, client, chat_url, &dir, model_id).await?;
                save_stage_score_note(&dir, "stage2_routing", &note)?;
                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                };
                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage2_scores.push(candidate);
            }
        }
        beam = select_top_search_beam(stage2_scores, beam_width);
        if beam.is_empty() {
            anyhow::bail!(
                "All routing candidates were hard-rejected during stage 2 for {model_id}."
            );
        }
        if let Some(top) = beam.first() {
            if top.score - best_stage_score < 0.02 {
                stagnant_rounds += 1;
            } else {
                stagnant_rounds = 0;
                best_stage_score = top.score;
            }
        }
    }

    if stagnant_rounds < 2 {
        let orch_variants = ["orch_conservative", "orch_balanced", "orch_creative"];
        let mut stage3_scores = Vec::new();
        calibration_progress(
            args,
            &format!("tune stage 3/4: workflow orchestration for {model_id}"),
        );
        for parent in &beam {
            for variant in orch_variants {
                let dir =
                    make_candidate_dir(&run_root, &format!("30_{}_{}", parent.name, variant))?;
                copy_profile_set(&parent.dir, &dir)?;
                apply_orchestrator_param_variant(&dir, variant)?;
                validate_tuning_mutations(&parent.dir, &dir).with_context(|| {
                    format!("orch variant '{variant}' violated tuning boundaries")
                })?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_workflow_suite(args, client, chat_url, &dir, model_id).await?;
                save_stage_score_note(&dir, "stage3_workflow", &note)?;
                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                };
                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage3_scores.push(candidate);
            }
        }
        beam = select_top_search_beam(stage3_scores, beam_width);
        if beam.is_empty() {
            anyhow::bail!(
                "All workflow candidates were hard-rejected during stage 3 for {model_id}."
            );
        }
        if let Some(top) = beam.first() {
            if top.score - best_stage_score < 0.02 {
                stagnant_rounds += 1;
            } else {
                stagnant_rounds = 0;
                best_stage_score = top.score;
            }
        }
    }

    if stagnant_rounds < 2 {
        let response_variants = ["response_stable", "response_balanced", "response_creative"];
        let mut stage4_scores = Vec::new();
        calibration_progress(
            args,
            &format!("tune stage 4/4: response quality for {model_id}"),
        );
        for parent in &beam {
            for variant in response_variants {
                let dir =
                    make_candidate_dir(&run_root, &format!("40_{}_{}", parent.name, variant))?;
                copy_profile_set(&parent.dir, &dir)?;
                apply_response_param_variant(&dir, variant)?;
                validate_tuning_mutations(&parent.dir, &dir).with_context(|| {
                    format!("response variant '{variant}' violated tuning boundaries")
                })?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_response_suite(args, client, chat_url, &dir, model_id).await?;
                save_stage_score_note(&dir, "stage4_response", &note)?;
                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                };
                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage4_scores.push(candidate);
            }
        }
        let final_pool = select_top_search_beam(stage4_scores, beam_width);
        if let Some(top) = final_pool.first() {
            if top.score > best_search.score {
                best_search = top.clone();
                beam = final_pool;
            }
        }
    }

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
        Some(
            evaluate_candidate_dir(args, client, chat_url, base_url, dir, model_id, false).await?,
        )
    } else {
        None
    };

    let winner_dir = run_root.join("winner");
    snapshot_active_profile_set(&search_winner.dir, &winner_dir)?;
    snapshot_active_profile_set(&active_baseline_dir, &run_root.join("active_baseline_evaluated"))?;
    snapshot_active_profile_set(
        &shipped_baseline_dir,
        &run_root.join("shipped_baseline_evaluated"),
    )?;
    if let Some(dir) = runtime_baseline_dir.as_ref() {
        snapshot_active_profile_set(dir, &run_root.join("runtime_default_baseline_evaluated"))?;
    }

    calibration_progress(
        args,
        &format!("tune stability check: {}", search_winner.name),
    );
    let winner_stability =
        evaluate_candidate_stability(args, client, chat_url, base_url, &search_winner.dir, model_id)
            .await?;
    let active_stability = evaluate_candidate_stability(
        args,
        client,
        chat_url,
        base_url,
        &active_baseline_dir,
        model_id,
    )
    .await?;
    let shipped_stability = evaluate_candidate_stability(
        args,
        client,
        chat_url,
        base_url,
        &shipped_baseline_dir,
        model_id,
    )
    .await?;
    let runtime_stability = if let Some(dir) = runtime_baseline_dir.as_ref() {
        Some(
            evaluate_candidate_stability(args, client, chat_url, base_url, dir, model_id).await?,
        )
    } else {
        None
    };

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

    let (activation_src, activation_dir, final_score, final_certified, selected_name) =
        if should_activate {
            calibration_progress(
                args,
                &format!("tune activating winner ({}) — {}", search_winner.name, reason),
            );
            (
                "tune",
                &search_winner.dir,
                best_overall.score,
                best_overall.report.summary.certified,
                search_winner.name.clone(),
            )
        } else {
            calibration_progress(
                args,
                &format!("tune preferring baseline — {}", reason),
            );
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
        },
    )?;

    if should_activate {
        Ok(best_overall)
    } else {
        match preferred_baseline.source.as_str() {
            "runtime_default" => runtime_baseline_full
                .context("runtime default baseline score missing"),
            "shipped_baseline" => Ok(shipped_baseline_full),
            _ => Ok(active_baseline_full),
        }
    }
}

fn make_baseline_report(
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

fn choose_preferred_baseline(baselines: &[BaselineAnchorReport]) -> Result<BaselineAnchorReport> {
    let runtime = baselines.iter().find(|b| b.source == "runtime_default").cloned();
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

async fn evaluate_candidate_stability(
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
