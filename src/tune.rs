use crate::app::LoadedProfiles;
use crate::*;
use std::collections::HashMap;

/// Task 046: Compute SHA256 hash of a system prompt
fn compute_prompt_hash(prompt: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    prompt.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Task 046: Compute hashes for all intel unit system prompts
pub(crate) fn compute_all_prompt_hashes(profiles: &LoadedProfiles) -> HashMap<String, String> {
    let mut hashes = HashMap::new();

    // Router prompts
    hashes.insert(
        "router".to_string(),
        compute_prompt_hash(&profiles.router_cfg.system_prompt),
    );
    hashes.insert(
        "mode_router".to_string(),
        compute_prompt_hash(&profiles.mode_router_cfg.system_prompt),
    );
    hashes.insert(
        "speech_act".to_string(),
        compute_prompt_hash(&profiles.speech_act_cfg.system_prompt),
    );

    // Intel unit prompts (only those that exist and are tuned)
    hashes.insert(
        "complexity_assessor".to_string(),
        compute_prompt_hash(&profiles.complexity_cfg.system_prompt),
    );
    hashes.insert(
        "formula_selector".to_string(),
        compute_prompt_hash(&profiles.formula_cfg.system_prompt),
    );
    hashes.insert(
        "workflow_planner".to_string(),
        compute_prompt_hash(&profiles.workflow_planner_cfg.system_prompt),
    );
    hashes.insert(
        "scope_builder".to_string(),
        compute_prompt_hash(&profiles.scope_builder_cfg.system_prompt),
    );
    hashes.insert(
        "evidence_mode".to_string(),
        compute_prompt_hash(&profiles.evidence_mode_cfg.system_prompt),
    );
    hashes.insert(
        "outcome_verifier".to_string(),
        compute_prompt_hash(&profiles.outcome_verifier_cfg.system_prompt),
    );
    hashes.insert(
        "memory_gate".to_string(),
        compute_prompt_hash(&profiles.memory_gate_cfg.system_prompt),
    );
    hashes.insert(
        "orchestrator".to_string(),
        compute_prompt_hash(&profiles.orchestrator_cfg.system_prompt),
    );
    hashes.insert(
        "critic".to_string(),
        compute_prompt_hash(&profiles.critic_cfg.system_prompt),
    );

    hashes
}

/// Task 046: Check if any intel unit prompts have changed since last tuning
pub(crate) fn check_prompt_changes(
    model_cfg_dir: &PathBuf,
    current_hashes: &HashMap<String, String>,
) -> Result<(bool, Vec<String>)> {
    // Try both .toml and .json formats
    let manifest_path = model_cfg_dir.join("tune").join("active_manifest.toml");
    let manifest_path_json = model_cfg_dir.join("tune").join("active_manifest.json");

    let manifest_path = if manifest_path.exists() {
        &manifest_path
    } else if manifest_path_json.exists() {
        &manifest_path_json
    } else {
        // No previous tuning, changes detected
        return Ok((true, vec![]));
    };

    let manifest_json = std::fs::read_to_string(manifest_path)?;
    let manifest: TuneRunManifest = if manifest_path.extension().map_or(false, |e| e == "json") {
        serde_json::from_str(&manifest_json)?
    } else {
        toml::from_str(&manifest_json)?
    };

    let mut changed_units = Vec::new();

    for (unit_name, current_hash) in current_hashes {
        if let Some(stored_hash) = manifest.prompt_hashes.get(unit_name) {
            if stored_hash != current_hash {
                changed_units.push(unit_name.clone());
            }
        } else {
            // New unit or old manifest without prompt_hashes, treat as changed
            changed_units.push(unit_name.clone());
        }
    }

    Ok((!changed_units.is_empty(), changed_units))
}

pub(crate) struct TuneResources {
    pub(crate) elma_cfg: Profile,
    pub(crate) router_cfg: Profile,
    pub(crate) mode_router_cfg: Profile,
    pub(crate) speech_act_cfg: Profile,
    pub(crate) status_message_cfg: Profile,
    pub(crate) planner_master_cfg: Profile,
    pub(crate) planner_cfg: Profile,
    pub(crate) decider_cfg: Profile,
    pub(crate) selector_cfg: Profile,
    pub(crate) summarizer_cfg: Profile,
    pub(crate) formatter_cfg: Profile,
    pub(crate) json_outputter_cfg: Profile,
    pub(crate) complexity_cfg: Profile,
    pub(crate) formula_cfg: Profile,
    pub(crate) workflow_planner_cfg: Profile,
    pub(crate) command_repair_cfg: Profile,
    pub(crate) command_preflight_cfg: Profile,
    pub(crate) task_semantics_guard_cfg: Profile,
    pub(crate) execution_sufficiency_cfg: Profile,
    pub(crate) scope_builder_cfg: Profile,
    pub(crate) evidence_compactor_cfg: Profile,
    pub(crate) artifact_classifier_cfg: Profile,
    pub(crate) evidence_mode_cfg: Profile,
    pub(crate) outcome_verifier_cfg: Profile,
    pub(crate) memory_gate_cfg: Profile,
    pub(crate) result_presenter_cfg: Profile,
    pub(crate) claim_checker_cfg: Profile,
    pub(crate) orchestrator_cfg: Profile,
    pub(crate) critic_cfg: Profile,
    pub(crate) logical_reviewer_cfg: Profile,
    pub(crate) efficiency_reviewer_cfg: Profile,
    pub(crate) risk_reviewer_cfg: Profile,
    pub(crate) refinement_cfg: Profile,
    pub(crate) calibration_judge_cfg: Profile,
    pub(crate) cal: RouterCalibration,
    pub(crate) supports_logprobs: bool,
    pub(crate) n_probs: u32,
    pub(crate) repo: PathBuf,
    pub(crate) ws: String,
    pub(crate) ws_brief: String,
    pub(crate) system_content: String,
    pub(crate) tune_sessions_root: PathBuf,
}

pub(crate) struct ScenarioRuntimeOutcome {
    pub(crate) speech_pair: (String, String),
    pub(crate) workflow_pair: (String, String),
    pub(crate) mode_pair: Option<(String, String)>,
    pub(crate) route_pair: (String, String),
    pub(crate) scenario_result: ScenarioCalibrationResult,
    pub(crate) efficiency_result: EfficiencyScenarioResult,
}

#[derive(Default)]
pub(crate) struct RuntimeAggregation {
    pub(crate) speech_pairs: Vec<(String, String)>,
    pub(crate) workflow_pairs: Vec<(String, String)>,
    pub(crate) mode_pairs: Vec<(String, String)>,
    pub(crate) route_pairs: Vec<(String, String)>,
    pub(crate) scenario_results: Vec<ScenarioCalibrationResult>,
    pub(crate) efficiency_scenarios: Vec<EfficiencyScenarioResult>,
    pub(crate) speech_correct: usize,
    pub(crate) workflow_correct: usize,
    pub(crate) mode_correct: usize,
    pub(crate) mode_total: usize,
    pub(crate) route_correct: usize,
    pub(crate) program_parse_correct: usize,
    pub(crate) program_shape_correct: usize,
    pub(crate) program_policy_correct: usize,
    pub(crate) program_consistency_correct: usize,
    pub(crate) execution_correct: usize,
    pub(crate) execution_total: usize,
    pub(crate) critic_correct: usize,
    pub(crate) critic_total: usize,
    pub(crate) response_correct: usize,
    pub(crate) response_total: usize,
    pub(crate) scope_correct: usize,
    pub(crate) scope_total: usize,
    pub(crate) compaction_correct: usize,
    pub(crate) compaction_total: usize,
    pub(crate) classification_correct: usize,
    pub(crate) classification_total: usize,
    pub(crate) claim_check_correct: usize,
    pub(crate) claim_check_total: usize,
    pub(crate) presentation_correct: usize,
    pub(crate) presentation_total: usize,
    pub(crate) all_ok_correct: usize,
}

impl RuntimeAggregation {
    pub(crate) fn push(&mut self, outcome: ScenarioRuntimeOutcome) {
        self.speech_pairs.push(outcome.speech_pair);
        self.workflow_pairs.push(outcome.workflow_pair);
        if let Some(mode_pair) = outcome.mode_pair {
            self.mode_pairs.push(mode_pair);
        }

        let result = outcome.scenario_result;
        self.route_pairs.push(outcome.route_pair);
        if result.speech_act_ok {
            self.speech_correct += 1;
        }
        if result.workflow_ok {
            self.workflow_correct += 1;
        }
        if let Some(mode_ok) = result.mode_ok {
            self.mode_total += 1;
            if mode_ok {
                self.mode_correct += 1;
            }
        }
        if result.route_ok {
            self.route_correct += 1;
        }
        if result.program_parse_ok {
            self.program_parse_correct += 1;
        }
        if result.program_shape_ok {
            self.program_shape_correct += 1;
        }
        if result.program_policy_ok {
            self.program_policy_correct += 1;
        }
        if result.program_consistency_ok {
            self.program_consistency_correct += 1;
        }
        if let Some(ok) = result.execution_ok {
            self.execution_total += 1;
            if ok {
                self.execution_correct += 1;
            }
        }
        if let Some(ok) = result.critic_ok {
            self.critic_total += 1;
            if ok {
                self.critic_correct += 1;
            }
        }
        if let Some(ok) = result.response_ok {
            self.response_total += 1;
            if ok {
                self.response_correct += 1;
            }
        }
        if let Some(ok) = result.scope_ok {
            self.scope_total += 1;
            if ok {
                self.scope_correct += 1;
            }
        }
        if let Some(ok) = result.compaction_ok {
            self.compaction_total += 1;
            if ok {
                self.compaction_correct += 1;
            }
        }
        if let Some(ok) = result.classification_ok {
            self.classification_total += 1;
            if ok {
                self.classification_correct += 1;
            }
        }
        if let Some(ok) = result.claim_check_ok {
            self.claim_check_total += 1;
            if ok {
                self.claim_check_correct += 1;
            }
        }
        if let Some(ok) = result.presentation_ok {
            self.presentation_total += 1;
            if ok {
                self.presentation_correct += 1;
            }
        }
        if result.all_ok {
            self.all_ok_correct += 1;
        }

        self.scenario_results.push(result);
        self.efficiency_scenarios.push(outcome.efficiency_result);
    }
}

pub(crate) async fn tune_model(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
    intention_tune_cfg: &Profile,
    emit_progress: bool,
) -> Result<()> {
    set_trace_log_path(Some(model_cfg_dir.join("trace_debug.log")));
    let resources = tune_setup::prepare_tune_resources(
        args,
        client,
        chat_url,
        base_url,
        model_cfg_dir,
        model_id,
        intention_tune_cfg,
        emit_progress,
    )
    .await?;
    let aggregation =
        tune_runtime::run_runtime_calibration(args, client, chat_url, &resources, emit_progress)
            .await?;
    tune_summary::write_tune_reports(
        args,
        model_cfg_dir,
        model_id,
        base_url,
        &resources,
        aggregation,
        emit_progress,
    )
}
