use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(
    name = "elma-cli",
    version,
    about = "Minimal chat CLI for llama.cpp /v1/chat/completions"
)]
pub(crate) struct Args {
    /// Base URL of the server (example: http://192.168.1.186:8080)
    #[arg(long, env = "LLAMA_BASE_URL")]
    pub(crate) base_url: Option<String>,

    /// Optional model override. If omitted, we fetch the first model id from GET /v1/models.
    #[arg(long, env = "LLAMA_MODEL")]
    pub(crate) model: Option<String>,

    /// Root config directory (model-specific folders will be created under it).
    #[arg(long, default_value = "config")]
    pub(crate) config_root: String,

    /// Root sessions directory.
    #[arg(long, default_value = "sessions")]
    pub(crate) sessions_root: String,

    /// Print model thinking (reasoning_content) if present.
    #[arg(long, default_value_t = true)]
    pub(crate) show_thinking: bool,

    /// Disable ANSI colors.
    #[arg(long, default_value_t = false)]
    pub(crate) no_color: bool,

    /// Run tuning for all models exposed by the endpoint, then exit.
    #[arg(long, default_value_t = false)]
    pub(crate) tune: bool,

    /// Run calibration only for the selected model(s), then exit.
    #[arg(long, default_value_t = false)]
    pub(crate) calibrate: bool,

    /// When tuning or calibrating, target all models exposed by the endpoint.
    #[arg(long, default_value_t = false)]
    pub(crate) all_models: bool,

    /// Restore the immutable baseline profile set for the selected model, then exit.
    #[arg(long, default_value_t = false)]
    pub(crate) restore_base: bool,

    /// Restore the last active profile set for the selected model, then exit.
    #[arg(long, default_value_t = false)]
    pub(crate) restore_last: bool,

    /// Show raw machine trace lines in the terminal.
    #[arg(long, default_value_t = false)]
    pub(crate) debug_trace: bool,

    /// Disable hard constraints (capability guards, formula locks) for autonomous reasoning.
    /// Guards remain in code but are bypassed when this flag is set.
    #[arg(long, default_value_t = false, env = "ELMA_DISABLE_GUARDS")]
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
}

#[derive(Debug, Clone)]
pub(crate) struct SearchCandidate {
    pub(crate) name: String,
    pub(crate) dir: PathBuf,
    pub(crate) score: f64,
    pub(crate) hard_rejected: bool,
}

#[derive(Debug, Clone)]
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

