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

    // === Protected Baseline Anchor (Section A) ===
    // Evaluate the current active profiles as the protected baseline BEFORE
    // any candidate variants are applied. This score is used for the safer
    // activation policy (Section E).
    let baseline_dir = make_candidate_dir(&run_root, "00_baseline")?;
    snapshot_active_profile_set(model_cfg_dir, &baseline_dir)?;
    calibration_progress(
        args,
        &format!(
            "tune baseline: routing corpus ({}) for {model_id}",
            args.tune_mode
        ),
    );
    let (baseline_routing_score, baseline_reject, baseline_note) =
        evaluate_routing_suite(args, client, chat_url, &baseline_dir, model_id).await?;
    save_stage_score_note(&baseline_dir, "stage1_routing", &baseline_note)?;

    let baseline = SearchCandidate {
        name: "00_baseline".to_string(),
        dir: baseline_dir.clone(),
        score: baseline_routing_score,
        hard_rejected: baseline_reject,
    };
    let mut beam = vec![baseline.clone()];
    let mut best_search = baseline;
    let mut best_stage_score = best_search.score;
    let mut stagnant_rounds = 0usize;
    let beam_width = 3usize;

    let stage1_variants = ["none"];
    let mut stage1_scores = Vec::new();
    calibration_progress(
        args,
        &format!("tune stage 1/4: frozen prompt baseline for {model_id}"),
    );
    for variant in stage1_variants {
        let dir = make_candidate_dir(&run_root, &format!("10_prompt_{variant}"))?;
        copy_profile_set(&beam[0].dir, &dir)?;
        apply_prompt_bundle(&dir, variant)?;
        // Section B: Validate no immutable fields were mutated
        validate_tuning_mutations(&baseline_dir, &dir)
            .with_context(|| format!("prompt bundle '{variant}' violated tuning boundaries"))?;
        sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
        let (score, hard_rejected, note) =
            evaluate_routing_suite(args, client, chat_url, &dir, model_id).await?;
        save_stage_score_note(&dir, "stage1_routing", &note)?;
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
        stage1_scores.push(candidate);
    }
    beam = select_top_search_beam(stage1_scores, beam_width);
    if beam.is_empty() {
        beam.push(best_search.clone());
    }
    if let Some(top) = beam.first() {
        if top.score - best_stage_score < 0.02 {
            stagnant_rounds += 1;
        } else {
            stagnant_rounds = 0;
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
                // Section B: Validate mutation boundaries
                validate_tuning_mutations(&parent.dir, &dir)
                    .with_context(|| format!("router variant '{variant}' violated tuning boundaries"))?;
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
                // Section B: Validate mutation boundaries
                validate_tuning_mutations(&parent.dir, &dir)
                    .with_context(|| format!("orch variant '{variant}' violated tuning boundaries"))?;
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
                // Section B: Validate mutation boundaries
                validate_tuning_mutations(&parent.dir, &dir)
                    .with_context(|| format!("response variant '{variant}' violated tuning boundaries"))?;
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

    // === Protected Baseline Full Evaluation (Section A) ===
    // Evaluate the baseline with the same full calibration so we have a
    // comparable score for the safer activation policy.
    calibration_progress(
        args,
        &format!("tune baseline validation: scoring protected baseline for {model_id}"),
    );
    let baseline_full = evaluate_candidate_dir(
        args,
        client,
        chat_url,
        base_url,
        &baseline_dir,
        model_id,
        false,
    )
    .await?;
    let baseline_full_score = baseline_full.score;

    // Save baseline artifacts for traceability
    let winner_dir = run_root.join("winner");
    snapshot_active_profile_set(&search_winner.dir, &winner_dir)?;
    let baseline_winner_dir = run_root.join("baseline_evaluated");
    snapshot_active_profile_set(&baseline_dir, &baseline_winner_dir)?;

    // === Safer Activation Policy (Section E) ===
    // A candidate is only activated when it meaningfully outperforms the
    // protected baseline. If improvement is marginal, the baseline is preferred.
    let (should_activate, reason) = activation_reason(
        best_overall.score,
        baseline_full_score,
        best_overall.report.summary.certified,
    );

    let (activation_src, activation_dir, final_score, final_certified) = if should_activate {
        calibration_progress(
            args,
            &format!("tune activating winner ({}) — {}", search_winner.name, reason),
        );
        (
            "tune",
            &search_winner.dir,
            best_overall.score,
            best_overall.report.summary.certified,
        )
    } else {
        calibration_progress(
            args,
            &format!("tune preferring baseline — {}", reason),
        );
        (
            "tune_baseline_preferred",
            &baseline_dir,
            baseline_full_score,
            baseline_full.report.summary.certified,
        )
    };

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
        baseline_full_score,
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
            baseline_score: baseline_full_score,
        },
    )?;

    // Return the winner for upstream reporting (even if baseline was preferred,
    // return the candidate that was actually activated).
    if should_activate {
        Ok(best_overall)
    } else {
        Ok(baseline_full)
    }
}

