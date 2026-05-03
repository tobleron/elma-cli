//! @efficiency-role: domain-logic
//!
//! Execution Ladder Module
//!
//! Determines the minimum sufficient operational level before generating or executing a program.
//!
//! Elma starts at the lowest plausible level and escalates only when needed.
//!
//! Operational Ladder (top-to-bottom):
//! - MasterPlan: Strategic phased decomposition (multi-session, open-ended)
//! - Plan: Tactical ordered breakdown (bounded, dependencies matter)
//! - Task: Bounded local outcome (short action sequence, evidence chain)
//! - Action: Single primary operation (no decomposition needed)

use crate::intel_trait::*;
use crate::intel_units::*;
use crate::*;

mod depth;
pub use depth::{
    assessment_needs_decomposition, assessment_to_depth, calculate_confidence, depth_to_level,
    explicit_planning_signal, generate_level_reason, generate_strategy_hint, needs_revision_loop,
    strategic_signal, truncate_message,
};

// ============================================================================
// Execution Level
// ============================================================================

/// The minimum sufficient operational level for executing a request.
///
/// Elma starts at the lowest plausible level and escalates only when needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionLevel {
    /// One primary operation is sufficient.
    /// No decomposition, evidence chain, or dependency chain required.
    /// Example shape: one Shell/Read/Search/Select/Decide/Edit step + Reply
    Action,

    /// One bounded user outcome requiring a short sequence of actions.
    /// Evidence gathering and transformation are local.
    /// No explicit tactical planning artifact needed.
    /// Examples: Read→Summarize→Reply, Search→Read→Reply
    Task,

    /// Tactical ordered breakdown where order/dependencies matter.
    /// User explicitly asks for a plan, or task needs staged execution.
    /// Remains bounded in scope (single session or tightly coupled sessions).
    Plan,

    /// Strategic phased decomposition for open-ended, multi-session objectives.
    /// Major milestones, dependencies, or phased rollout required.
    /// Strategic decomposition IS the output or prerequisite.
    MasterPlan,
}

impl ExecutionLevel {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ExecutionLevel::Action => "Single primary operation (no decomposition needed)",
            ExecutionLevel::Task => "Bounded outcome requiring short action sequence",
            ExecutionLevel::Plan => "Tactical ordered breakdown (dependencies matter)",
            ExecutionLevel::MasterPlan => "Strategic phased decomposition (multi-session)",
        }
    }

    /// Check if this level requires planning structure
    pub fn requires_planning_structure(&self) -> bool {
        matches!(self, ExecutionLevel::Plan | ExecutionLevel::MasterPlan)
    }

    /// Check if this level allows direct execution
    pub fn allows_direct_execution(&self) -> bool {
        matches!(self, ExecutionLevel::Action | ExecutionLevel::Task)
    }
}

impl std::fmt::Display for ExecutionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutionLevel::Action => write!(f, "Action"),
            ExecutionLevel::Task => write!(f, "Task"),
            ExecutionLevel::Plan => write!(f, "Plan"),
            ExecutionLevel::MasterPlan => write!(f, "MasterPlan"),
        }
    }
}

// ============================================================================
// Execution Ladder Assessment
// ============================================================================

/// Result of assessing the minimum sufficient execution level.
///
/// This assessment becomes the operational bridge between:
/// - Classification/complexity analysis
/// - Program generation and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLadderAssessment {
    /// The chosen execution level
    pub level: ExecutionLevel,

    /// Human-readable justification for the level choice
    /// Used for session trace, debugging, and reflection
    pub reason: String,

    /// Whether evidence gathering is required before execution
    /// (maps to needs_evidence from ComplexityAssessment)
    pub requires_evidence: bool,

    /// Whether explicit ordering of steps matters
    /// (dependencies between steps, sequential execution required)
    pub requires_ordering: bool,

    /// Whether phased decomposition is required
    /// (multiple milestones, sessions, or strategic phases)
    pub requires_phases: bool,

    /// Whether a revision loop is anticipated
    /// (edit→verify→edit cycles, iterative refinement)
    pub requires_revision_loop: bool,

    /// Risk level (LOW/MEDIUM/HIGH)
    /// (preserved from ComplexityAssessment for compatibility)
    pub risk: String,

    /// Complexity classification
    /// (DIRECT/INVESTIGATE/MULTISTEP/OPEN_ENDED for backward compat)
    pub complexity: String,

    /// Optional hint for formula selection or planning strategy
    /// Examples: "start with search", "verify after edit", "phase by module"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_hint: Option<String>,

    /// Whether fallback was used in assessment
    #[serde(default)]
    pub fallback_used: bool,

    /// Confidence score (0.0 to 1.0)
    #[serde(default)]
    pub confidence: f64,
}