/// Classification features for autonomous reasoning
/// 
/// DESIGN RATIONALE:
/// These features are provided as SOFT EVIDENCE, not hard constraints.
/// This change is intentional to test Elma's autonomous reasoning capabilities
/// and reduce deterministic practices. The orchestrator should:
/// 1. Consider these probabilities as signals, not rules
/// 2. Override priors when task requirements demand it
/// 3. Reason about the actual user request, not just follow classifications
/// 
/// Previous behavior (classifications as hard decisions) made Elma behave
/// like a rule engine. This change enables genuine autonomous reasoning.
#[derive(Debug, Clone)]
pub(crate) struct ClassificationFeatures {
    /// Speech act probabilities (CAPABILITY_CHECK, INFO_REQUEST, ACTION_REQUEST)
    pub(crate) speech_act_probs: Vec<(String, f64)>,
    /// Workflow probabilities (CHAT, WORKFLOW)
    pub(crate) workflow_probs: Vec<(String, f64)>,
    /// Mode probabilities (INSPECT, EXECUTE, PLAN, MASTERPLAN, DECIDE)
    pub(crate) mode_probs: Vec<(String, f64)>,
    /// Route probabilities (CHAT, SHELL, PLAN, MASTERPLAN, DECIDE)
    pub(crate) route_probs: Vec<(String, f64)>,
    /// Classification entropy (low = over-confident, high = uncertain)
    pub(crate) entropy: f64,
    /// Chosen route (for backwards compatibility, but treat as suggestion)
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

#[derive(Debug, Clone)]
pub(crate) struct ProbabilityDecision {
    pub(crate) choice: String,
    pub(crate) source: String,
    pub(crate) distribution: Vec<(String, f64)>,
    pub(crate) margin: f64,
    pub(crate) entropy: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Program {
    pub(crate) objective: String,
    pub(crate) steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct StepCommon {
    #[serde(default)]
    pub(crate) purpose: String,
    #[serde(default)]
    pub(crate) depends_on: Vec<String>,
    #[serde(default)]
    pub(crate) success_condition: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub(crate) enum Step {
    #[serde(rename = "shell")]
    Shell {
        id: String,
        cmd: String,
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
}

impl Step {
    pub(crate) fn id(&self) -> &str {
        match self {
            Step::Shell { id, .. } => id,
            Step::Select { id, .. } => id,
            Step::Plan { id, .. } => id,
            Step::MasterPlan { id, .. } => id,
            Step::Decide { id, .. } => id,
            Step::Summarize { id, .. } => id,
            Step::Edit { id, .. } => id,
            Step::Reply { id, .. } => id,
        }
    }
    
    pub(crate) fn kind(&self) -> &str {
        match self {
            Step::Shell { .. } => "shell",
            Step::Select { .. } => "select",
            Step::Plan { .. } => "plan",
            Step::MasterPlan { .. } => "masterplan",
            Step::Decide { .. } => "decide",
            Step::Summarize { .. } => "summarize",
            Step::Edit { .. } => "edit",
            Step::Reply { .. } => "reply",
        }
    }
    
    pub(crate) fn purpose(&self) -> &str {
        match self {
            Step::Shell { common, .. } => &common.purpose,
            Step::Select { common, .. } => &common.purpose,
            Step::Plan { common, .. } => &common.purpose,
            Step::MasterPlan { common, .. } => &common.purpose,
            Step::Decide { common, .. } => &common.purpose,
            Step::Summarize { common, .. } => &common.purpose,
            Step::Edit { common, .. } => &common.purpose,
            Step::Reply { common, .. } => &common.purpose,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct CriticVerdict {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) program: Option<Program>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RiskReviewVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentPlan {
    pub(crate) objective: String,
    pub(crate) current_program: Program,
    pub(crate) program_history: Vec<Program>,
    pub(crate) attempts: u32,
    pub(crate) executed_steps: usize,
    pub(crate) max_steps: usize,
    pub(crate) recovery_failures: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct AutonomousLoopOutcome {
    pub(crate) program: Program,
    pub(crate) step_results: Vec<StepResult>,
    pub(crate) final_reply: Option<String>,
    pub(crate) reasoning_clean: bool,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StepResult {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) purpose: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) success_condition: String,
    pub(crate) ok: bool,
    pub(crate) summary: String,
    pub(crate) command: Option<String>,
    pub(crate) raw_output: Option<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) output_bytes: Option<u64>,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) artifact_path: Option<String>,
    pub(crate) artifact_kind: Option<String>,
    pub(crate) outcome_status: Option<String>,
    pub(crate) outcome_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ComplexityAssessment {
    #[serde(default)]
    pub(crate) complexity: String,
    #[serde(default)]
    pub(crate) needs_evidence: bool,
    #[serde(default)]
    pub(crate) needs_tools: bool,
    #[serde(default)]
    pub(crate) needs_decision: bool,
    #[serde(default)]
    pub(crate) needs_plan: bool,
    #[serde(default)]
    pub(crate) risk: String,
    #[serde(default)]
    pub(crate) suggested_pattern: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct FormulaSelection {
    #[serde(default)]
    pub(crate) primary: String,
    #[serde(default)]
    pub(crate) alternatives: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) memory_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct WorkflowPlannerOutput {
    #[serde(default)]
    pub(crate) objective: String,
    #[serde(default)]
    pub(crate) complexity: String,
    #[serde(default)]
    pub(crate) risk: String,
    #[serde(default)]
    pub(crate) needs_evidence: bool,
    #[serde(default)]
    pub(crate) scope: ScopePlan,
    #[serde(default)]
    pub(crate) preferred_formula: String,
    #[serde(default)]
    pub(crate) alternatives: Vec<String>,
    #[serde(default)]
    pub(crate) memory_id: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct EvidenceModeDecision {
    #[serde(default)]
    pub(crate) mode: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct CommandRepair {
    #[serde(default)]
    pub(crate) cmd: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct RepairSemanticsVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ExecutionSufficiencyVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) program: Option<Program>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct OutcomeVerificationVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct MemoryGateVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct CommandPreflightVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) cmd: String,
    #[serde(default)]
    pub(crate) question: String,
    #[serde(default)]
    pub(crate) execution_mode: String,
    #[serde(default)]
    pub(crate) artifact_kind: String,
    #[serde(default)]
    pub(crate) preview_strategy: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct SelectionOutput {
    #[serde(default)]
    pub(crate) items: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ScopePlan {
    #[serde(default)]
    pub(crate) objective: String,
    #[serde(default)]
    pub(crate) focus_paths: Vec<String>,
    #[serde(default)]
    pub(crate) include_globs: Vec<String>,
    #[serde(default)]
    pub(crate) exclude_globs: Vec<String>,
    #[serde(default)]
    pub(crate) query_terms: Vec<String>,
    #[serde(default)]
    pub(crate) expected_artifacts: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct EvidenceCompact {
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) key_facts: Vec<String>,
    #[serde(default)]
    pub(crate) noise: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ArtifactClassification {
    #[serde(default)]
    pub(crate) safe: Vec<String>,
    #[serde(default)]
    pub(crate) maybe: Vec<String>,
    #[serde(default)]
    pub(crate) keep: Vec<String>,
    #[serde(default)]
    pub(crate) ignore: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ClaimCheckVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) unsupported_claims: Vec<String>,
    #[serde(default)]
    pub(crate) missing_points: Vec<String>,
    #[serde(default)]
    pub(crate) rewrite_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FormulaMemoryRecord {
    pub(crate) id: String,
    pub(crate) created_unix_s: u64,
    #[serde(default)]
    pub(crate) model_id: String,
    #[serde(default)]
    pub(crate) active_run_id: String,
    pub(crate) user_message: String,
    pub(crate) route: String,
    pub(crate) complexity: String,
    pub(crate) formula: String,
    pub(crate) objective: String,
    pub(crate) title: String,
    pub(crate) program_signature: String,
    #[serde(default)]
    pub(crate) last_success_unix_s: u64,
    #[serde(default)]
    pub(crate) last_failure_unix_s: u64,
    #[serde(default)]
    pub(crate) success_count: u64,
    #[serde(default)]
    pub(crate) failure_count: u64,
    #[serde(default)]
    pub(crate) disabled: bool,
    #[serde(default)]
    pub(crate) artifact_mode_capable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotManifest {
    pub(crate) version: u32,
    pub(crate) snapshot_id: String,
    pub(crate) created_unix_s: u64,
    pub(crate) automatic: bool,
    pub(crate) reason: String,
    pub(crate) repo_root: String,
    pub(crate) git_aware: bool,
    pub(crate) scope_mode: String,
    pub(crate) file_count: u64,
    pub(crate) files: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotCreateResult {
    pub(crate) snapshot_id: String,
    pub(crate) snapshot_dir: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) file_count: u64,
    pub(crate) automatic: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RollbackResult {
    pub(crate) snapshot_id: String,
    pub(crate) manifest_path: PathBuf,
    pub(crate) restored_files: u64,
    pub(crate) removed_files: u64,
    pub(crate) verified_files: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelsList {
    pub(crate) data: Option<Vec<ModelItem>>,
    pub(crate) models: Option<Vec<ModelItem>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelItem {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChatCompletionRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) temperature: f64,
    pub(crate) top_p: f64,
    pub(crate) stream: bool,
    pub(crate) max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) n_probs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repeat_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArtifactRecord {
    pub(crate) artifact_id: String,
    pub(crate) source_step_id: String,
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) bytes_written: u64,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) created_unix_s: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellExecutionResult {
    pub(crate) exit_code: i32,
    pub(crate) inline_text: String,
    pub(crate) bytes_written: u64,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) artifact_path: Option<PathBuf>,
    pub(crate) artifact_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
    pub(crate) choices: Vec<Choice>,
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) created: Option<i64>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) system_fingerprint: Option<String>,
    #[serde(default)]
    pub(crate) usage: Option<Usage>,
    #[serde(default)]
    pub(crate) timings: Option<Timings>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    pub(crate) message: ChoiceMessage,
    #[serde(default)]
    pub(crate) finish_reason: Option<String>,
    #[serde(default)]
    pub(crate) logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChoiceMessage {
    #[allow(dead_code)]
    pub(crate) role: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Usage {
    #[serde(default)]
    pub(crate) prompt_tokens: Option<u64>,
    #[serde(default)]
    pub(crate) completion_tokens: Option<u64>,
    #[serde(default)]
    pub(crate) total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Timings {
    #[serde(default)]
    pub(crate) prompt_n: Option<u64>,
    #[serde(default)]
    pub(crate) prompt_ms: Option<f64>,
    #[serde(default)]
    pub(crate) predicted_n: Option<u64>,
    #[serde(default)]
    pub(crate) predicted_ms: Option<f64>,
    #[serde(default)]
    pub(crate) predicted_per_second: Option<f64>,
    #[serde(default)]
    pub(crate) cache_n: Option<u64>,
}
