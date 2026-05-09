use crate::*;
pub(crate) type WorkflowPlan = WorkflowPlannerOutput;
use crate::tune::TuneResources;
use std::path::Path;

pub(crate) async fn orchestrate_and_evaluate_program(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _orchestrator_cfg: &Profile,
    _user_message: &str,
    _decision: &RouteDecision,
    _workflow_plan: Option<&WorkflowPlan>,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _ws: &str,
    _ws_brief: &str,
    _messages: &[ChatMessage],
    _scenario: &CalibrationScenario,
    _args: &Args,
) -> Result<(Option<Program>, ProgramEvaluation)> {
    Ok((None, ProgramEvaluation::default()))
}

pub(crate) async fn check_program_consistency(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _orchestrator_cfg: &Profile,
    _user_message: &str,
    _decision: &RouteDecision,
    _workflow_plan: Option<&WorkflowPlan>,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _ws: &str,
    _ws_brief: &str,
    _messages: &[ChatMessage],
    _program: &Option<Program>,
) -> bool {
    true
}

pub(crate) async fn execute_and_evaluate_program(
    _args: &Args,
    _client: &reqwest::Client,
    _chat_url: &Url,
    _resources: &TuneResources,
    _scenario: &CalibrationScenario,
    _user_message: &str,
    _decision: &RouteDecision,
    _workflow_plan: Option<&WorkflowPlan>,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _messages: &[ChatMessage],
    _program: Program,
    _actual_step_count: usize,
) -> Result<(bool, Option<bool>, Option<bool>, Option<String>, Option<bool>, Option<String>, Option<bool>, Option<bool>, Option<String>, Option<bool>, Option<String>, Option<bool>, Option<String>, Option<bool>, Option<String>)> {
    Ok((false, None, None, None, None, None, None, None, None, None, None, None, None, None, None))
}

pub(crate) fn probability_of(distribution: &HashMap<String, f64>, key: &str) -> f64 {
    *distribution.get(key).unwrap_or(&0.0)
}

pub(crate) fn score_calibration_report(_report: &CalibrationReport) -> f64 {
    0.0
}