impl ExecutionLadderAssessment {
    /// Create assessment with default values
    pub fn new(
        level: ExecutionLevel,
        reason: String,
        requires_evidence: bool,
        requires_ordering: bool,
        requires_phases: bool,
        requires_revision_loop: bool,
        risk: String,
        complexity: String,
    ) -> Self {
        Self {
            level,
            reason,
            requires_evidence,
            requires_ordering,
            requires_phases,
            requires_revision_loop,
            risk,
            complexity,
            strategy_hint: None,
            fallback_used: false,
            confidence: 0.9,
        }
    }

    /// Create assessment with fallback defaults
    pub fn fallback(reason: &str) -> Self {
        Self {
            level: ExecutionLevel::Task, // Safe default
            reason: reason.to_string(),
            requires_evidence: false,
            requires_ordering: false,
            requires_phases: false,
            requires_revision_loop: false,
            risk: "LOW".to_string(),
            complexity: "INVESTIGATE".to_string(),
            strategy_hint: None,
            fallback_used: true,
            confidence: 0.5,
        }
    }
}

// ============================================================================
// Level Assessment Functions
// ============================================================================

/// Assess the minimum sufficient execution level for a request.
///
/// Uses principle-based heuristics, not hardcoded rules.
/// Classification priors are advisory, not deterministic.
///
/// # Arguments
/// * `client` - HTTP client for model calls
/// * `chat_url` - Base URL for chat completions
/// * `profiles` - Loaded profiles (needs complexity, evidence_need, action_need, workflow_planner)
/// * `user_message` - The original user request
/// * `route_decision` - Classification priors (advisory)
/// * `workspace_facts` - Workspace context (file tree, recent files)
/// * `workspace_brief` - Project summary
/// * `messages` - Conversation history
///
/// # Returns
/// ExecutionLadderAssessment with chosen level and justification
pub async fn assess_execution_level(
    client: &reqwest::Client,
    chat_url: &Url,
    complexity_profile: &Profile,
    evidence_need_profile: &Profile,
    action_need_profile: &Profile,
    workflow_planner_profile: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    features: &ClassificationFeatures, // Task 007: Full feature vector for better escalation
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<(ExecutionLadderAssessment, WorkflowPlannerOutput)> {
    // Build context for all units - pass shared client to prevent connection pool exhaustion
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        workspace_facts.to_string(),
        workspace_brief.to_string(),
        messages.to_vec(),
        client.clone(),
    );

    // Run all 4 assessment units in parallel where possible
    // For now, run sequentially (can optimize later)

    // 1. Get complexity assessment
    let mut bounded_complexity_profile = complexity_profile.clone();
    bounded_complexity_profile.timeout_s = bounded_complexity_profile.timeout_s.min(45);
    let complexity_unit = ComplexityAssessmentUnit::new(bounded_complexity_profile);
    let complexity_output = complexity_unit.execute_with_fallback(&context).await?;
    let complexity: ComplexityAssessment = serde_json::from_value(complexity_output.data.clone())
        .unwrap_or_else(|_| ComplexityAssessment::default());

    // 2. Get evidence needs
    let mut bounded_evidence_profile = evidence_need_profile.clone();
    bounded_evidence_profile.timeout_s = bounded_evidence_profile.timeout_s.min(45);
    let evidence_unit = EvidenceNeedsUnit::new(bounded_evidence_profile);
    let evidence_output = evidence_unit.execute_with_fallback(&context).await?;
    let needs_evidence = evidence_output
        .get_bool("needs_evidence")
        .unwrap_or(complexity.needs_evidence);
    let needs_tools = evidence_output
        .get_bool("needs_tools")
        .unwrap_or(complexity.needs_tools);

    // 3. Get action needs
    let mut bounded_action_profile = action_need_profile.clone();
    bounded_action_profile.timeout_s = bounded_action_profile.timeout_s.min(45);
    let action_unit = ActionNeedsUnit::new(bounded_action_profile);
    let action_output = action_unit.execute_with_fallback(&context).await?;
    let needs_decision = action_output
        .get_bool("needs_decision")
        .unwrap_or(complexity.needs_decision);
    let needs_plan = action_output
        .get_bool("needs_plan")
        .unwrap_or(complexity.needs_plan);

    // 4. Get workflow plan (includes objective and reason)
    let mut bounded_workflow_profile = workflow_planner_profile.clone();
    bounded_workflow_profile.timeout_s = bounded_workflow_profile.timeout_s.min(45);
    let workflow_unit = WorkflowPlannerUnit::new(bounded_workflow_profile);
    let workflow_output = workflow_unit.execute_with_fallback(&context).await?;
    let workflow_plan: WorkflowPlannerOutput =
        serde_json::from_value(workflow_output.data.clone()).unwrap_or_default();

    // Determine base level from complexity
    let base_level = complexity_to_level(&complexity.complexity);

    // Apply escalation heuristics using intel unit signals and feature vectors
    let mut level = base_level;
    let mut escalation_factors = Vec::new();

    // Escalate if intel units signal planning need
    if explicit_planning_signal(needs_plan, &complexity.complexity) {
        if level < ExecutionLevel::Plan {
            level = ExecutionLevel::Plan;
            escalation_factors.push("planning signal from intel units");
        }
    }

    // Escalate if complexity assessment indicates open-ended (strategic) work
    if strategic_signal(&complexity.complexity) {
        if level < ExecutionLevel::MasterPlan {
            level = ExecutionLevel::MasterPlan;
            escalation_factors.push("open-ended complexity assessment");
        }
    }

    // Escalate for high risk
    if complexity.risk == "HIGH" {
        if level < ExecutionLevel::Task {
            level = ExecutionLevel::Task;
            escalation_factors.push("high risk");
        }
    }

    // Escalate for high entropy (uncertain classification)
    // Task 007: Use full feature vector for better escalation decisions
    if features.entropy > 0.8 {
        if level < ExecutionLevel::Task {
            level = ExecutionLevel::Task;
            escalation_factors.push("high classification uncertainty");
        }
    }

    // Escalate for low margin (close classification)
    if route_decision.margin < 0.2 {
        if level < ExecutionLevel::Task {
            level = ExecutionLevel::Task;
            escalation_factors.push("low classification margin");
        }
    }

    // Task 007: Additional escalation based on feature mismatches
    // Check for speech act / route mismatch (suggests classification confusion)
    if let (Some((speech_act, _)), Some((route, _))) = (
        features.speech_act_probs.first(),
        features.route_probs.first(),
    ) {
        if speech_act == "ACTION_REQUEST" && route == "CHAT" {
            if level < ExecutionLevel::Task {
                level = ExecutionLevel::Task;
                escalation_factors.push("speech act/route mismatch");
            }
        }
        if speech_act == "INSTRUCTION" && route == "CHAT" {
            if level < ExecutionLevel::Task {
                level = ExecutionLevel::Task;
                escalation_factors.push("instruction classified as chat");
            }
        }
    }

    // Check for low confidence in top route choice
    if let Some((_, top_prob)) = features.route_probs.first() {
        if *top_prob < 0.5 {
            if level < ExecutionLevel::Task {
                level = ExecutionLevel::Task;
                escalation_factors.push("low confidence in route choice");
            }
        }
    }

    // A bounded evidence chain, bounded decision, or selection still needs a short workflow,
    // even when the underlying complexity is otherwise direct.
    if (needs_evidence || needs_decision || route_decision.route.eq_ignore_ascii_case("SELECT"))
        && level < ExecutionLevel::Task
    {
        level = ExecutionLevel::Task;
        escalation_factors.push("bounded evidence, decision, or selection chain");
    }

    // Bounded ordered shell tasks should stay executable.
    // MULTISTEP means more than one ordered operation, not necessarily a user-facing plan artifact.
    let bounded_ordered_shell_task = route_decision.route.eq_ignore_ascii_case("SHELL")
        && level == ExecutionLevel::Plan
        && complexity.risk == "LOW"
        && !needs_plan
        && !strategic_signal(&complexity.complexity)
        && !explicit_planning_signal(needs_plan, &complexity.complexity)
        && complexity.complexity != "OPEN_ENDED";
    if bounded_ordered_shell_task {
        level = ExecutionLevel::Task;
    }

    // Determine requires_ordering
    let requires_ordering = needs_plan || complexity.complexity == "MULTISTEP";

    // Determine requires_phases
    let requires_phases = level == ExecutionLevel::MasterPlan
        || complexity.complexity == "OPEN_ENDED";

    // Determine requires_revision_loop
    let requires_revision_loop = needs_revision_loop(
        &route_decision.route,
        &complexity.complexity,
        &complexity.risk,
    );

    // Generate reason
    let reason = generate_level_reason(level, user_message, &escalation_factors);

    // Generate strategy hint
    let strategy_hint = generate_strategy_hint(level, needs_evidence, requires_ordering);

    let confidence = calculate_confidence(
        &complexity_output,
        &evidence_output,
        &action_output,
        &workflow_output,
    );

    Ok((
        ExecutionLadderAssessment {
            level,
            reason,
            requires_evidence: needs_evidence || needs_tools,
            requires_ordering,
            requires_phases,
            requires_revision_loop,
            risk: complexity.risk.clone(),
            complexity: complexity.complexity.clone(),
            strategy_hint,
            fallback_used: complexity_output.fallback_used
                || evidence_output.fallback_used
                || action_output.fallback_used
                || workflow_output.fallback_used,
            confidence,
        },
        workflow_plan,
    ))
}

