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

use crate::*;
use crate::intel_trait::*;
use crate::intel_units::*;

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
            level: ExecutionLevel::Task,  // Safe default
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
    _evidence_need_profile: &Profile,  // Reserved for future use
    _action_need_profile: &Profile,  // Reserved for future use
    workflow_planner_profile: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    features: &ClassificationFeatures,  // Task 007: Full feature vector for better escalation
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ExecutionLadderAssessment> {
    // Build context for all units
    let context = IntelContext::new(
        user_message.to_string(),
        route_decision.clone(),
        workspace_facts.to_string(),
        workspace_brief.to_string(),
        messages.to_vec(),
    );
    
    // Run all 4 assessment units in parallel where possible
    // For now, run sequentially (can optimize later)
    
    // 1. Get complexity assessment
    let complexity_unit = ComplexityAssessmentUnit::new(complexity_profile.clone());
    let complexity_output = complexity_unit.execute_with_fallback(&context).await?;
    let complexity: ComplexityAssessment = serde_json::from_value(complexity_output.data.clone())
        .unwrap_or_else(|_| ComplexityAssessment::default());

    // 2. Get evidence needs (DISABLED - unit not configured)
    // let evidence_unit = EvidenceNeedsUnit::new(evidence_need_profile.clone());
    // let evidence_output = evidence_unit.execute_with_fallback(&context).await?;
    // let needs_evidence = evidence_output.get_bool("needs_evidence").unwrap_or(false);
    // let needs_tools = evidence_output.get_bool("needs_tools").unwrap_or(false);
    let needs_evidence = complexity.needs_evidence;
    let needs_tools = complexity.needs_tools;

    // 3. Get action needs (DISABLED - unit not configured)
    // let action_unit = ActionNeedsUnit::new(action_need_profile.clone());
    // let action_output = action_unit.execute_with_fallback(&context).await?;
    // let needs_decision = action_output.get_bool("needs_decision").unwrap_or(false);
    // let needs_plan = action_output.get_bool("needs_plan").unwrap_or(false);
    let needs_decision = complexity.needs_decision;
    let needs_plan = complexity.needs_plan;
    
    // 4. Get workflow plan (includes objective and reason)
    let workflow_unit = WorkflowPlannerUnit::new(workflow_planner_profile.clone());
    let workflow_output = workflow_unit.execute_with_fallback(&context).await?;
    
    // Determine base level from complexity
    let base_level = complexity_to_level(&complexity.complexity);
    
    // Apply escalation heuristics
    let mut level = base_level;
    let mut escalation_factors = Vec::new();
    
    // Escalate for explicit planning request
    if requests_planning(user_message) {
        if level < ExecutionLevel::Plan {
            level = ExecutionLevel::Plan;
            escalation_factors.push("explicit planning request");
        }
    }
    
    // Escalate for strategic request
    if requests_strategy(user_message) {
        if level < ExecutionLevel::MasterPlan {
            level = ExecutionLevel::MasterPlan;
            escalation_factors.push("strategic decomposition request");
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
        features.route_probs.first()
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
    
    // Determine requires_ordering
    let requires_ordering = needs_plan || has_dependencies(user_message, workspace_brief);
    
    // Determine requires_phases
    let requires_phases = level == ExecutionLevel::MasterPlan || 
        requests_phases(user_message) ||
        complexity.complexity == "OPEN_ENDED";
    
    // Determine requires_revision_loop
    let requires_revision_loop = needs_revision_loop(user_message, &complexity);
    
    // Generate reason
    let reason = generate_level_reason(level, user_message, &escalation_factors);
    
    // Generate strategy hint
    let strategy_hint = generate_strategy_hint(level, needs_evidence, requires_ordering);

    // Calculate confidence (DISABLED units use complexity as fallback)
    let confidence = calculate_confidence(
        &complexity_output,
        &complexity_output,  // Use complexity as fallback for evidence
        &complexity_output,  // Use complexity as fallback for action
        &workflow_output,
    );

    Ok(ExecutionLadderAssessment {
        level,
        reason,
        requires_evidence: needs_evidence || needs_tools,
        requires_ordering,
        requires_phases,
        requires_revision_loop,
        risk: complexity.risk.clone(),
        complexity: complexity.complexity.clone(),
        strategy_hint,
        fallback_used: complexity_output.fallback_used ||
            workflow_output.fallback_used,
        confidence,
    })
}

/// Map complexity classification to base execution level
fn complexity_to_level(complexity: &str) -> ExecutionLevel {
    match complexity {
        "DIRECT" => ExecutionLevel::Action,
        "INVESTIGATE" => ExecutionLevel::Task,
        "MULTISTEP" => ExecutionLevel::Plan,
        "OPEN_ENDED" => ExecutionLevel::MasterPlan,
        _ => ExecutionLevel::Task,  // Safe default
    }
}

/// Check if request explicitly asks for planning
fn requests_planning(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();
    
    // Principle: Look for planning SEMANTICS, not just keywords
    // Sequential language, decomposition language, planning language
    
    let planning_indicators = [
        "step-by-step", "step by step", "give me a plan", "create a plan",
        "break down", "breakdown", "detailed plan", "implementation plan",
        "how would you approach", "what steps", "ordered steps",
    ];
    
    planning_indicators.iter().any(|indicator| lower.contains(indicator))
}

/// Check if request implies strategic decomposition
fn requests_strategy(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();
    
    // Principle: Strategic = multi-phase, multi-session, or architectural
    
    let strategy_indicators = [
        "migration strategy", "architecture redesign", "phased approach",
        "long-term plan", "overall strategy", "master plan", "masterplan",
        "strategic overview", "roadmap", "multi-phase", "multi-session",
    ];
    
    strategy_indicators.iter().any(|indicator| lower.contains(indicator))
}

/// Check if request asks for phased decomposition
fn requests_phases(user_message: &str) -> bool {
    let lower = user_message.to_lowercase();
    
    let phase_indicators = [
        "phases", "phase", "milestone", "stages", "stage",
        "rollout", "deployment plan", "staged approach",
    ];
    
    phase_indicators.iter().any(|indicator| lower.contains(indicator))
}

/// Check if request has dependencies requiring ordering
fn has_dependencies(user_message: &str, _workspace_brief: &str) -> bool {
    let lower = user_message.to_lowercase();
    
    // Look for dependency language
    let dependency_indicators = [
        "first x then y", "before doing", "after completing",
        "dependencies", "prerequisite", "must complete",
        "implement feature", "refactor", "clean up",
    ];
    
    dependency_indicators.iter().any(|indicator| lower.contains(indicator))
}

/// Check if request needs revision loop
fn needs_revision_loop(user_message: &str, complexity: &ComplexityAssessment) -> bool {
    let lower = user_message.to_lowercase();
    
    // Revision indicators
    let revision_indicators = [
        "fix", "debug", "troubleshoot", "refactor", "iterate",
        "keep trying", "refine", "adjust", "verify after",
    ];
    
    let has_revision_language = revision_indicators.iter()
        .any(|indicator| lower.contains(indicator));
    
    // Edit operations often need revision
    let is_edit_heavy = lower.contains("edit") || lower.contains("modify") || 
        lower.contains("update") || lower.contains("change");
    
    has_revision_language || (is_edit_heavy && complexity.complexity != "DIRECT")
}

/// Generate human-readable reason for level choice
fn generate_level_reason(
    level: ExecutionLevel,
    user_message: &str,
    escalation_factors: &[&str],
) -> String {
    let truncated = truncate_message(user_message);
    
    let base_reason = match level {
        ExecutionLevel::Action => format!("Direct execution: '{}'", truncated),
        ExecutionLevel::Task => format!("Bounded outcome requiring evidence chain: '{}'", truncated),
        ExecutionLevel::Plan => format!("Tactical breakdown required: '{}'", truncated),
        ExecutionLevel::MasterPlan => format!("Strategic decomposition required: '{}'", truncated),
    };
    
    if escalation_factors.is_empty() {
        base_reason
    } else {
        format!("{} (escalated: {})", base_reason, escalation_factors.join(", "))
    }
}

/// Generate optional strategy hint for formula selection/planning
fn generate_strategy_hint(
    level: ExecutionLevel,
    requires_evidence: bool,
    requires_ordering: bool,
) -> Option<String> {
    match (level, requires_evidence, requires_ordering) {
        (ExecutionLevel::Action, false, false) => {
            None  // No hint needed for simple action
        }
        (ExecutionLevel::Task, true, false) => {
            Some("gather evidence before execution".to_string())
        }
        (ExecutionLevel::Task, true, true) => {
            Some("gather evidence, then execute in order".to_string())
        }
        (ExecutionLevel::Plan, _, _) => {
            Some("explicit planning structure required".to_string())
        }
        (ExecutionLevel::MasterPlan, _, _) => {
            Some("phased strategic decomposition".to_string())
        }
        _ => None,
    }
}

/// Calculate overall confidence from unit outputs
fn calculate_confidence(
    complexity: &IntelOutput,
    evidence: &IntelOutput,
    action: &IntelOutput,
    workflow: &IntelOutput,
) -> f64 {
    // Average confidence from all units
    let confidences = [
        complexity.confidence,
        evidence.confidence,
        action.confidence,
        workflow.confidence,
    ];
    
    let avg = confidences.iter().sum::<f64>() / confidences.len() as f64;
    
    // Reduce confidence if any unit used fallback
    let fallback_penalty = [
        complexity.fallback_used,
        evidence.fallback_used,
        action.fallback_used,
        workflow.fallback_used,
    ].iter().filter(|&&x| x).count() as f64 * 0.1;
    
    (avg - fallback_penalty).max(0.3).min(1.0)
}

/// Truncate message for display in reason
fn truncate_message(msg: &str) -> String {
    let truncated = msg.split_whitespace().take(5).collect::<Vec<_>>().join(" ");
    if msg.len() > truncated.len() {
        format!("{}...", truncated)
    } else {
        truncated
    }
}

// ============================================================================
// Compatibility Functions
// ============================================================================

/// Check if hierarchical decomposition is needed (compatibility wrapper)
pub fn assessment_needs_decomposition(assessment: &ExecutionLadderAssessment) -> bool {
    matches!(assessment.level, ExecutionLevel::Plan | ExecutionLevel::MasterPlan)
}

/// Convert assessment to legacy depth (compatibility wrapper)
pub fn assessment_to_depth(assessment: &ExecutionLadderAssessment) -> u8 {
    match assessment.level {
        ExecutionLevel::Action => 1,
        ExecutionLevel::Task => 2,
        ExecutionLevel::Plan => 3,
        ExecutionLevel::MasterPlan => 4,
    }
}

/// Convert legacy depth to level (compatibility wrapper)
pub fn depth_to_level(depth: u8) -> ExecutionLevel {
    match depth {
        0 | 1 => ExecutionLevel::Action,
        2 => ExecutionLevel::Task,
        3 => ExecutionLevel::Plan,
        _ => ExecutionLevel::MasterPlan,
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
        assert_eq!(complexity_to_level("OPEN_ENDED"), ExecutionLevel::MasterPlan);
        assert_eq!(complexity_to_level("UNKNOWN"), ExecutionLevel::Task);
    }

    #[test]
    fn test_requests_planning() {
        assert!(requests_planning("give me a step-by-step plan"));
        assert!(requests_planning("create a plan to refactor"));
        assert!(requests_planning("break down this task"));
        assert!(!requests_planning("run cargo test"));
        assert!(!requests_planning("read this file"));
    }

    #[test]
    fn test_requests_strategy() {
        assert!(requests_strategy("design a migration strategy"));
        assert!(requests_strategy("create a masterplan"));
        assert!(requests_strategy("phased approach for redesign"));
        assert!(!requests_strategy("run tests"));
        assert!(!requests_strategy("read file"));
    }

    #[test]
    fn test_assessment_fallback() {
        let assessment = ExecutionLadderAssessment::fallback("test error");
        assert_eq!(assessment.level, ExecutionLevel::Task);
        assert!(assessment.fallback_used);
        assert_eq!(assessment.confidence, 0.5);
    }

    #[test]
    fn test_generate_strategy_hint() {
        assert_eq!(
            generate_strategy_hint(ExecutionLevel::Action, false, false),
            None
        );
        assert_eq!(
            generate_strategy_hint(ExecutionLevel::Task, true, false),
            Some("gather evidence before execution".to_string())
        );
        assert_eq!(
            generate_strategy_hint(ExecutionLevel::Plan, false, false),
            Some("explicit planning structure required".to_string())
        );
    }

    #[test]
    fn test_depth_conversion_roundtrip() {
        for depth in 1..=4 {
            let level = depth_to_level(depth);
            let converted_back = assessment_to_depth(&ExecutionLadderAssessment {
                level,
                reason: "test".to_string(),
                requires_evidence: false,
                requires_ordering: false,
                requires_phases: false,
                requires_revision_loop: false,
                risk: "LOW".to_string(),
                complexity: "DIRECT".to_string(),
                strategy_hint: None,
                fallback_used: false,
                confidence: 0.9,
            });
            assert_eq!(converted_back, depth);
        }
    }
}
