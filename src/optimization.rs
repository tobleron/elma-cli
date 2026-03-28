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
        },
    )?;

    let baseline_dir = make_candidate_dir(&run_root, "00_baseline")?;
    snapshot_active_profile_set(model_cfg_dir, &baseline_dir)?;
    let (baseline_score, baseline_reject, baseline_note) =
        evaluate_routing_suite(client, chat_url, &baseline_dir, model_id).await?;
    save_stage_score_note(&baseline_dir, "stage1_routing", &baseline_note)?;
    let baseline = SearchCandidate {
        name: "00_baseline".to_string(),
        dir: baseline_dir,
        score: baseline_score,
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
        sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
        let (score, hard_rejected, note) =
            evaluate_routing_suite(client, chat_url, &dir, model_id).await?;
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
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_routing_suite(client, chat_url, &dir, model_id).await?;
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

    let winner_dir = run_root.join("winner");
    snapshot_active_profile_set(&search_winner.dir, &winner_dir)?;
    activate_profile_set(
        model_cfg_dir,
        &search_winner.dir,
        base_url,
        model_id,
        "tune",
        Some(run_id.clone()),
        best_overall.score,
        best_overall.report.summary.certified,
    )?;
    save_tune_run_manifest(
        &run_root.join("run_manifest.toml"),
        &TuneRunManifest {
            version: 1,
            run_id,
            model: model_id.to_string(),
            mode: "tune".to_string(),
            started_unix_s: now_unix_s()?,
            activated: true,
            final_score: best_overall.score,
            certified: best_overall.report.summary.certified,
        },
    )?;
    Ok(best_overall)
}