/// Map complexity classification to base execution level
fn complexity_to_level(complexity: &str) -> ExecutionLevel {
    match complexity {
        "DIRECT" => ExecutionLevel::Action,
        "INVESTIGATE" => ExecutionLevel::Task,
        "MULTISTEP" => ExecutionLevel::Plan,
        "OPEN_ENDED" => ExecutionLevel::MasterPlan,
        _ => ExecutionLevel::Task, // Safe default
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_level_display() {
        assert_eq!(format!("{}", ExecutionLevel::Action), "Action");
        assert_eq!(format!("{}", ExecutionLevel::Task), "Task");
        assert_eq!(format!("{}", ExecutionLevel::Plan), "Plan");
        assert_eq!(format!("{}", ExecutionLevel::MasterPlan), "MasterPlan");
    }

    #[test]
    fn test_execution_level_requires_planning_structure() {
        assert!(!ExecutionLevel::Action.requires_planning_structure());
        assert!(!ExecutionLevel::Task.requires_planning_structure());
        assert!(ExecutionLevel::Plan.requires_planning_structure());
        assert!(ExecutionLevel::MasterPlan.requires_planning_structure());
    }

    #[test]
    fn test_execution_level_allows_direct_execution() {
        assert!(ExecutionLevel::Action.allows_direct_execution());
        assert!(ExecutionLevel::Task.allows_direct_execution());
        assert!(!ExecutionLevel::Plan.allows_direct_execution());
        assert!(!ExecutionLevel::MasterPlan.allows_direct_execution());
    }

    #[test]
    fn test_complexity_to_level() {
        assert_eq!(complexity_to_level("DIRECT"), ExecutionLevel::Action);
        assert_eq!(complexity_to_level("INVESTIGATE"), ExecutionLevel::Task);
        assert_eq!(complexity_to_level("MULTISTEP"), ExecutionLevel::Plan);
        assert_eq!(
            complexity_to_level("OPEN_ENDED"),
            ExecutionLevel::MasterPlan
        );
        assert_eq!(complexity_to_level("UNKNOWN"), ExecutionLevel::Task);
    }

    #[test]
    fn test_assessment_fallback() {
        let assessment = ExecutionLadderAssessment::fallback("test error");
        assert_eq!(assessment.level, ExecutionLevel::Task);
        assert!(assessment.fallback_used);
        assert_eq!(assessment.confidence, 0.5);
    }
}
