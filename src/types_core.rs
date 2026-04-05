//! @efficiency-role: data-model
//!
//! Types - Core Types and Step Definitions

use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Goal state for multi-turn task persistence (Task 014)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct GoalState {
    #[serde(default)]
    pub active_objective: Option<String>,
    #[serde(default)]
    pub completed_subgoals: Vec<String>,
    #[serde(default)]
    pub pending_subgoals: Vec<String>,
    #[serde(default)]
    pub blocked_reason: Option<String>,
    #[serde(default)]
    pub created_at: u64,
    #[serde(default)]
    pub last_updated: u64,
}

#[derive(Parser, Debug, Clone)]
#[command(
    name = "elma-cli",
    version,
    about = "Minimal chat CLI for llama.cpp /v1/chat/completions"
)]
pub(crate) struct Args {
    #[arg(long, env = "LLAMA_BASE_URL")]
    pub(crate) base_url: Option<String>,
    #[arg(long, env = "LLAMA_MODEL")]
    pub(crate) model: Option<String>,
    #[arg(long, default_value = "config")]
    pub(crate) config_root: String,
    #[arg(long, default_value = "sessions")]
    pub(crate) sessions_root: String,
    #[arg(long, default_value_t = true)]
    pub(crate) show_thinking: bool,
    #[arg(long, default_value_t = false)]
    pub(crate) no_color: bool,
    #[arg(long, default_value_t = true, env = "ELMA_SHOW_PROCESS")]
    pub(crate) show_process: bool,
    #[arg(long, default_value_t = 0.2)]
    pub(crate) retry_temp_step: f64,
    #[arg(long, default_value_t = 1.2)]
    pub(crate) max_retry_temp: f64,
    #[arg(long, default_value_t = 2)] // Reduced from 4 to prevent reviewer retry loops
    pub(crate) max_retries: u32,
    #[arg(long, default_value_t = 0.7)]
    pub(crate) temp_critic: f64,
    #[arg(long, default_value_t = 0.7)]
    pub(crate) temp_judges: f64,
    #[arg(long, default_value_t = 0.6)]
    pub(crate) temp_reviewers: f64,
    #[arg(long, default_value_t = 0.5)]
    pub(crate) temp_routers: f64,
    #[arg(long, default_value_t = 0.4)]
    pub(crate) temp_gates: f64,
    #[arg(long, default_value = "quick", value_parser = ["quick", "full"])]
    pub(crate) tune_mode: String,
    #[arg(long, default_value_t = false)]
    pub(crate) tune: bool,
    #[arg(long, default_value_t = false)]
    pub(crate) calibrate: bool,
    #[arg(long, default_value_t = false)]
    pub(crate) all_models: bool,
    #[arg(long, default_value_t = false)]
    pub(crate) restore_base: bool,
    #[arg(long, default_value_t = false)]
    pub(crate) restore_last: bool,
    #[arg(long, default_value_t = false)]
    pub(crate) debug_trace: bool,
    #[arg(long, default_value_t = true, env = "ELMA_DISABLE_GUARDS")]
    pub(crate) disable_guards: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Profile {
    pub(crate) version: u32,
    pub(crate) name: String,
    pub(crate) base_url: String,
    pub(crate) model: String,
    pub(crate) temperature: f64,
    pub(crate) top_p: f64,
    pub(crate) repeat_penalty: f64,
    pub(crate) reasoning_format: String,
    pub(crate) max_tokens: u32,
    pub(crate) timeout_s: u64,
    pub(crate) system_prompt: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RouterCalibration {
    pub(crate) version: u32,
    pub(crate) model: String,
    pub(crate) base_url: String,
    pub(crate) n_probs: u32,
    pub(crate) supports_logprobs: bool,
    pub(crate) routes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct GlobalConfig {
    pub(crate) version: u32,
    pub(crate) base_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ModelBehaviorProfile {
    pub(crate) version: u32,
    pub(crate) model: String,
    pub(crate) base_url: String,
    pub(crate) auto_reasoning_separated: bool,
    pub(crate) auto_final_clean: bool,
    pub(crate) auto_truncated_before_final: bool,
    pub(crate) none_final_clean: bool,
    pub(crate) none_reasoning_leak_suspected: bool,
    pub(crate) json_clean_with_auto: bool,
    pub(crate) json_clean_with_none: bool,
    pub(crate) needs_text_finalizer: bool,
    pub(crate) preferred_reasoning_format: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct RuntimeGenerationDefaults {
    #[serde(default)]
    pub(crate) temperature: Option<f64>,
    #[serde(default)]
    pub(crate) top_p: Option<f64>,
    #[serde(default)]
    pub(crate) repeat_penalty: Option<f64>,
    #[serde(default)]
    pub(crate) max_tokens: Option<u32>,
    #[serde(default)]
    pub(crate) source: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ActiveManifest {
    pub(crate) version: u32,
    pub(crate) model: String,
    pub(crate) active_source: String,
    pub(crate) active_run_id: Option<String>,
    pub(crate) activated_unix_s: u64,
    pub(crate) final_score: f64,
    pub(crate) certified: bool,
    pub(crate) restore_last_dir: String,
    pub(crate) restore_base_dir: String,
    #[serde(default)]
    pub(crate) activation_reason: String,
    #[serde(default)]
    pub(crate) baseline_score: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TuneRunManifest {
    pub(crate) version: u32,
    pub(crate) run_id: String,
    pub(crate) model: String,
    pub(crate) mode: String,
    pub(crate) started_unix_s: u64,
    pub(crate) activated: bool,
    pub(crate) final_score: f64,
    pub(crate) certified: bool,
    #[serde(default)]
    pub(crate) activation_reason: String,
    #[serde(default)]
    pub(crate) baseline_score: f64,
    /// Task 046: Track system prompt hashes to detect when re-tuning is needed
    #[serde(default)]
    pub(crate) prompt_hashes: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct StabilitySummary {
    pub(crate) runs: usize,
    pub(crate) mean_score: f64,
    pub(crate) min_score: f64,
    pub(crate) max_score: f64,
    pub(crate) stddev: f64,
    pub(crate) penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BaselineAnchorReport {
    pub(crate) name: String,
    pub(crate) source: String,
    pub(crate) raw_score: f64,
    pub(crate) adjusted_score: f64,
    pub(crate) certified: bool,
    pub(crate) hard_rejected: bool,
    pub(crate) stability: StabilitySummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TuneDecisionReport {
    pub(crate) version: u32,
    pub(crate) model: String,
    pub(crate) selected_name: String,
    pub(crate) selected_source: String,
    pub(crate) selected_raw_score: f64,
    pub(crate) selected_adjusted_score: f64,
    pub(crate) protected_baseline_name: String,
    pub(crate) protected_baseline_adjusted_score: f64,
    pub(crate) activation_reason: String,
    pub(crate) baselines: Vec<BaselineAnchorReport>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CalibrationManifest {
    pub(crate) version: u32,
    pub(crate) scenarios: Vec<CalibrationScenario>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CalibrationScenario {
    #[serde(default)]
    pub(crate) suite: String,
    pub(crate) file: String,
    pub(crate) speech_act: String,
    pub(crate) workflow: String,
    #[serde(default)]
    pub(crate) mode: Option<String>,
    pub(crate) route: String,
    #[serde(default)]
    pub(crate) notes: String,
    #[serde(default = "default_runtime_safe")]
    pub(crate) runtime_safe: bool,
    #[serde(default)]
    pub(crate) expected_formula: Option<String>,
    #[serde(default)]
    pub(crate) expected_scope_terms: Vec<String>,
    #[serde(default)]
    pub(crate) forbidden_scope_terms: Vec<String>,
    #[serde(default)]
    pub(crate) expected_answer_keywords: Vec<String>,
    #[serde(default)]
    pub(crate) avoid_answer_keywords: Vec<String>,
    #[serde(default)]
    pub(crate) expected_categories: Vec<String>,
    #[serde(default)]
    pub(crate) minimum_step_count: Option<usize>,
    #[serde(default)]
    pub(crate) maximum_step_count: Option<usize>,
}

fn default_runtime_safe() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CalibrationMetric {
    pub(crate) total: usize,
    pub(crate) correct: usize,
    pub(crate) accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CalibrationConfusion {
    pub(crate) expected: String,
    pub(crate) predicted: String,
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScenarioCalibrationResult {
    pub(crate) suite: String,
    pub(crate) file: String,
    pub(crate) notes: String,
    pub(crate) speech_act_expected: String,
    pub(crate) speech_act_predicted: String,
    pub(crate) speech_act_probability: f64,
    pub(crate) speech_act_ok: bool,
    pub(crate) workflow_expected: String,
    pub(crate) workflow_predicted: String,
    pub(crate) workflow_probability: f64,
    pub(crate) workflow_ok: bool,
    pub(crate) mode_expected: Option<String>,
    pub(crate) mode_predicted: Option<String>,
    pub(crate) mode_probability: Option<f64>,
    pub(crate) mode_ok: Option<bool>,
    pub(crate) route_expected: String,
    pub(crate) route_predicted: String,
    pub(crate) route_probability: f64,
    pub(crate) route_ok: bool,
    pub(crate) program_signature: String,
    pub(crate) program_parse_ok: bool,
    pub(crate) program_parse_error: String,
    pub(crate) program_shape_ok: bool,
    pub(crate) program_shape_reason: String,
    pub(crate) program_policy_ok: bool,
    pub(crate) program_policy_reason: String,
    pub(crate) program_consistency_ok: bool,
    pub(crate) executed_in_tune: bool,
    pub(crate) execution_ok: Option<bool>,
    pub(crate) critic_ok: Option<bool>,
    pub(crate) critic_reason: Option<String>,
    pub(crate) response_ok: Option<bool>,
    pub(crate) response_reason: Option<String>,
    pub(crate) response_plain_text: Option<bool>,
    pub(crate) scope_ok: Option<bool>,
    pub(crate) scope_reason: Option<String>,
    pub(crate) compaction_ok: Option<bool>,
    pub(crate) compaction_reason: Option<String>,
    pub(crate) classification_ok: Option<bool>,
    pub(crate) classification_reason: Option<String>,
    pub(crate) claim_check_ok: Option<bool>,
    pub(crate) claim_check_reason: Option<String>,
    pub(crate) presentation_ok: Option<bool>,
    pub(crate) presentation_reason: Option<String>,
    pub(crate) tool_economy_score: Option<f64>,
    pub(crate) all_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CalibrationSummary {
    pub(crate) total_cases: usize,
    pub(crate) speech_act: CalibrationMetric,
    pub(crate) workflow: CalibrationMetric,
    pub(crate) mode: CalibrationMetric,
    pub(crate) route: CalibrationMetric,
    pub(crate) program_parse: CalibrationMetric,
    pub(crate) program_shape: CalibrationMetric,
    pub(crate) program_policy: CalibrationMetric,
    pub(crate) program_consistency: CalibrationMetric,
    pub(crate) execution: CalibrationMetric,
    pub(crate) critic: CalibrationMetric,
    pub(crate) response: CalibrationMetric,
    pub(crate) scope: CalibrationMetric,
    pub(crate) compaction: CalibrationMetric,
    pub(crate) classification: CalibrationMetric,
    pub(crate) claim_check: CalibrationMetric,
    pub(crate) presentation: CalibrationMetric,
    pub(crate) all_ok: CalibrationMetric,
    pub(crate) certified: bool,
    pub(crate) certification_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CalibrationReport {
    pub(crate) version: u32,
    pub(crate) model: String,
    pub(crate) base_url: String,
    pub(crate) supports_logprobs: bool,
    pub(crate) n_probs: u32,
    pub(crate) summary: CalibrationSummary,
    pub(crate) speech_act_confusions: Vec<CalibrationConfusion>,
    pub(crate) workflow_confusions: Vec<CalibrationConfusion>,
    pub(crate) mode_confusions: Vec<CalibrationConfusion>,
    pub(crate) route_confusions: Vec<CalibrationConfusion>,
    pub(crate) scenarios: Vec<ScenarioCalibrationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EfficiencyMetric {
    pub(crate) total: usize,
    pub(crate) score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EfficiencyScenarioResult {
    pub(crate) suite: String,
    pub(crate) file: String,
    pub(crate) task_success: bool,
    pub(crate) grounding_ok: Option<bool>,
    pub(crate) scope_ok: Option<bool>,
    pub(crate) compaction_ok: Option<bool>,
    pub(crate) classification_ok: Option<bool>,
    pub(crate) claim_check_ok: Option<bool>,
    pub(crate) presentation_ok: Option<bool>,
    pub(crate) tool_economy_score: f64,
    pub(crate) actual_steps: usize,
    pub(crate) expected_min_steps: Option<usize>,
    pub(crate) expected_max_steps: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EfficiencySummary {
    pub(crate) total_cases: usize,
    pub(crate) task_success_rate: EfficiencyMetric,
    pub(crate) grounding_rate: EfficiencyMetric,
    pub(crate) scope_precision: EfficiencyMetric,
    pub(crate) compaction_rate: EfficiencyMetric,
    pub(crate) classification_rate: EfficiencyMetric,
    pub(crate) claim_check_rate: EfficiencyMetric,
    pub(crate) presentation_rate: EfficiencyMetric,
    pub(crate) tool_economy: EfficiencyMetric,
    pub(crate) overall_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EfficiencyReport {
    pub(crate) version: u32,
    pub(crate) model: String,
    pub(crate) base_url: String,
    pub(crate) summary: EfficiencySummary,
    pub(crate) scenarios: Vec<EfficiencyScenarioResult>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProgramEvaluation {
    pub(crate) parsed: bool,
    pub(crate) parse_error: String,
    pub(crate) shape_ok: bool,
    pub(crate) shape_reason: String,
    pub(crate) policy_ok: bool,
    pub(crate) policy_reason: String,
    pub(crate) executable_in_tune: bool,
    pub(crate) signature: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CalibrationJudgeVerdict {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) answered_request: bool,
    #[serde(default)]
    pub(crate) faithful_to_evidence: bool,
    #[serde(default)]
    pub(crate) plain_text: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CandidateScore {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
    pub(crate) report: CalibrationReport,
    pub(crate) score: f64,
    pub(crate) hard_rejected: bool,
    // Task 009: Variance/stability tracking
    pub(crate) variance: f64,
    pub(crate) std_dev: f64,
    pub(crate) parse_failure_count: usize,
    pub(crate) latency_avg_ms: f64,
}

#[derive(Debug, Clone)]
pub(crate) struct SearchCandidate {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
    pub(crate) score: f64,
    pub(crate) hard_rejected: bool,
    // Task 009: Variance/stability tracking
    pub(crate) variance: f64,
    pub(crate) std_dev: f64,
}

/// Parameter search bands by unit type (Task 009)
#[derive(Debug, Clone)]
pub(crate) struct ParameterBands {
    pub temperature: (f64, f64),
    pub top_p: (f64, f64),
    pub repeat_penalty: (f64, f64),
    pub max_tokens: (u32, u32),
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub(crate) struct RouteDecision {
    pub(crate) route: String,
    pub(crate) source: String,
    pub(crate) distribution: Vec<(String, f64)>,
    pub(crate) margin: f64,
    pub(crate) entropy: f64,
    pub(crate) speech_act: ProbabilityDecision,
    pub(crate) workflow: ProbabilityDecision,
    pub(crate) mode: ProbabilityDecision,
}

#[derive(Debug, Clone)]
pub(crate) struct ClassificationFeatures {
    pub(crate) speech_act_probs: Vec<(String, f64)>,
    pub(crate) workflow_probs: Vec<(String, f64)>,
    pub(crate) mode_probs: Vec<(String, f64)>,
    pub(crate) route_probs: Vec<(String, f64)>,
    pub(crate) entropy: f64,
    pub(crate) suggested_route: String,
}

impl From<&RouteDecision> for ClassificationFeatures {
    fn from(decision: &RouteDecision) -> Self {
        Self {
            speech_act_probs: decision.speech_act.distribution.clone(),
            workflow_probs: decision.workflow.distribution.clone(),
            mode_probs: decision.mode.distribution.clone(),
            route_probs: decision.distribution.clone(),
            entropy: decision.entropy,
            suggested_route: decision.route.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub(crate) struct ProbabilityDecision {
    pub(crate) choice: String,
    pub(crate) source: String,
    pub(crate) distribution: Vec<(String, f64)>,
    pub(crate) margin: f64,
    pub(crate) entropy: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct Program {
    pub(crate) objective: String,
    pub(crate) steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct StepCommon {
    #[serde(default)]
    pub(crate) purpose: String,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    #[serde(default)]
    pub(crate) success_condition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) parent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) depth: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) unit_type: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct EditSpec {
    #[serde(default)]
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) operation: String,
    #[serde(default)]
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) find: String,
    #[serde(default)]
    pub(crate) replace: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type")]
pub(crate) enum Step {
    #[serde(rename = "shell")]
    Shell {
        id: String,
        cmd: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "read")]
    Read {
        id: String,
        path: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "search")]
    Search {
        id: String,
        query: String,
        paths: Vec<String>,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "select")]
    Select {
        id: String,
        #[serde(default)]
        instructions: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "plan")]
    Plan {
        id: String,
        goal: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "masterplan")]
    MasterPlan {
        id: String,
        goal: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "decide")]
    Decide {
        id: String,
        prompt: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "summarize")]
    Summarize {
        id: String,
        #[serde(default)]
        text: String,
        #[serde(default)]
        instructions: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "edit")]
    Edit {
        id: String,
        #[serde(flatten)]
        spec: EditSpec,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "reply")]
    Reply {
        id: String,
        instructions: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    /// Respond — answer grounded in executed step evidence
    #[serde(rename = "respond")]
    Respond {
        id: String,
        #[serde(default)]
        instructions: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    /// Explore — map unfamiliar code, form hypotheses, test them
    #[serde(rename = "explore")]
    Explore {
        id: String,
        #[serde(default)]
        objective: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    /// Write — create new file content
    #[serde(rename = "write")]
    Write {
        id: String,
        path: String,
        content: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    /// Delete — remove file
    #[serde(rename = "delete")]
    Delete {
        id: String,
        path: String,
        #[serde(flatten)]
        common: StepCommon,
    },
}