pub(crate) fn evaluate_scope(_scope: &ScopePlan, _scenario: &CalibrationScenario) -> (bool, String) {
    (true, "Mock scope evaluation".to_string())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn check_execution_sufficiency_once(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _cfg: &Profile,
    _user_message: &str,
    _decision: &RouteDecision,
    _program: &Program,
    _step_results: &[StepResult],
) -> Result<ExecutionSufficiencyVerdict> {
    Ok(ExecutionSufficiencyVerdict::default())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn generate_final_answer_once(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _p1: &Profile,
    _p2: &Profile,
    _p3: &Profile,
    _p4: &Profile,
    _p5: &Profile,
    _p6: &Profile,
    _s1: &str,
    _s2: &str,
    _s3: &str,
    _s4: &str,
    _decision: &RouteDecision,
    _step_results: &[StepResult],
    _reply_instructions: &str,
    _s5: &str,
    _s6: &str,
    _interrupt: Option<InterruptBehavior>,
) -> Result<(String, String)> {
    Ok((String::new(), String::new()))
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn judge_final_answer_once(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _profile: &Profile,
    _scenario: &CalibrationScenario,
    _user_message: &str,
    _step_results: &[StepResult],
    _final_answer: &str,
) -> Result<OutcomeVerificationVerdict> {
    Ok(OutcomeVerificationVerdict::default())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn infer_route_prior(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _speech_act_cfg: &Profile,
    _router_cfg: &Profile,
    _mode_router_cfg: &Profile,
    _cal: &RouterCalibration,
    _user_message: &str,
    _ws: &str,
    _ws_brief: &str,
    _messages: &[ChatMessage],
    _forced: Option<RouteDecision>,
) -> Result<RouteDecision> {
    Ok(RouteDecision::default())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn derive_planning_prior(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _orchestrator_cfg: &Profile,
    _complexity_cfg: &Profile,
    _workflow_cfg: &Profile,
    _formula_cfg: &Profile,
    _user_message: &str,
    _decision: &RouteDecision,
    _ws: &str,
    _ws_brief: &str,
    _memories: &Vec<FormulaMemoryRecord>,
    _messages: &[ChatMessage],
) -> Result<(WorkflowPlan, ComplexityAssessment, ScopePlan, FormulaSelection, String)> {
    Ok((WorkflowPlan::default(), ComplexityAssessment::default(), ScopePlan::default(), FormulaSelection::default(), String::new()))
}

pub(crate) fn activation_reason(
    _score: f64,
    _baseline: f64,
    _certified: bool,
) -> (bool, String) {
    (false, "Mock activation reason".to_string())
}

pub(crate) fn apply_router_param_variant(_dir: &Path, _variant: &str) -> Result<()> {
    Ok(())
}

pub(crate) fn apply_orchestrator_param_variant(_dir: &Path, _variant: &str) -> Result<()> {
    Ok(())
}

pub(crate) fn apply_response_param_variant(_dir: &Path, _variant: &str) -> Result<()> {
    Ok(())
}

pub(crate) fn validate_tuning_mutations(_parent: &Path, _dir: &Path) -> Result<()> {
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn orchestrate_program_once(
    _client: &reqwest::Client,
    _chat_url: &Url,
    _orchestrator_cfg: &Profile,
    _user_message: &str,
    _decision: &RouteDecision,
    _workflow_plan: Option<&WorkflowPlan>,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _ws: &str,
    _ws_brief: &str,
    _messages: &[ChatMessage],
) -> Result<(Program, ProgramEvaluation)> {
    Ok((Program { objective: String::new(), steps: Vec::new() }, ProgramEvaluation::default()))
}

pub(crate) fn apply_capability_guard(
    _program: &mut Program,
    _decision: &RouteDecision,
    _is_tune: bool,
) -> Result<()> {
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_autonomous_loop(
    _args: &Args,
    _client: &reqwest::Client,
    _chat_url: &Url,
    _session: &crate::session_paths::SessionPaths,
    _repo: &std::path::Path,
    _program: Program,
    _decision: &RouteDecision,
    _workflow_plan: Option<&WorkflowPlan>,
    _complexity: &ComplexityAssessment,
    _scope: &ScopePlan,
    _formula: &FormulaSelection,
    _ws: &str,
    _ws_brief: &str,
    _messages: &[ChatMessage],
    _orchestrator_cfg: &Profile,
    _status_message_cfg: &Profile,
    _planner_cfg: &Profile,
    _planner_master_cfg: &Profile,
    _decider_cfg: &Profile,
    _selector_cfg: &Profile,
    _summarizer_cfg: &Profile,
    _command_repair_cfg: &Profile,
    _command_preflight_cfg: &Profile,
    _task_semantics_guard_cfg: &Profile,
    _evidence_compactor_cfg: &Profile,
    _artifact_classifier_cfg: &Profile,
    _outcome_verifier_cfg: &Profile,
    _execution_sufficiency_cfg: &Profile,
    _critic_cfg: &Profile,
    _logical_reviewer_cfg: &Profile,
    _efficiency_reviewer_cfg: &Profile,
    _risk_reviewer_cfg: &Profile,
    _refinement_cfg: &Profile,
    _interrupt: Option<InterruptBehavior>,
) -> Result<crate::types_api::AutonomousLoopOutcome> {
    Err(anyhow::anyhow!("Mock autonomous loop failed"))
}

pub(crate) fn apply_runtime_generation_defaults(_dir: &Path, _defaults: &RuntimeGenerationDefaults) -> Result<()> {
    Ok(())
}

pub(crate) fn hard_rejects_calibration_report(_report: &CalibrationReport) -> bool {
    false
}
