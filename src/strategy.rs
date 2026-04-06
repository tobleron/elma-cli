//! @efficiency-role: domain-logic
//!
//! Strategy Module - Multi-Strategy Planning with Fallback Chains
//!
//! Implements Elma's ability to improvise solutions when rigid approaches fail.
//! Philosophy: "flexibility to improvise solutions that rigid rule-based systems would miss"

use crate::*;

// ============================================================================
// Execution Strategy
// ============================================================================

/// Execution strategy for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStrategy {
    /// Execute immediately with minimal overhead
    /// Best for: Simple, low-risk tasks with clear path
    Direct,

    /// Gather evidence first, then act
    /// Best for: Tasks requiring workspace context before execution
    InspectFirst,

    /// Create detailed plan, then execute
    /// Best for: Complex tasks with multiple dependencies
    PlanThenExecute,

    /// Dry-run/preview first, then execute for real
    /// Best for: Risky operations (deletions, modifications)
    SafeMode,

    /// Small verifiable steps with checks between
    /// Best for: Tasks where intermediate verification matters
    Incremental,
}

impl ExecutionStrategy {
    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            ExecutionStrategy::Direct => "Execute immediately with minimal overhead",
            ExecutionStrategy::InspectFirst => "Gather evidence first, then act",
            ExecutionStrategy::PlanThenExecute => "Create detailed plan, then execute",
            ExecutionStrategy::SafeMode => "Dry-run/preview first, then execute",
            ExecutionStrategy::Incremental => "Small verifiable steps with checks",
        }
    }

    /// Get strategy hint for prompts
    pub fn hint(&self) -> &'static str {
        match self {
            ExecutionStrategy::Direct => "Use the most straightforward approach.",
            ExecutionStrategy::InspectFirst => "Gather evidence first using 1-2 SIMPLE shell commands (ls, rg, cat). Avoid complex pipes or jq logic in early steps. Propose action ONLY after seeing results.",
            ExecutionStrategy::PlanThenExecute => "Create a detailed plan before acting.",
            ExecutionStrategy::SafeMode => "Start with dry-run/preview, then execute.",
            ExecutionStrategy::Incremental => "Break into small verifiable steps.",
        }
    }
}

// ============================================================================
// Strategy Chain
// ============================================================================

/// Strategy chain with primary and fallback strategies
pub struct StrategyChain {
    pub primary: ExecutionStrategy,
    pub fallbacks: Vec<ExecutionStrategy>,
    pub current_attempt: usize,
    pub failures: Vec<String>,
}

impl StrategyChain {
    /// Create new strategy chain
    pub fn new(primary: ExecutionStrategy, fallbacks: Vec<ExecutionStrategy>) -> Self {
        Self {
            primary,
            fallbacks,
            current_attempt: 0,
            failures: Vec::new(),
        }
    }

    /// Get current strategy
    pub fn current_strategy(&self) -> ExecutionStrategy {
        if self.current_attempt == 0 {
            self.primary
        } else if self.current_attempt <= self.fallbacks.len() {
            self.fallbacks[self.current_attempt - 1]
        } else {
            self.primary // Exhausted, return primary as fallback
        }
    }

    /// Get next strategy (returns None if exhausted)
    pub fn next_strategy(&mut self) -> Option<ExecutionStrategy> {
        if self.current_attempt == 0 {
            self.current_attempt = 1;
            Some(self.primary)
        } else if self.current_attempt <= self.fallbacks.len() {
            let strategy = self.fallbacks[self.current_attempt - 1];
            self.current_attempt += 1;
            Some(strategy)
        } else {
            None // Exhausted
        }
    }

    /// Record a failure
    pub fn record_failure(&mut self, error: &str) {
        self.failures.push(error.to_string());
    }

    /// Record success
    pub fn record_success(&mut self) {
        // Could log success for learning
    }

    /// Check if strategies are exhausted
    pub fn is_exhausted(&self) -> bool {
        self.current_attempt >= self.fallbacks.len() + 1
    }

    /// Get attempt number
    pub fn attempt(&self) -> usize {
        self.current_attempt
    }

    /// Get total strategies available
    pub fn total_strategies(&self) -> usize {
        1 + self.fallbacks.len()
    }
}

// ============================================================================
// Strategy Selection
// ============================================================================

/// Select strategy chain based on task characteristics
pub fn select_strategy_chain(
    user_message: &str,
    complexity: &ComplexityAssessment,
    route_decision: &RouteDecision,
) -> StrategyChain {
    // Determine primary strategy based on complexity and risk
    let primary = select_primary_strategy(complexity, route_decision);

    // Determine fallbacks based on task type
    let fallbacks = select_fallback_strategies(primary, user_message, complexity);

    StrategyChain::new(primary, fallbacks)
}

/// Select primary strategy based on complexity and risk
fn select_primary_strategy(
    complexity: &ComplexityAssessment,
    route_decision: &RouteDecision,
) -> ExecutionStrategy {
    // High risk → SafeMode
    if complexity.risk == "HIGH" {
        return ExecutionStrategy::SafeMode;
    }

    // Selection tasks → InspectFirst
    if route_decision.route.eq_ignore_ascii_case("SELECT") {
        return ExecutionStrategy::InspectFirst;
    }

    // Complex tasks → PlanThenExecute or InspectFirst
    match complexity.complexity.as_str() {
        "OPEN_ENDED" => ExecutionStrategy::PlanThenExecute,
        "MULTISTEP" => {
            if complexity.needs_evidence {
                ExecutionStrategy::InspectFirst
            } else {
                ExecutionStrategy::PlanThenExecute
            }
        }
        "INVESTIGATE" => {
            if complexity.needs_evidence {
                ExecutionStrategy::InspectFirst
            } else {
                ExecutionStrategy::Direct
            }
        }
        "DIRECT" | _ => {
            // Simple tasks: Direct, but check entropy
            if route_decision.entropy > 0.8 {
                // High uncertainty → InspectFirst
                ExecutionStrategy::InspectFirst
            } else {
                ExecutionStrategy::Direct
            }
        }
    }
}

/// Select fallback strategies based on primary and task characteristics
fn select_fallback_strategies(
    primary: ExecutionStrategy,
    _user_message: &str,
    complexity: &ComplexityAssessment,
) -> Vec<ExecutionStrategy> {
    let mut fallbacks = Vec::new();

    // Add fallbacks based on primary strategy
    match primary {
        ExecutionStrategy::Direct => {
            // Direct failed → try inspecting first
            fallbacks.push(ExecutionStrategy::InspectFirst);
            // Still failing → plan it out
            if complexity.complexity != "DIRECT" {
                fallbacks.push(ExecutionStrategy::PlanThenExecute);
            }
        }
        ExecutionStrategy::InspectFirst => {
            // Inspection failed → try direct with gathered info
            fallbacks.push(ExecutionStrategy::Direct);
            // Still failing → plan it
            fallbacks.push(ExecutionStrategy::PlanThenExecute);
        }
        ExecutionStrategy::PlanThenExecute => {
            // Planning failed → try incremental
            fallbacks.push(ExecutionStrategy::Incremental);
            // Still failing → try safe mode
            fallbacks.push(ExecutionStrategy::SafeMode);
        }
        ExecutionStrategy::SafeMode => {
            // Safe mode failed → try direct
            fallbacks.push(ExecutionStrategy::Direct);
            // Still failing → inspect first
            fallbacks.push(ExecutionStrategy::InspectFirst);
        }
        ExecutionStrategy::Incremental => {
            // Incremental failed → try direct
            fallbacks.push(ExecutionStrategy::Direct);
            // Still failing → plan it
            fallbacks.push(ExecutionStrategy::PlanThenExecute);
        }
    }

    fallbacks
}

// ============================================================================
// Strategy Log
// ============================================================================

/// Log entry for strategy effectiveness tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyLog {
    pub timestamp: u64,
    pub task_type: String,
    pub strategy: String,
    pub attempt_number: usize,
    pub success: bool,
    pub error_if_failed: Option<String>,
    pub execution_time_ms: u64,
}

impl StrategyLog {
    /// Create new strategy log entry
    pub fn new(
        task_type: &str,
        strategy: ExecutionStrategy,
        attempt_number: usize,
        success: bool,
        error_if_failed: Option<&str>,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            timestamp: crate::now_unix_s().unwrap_or(0),
            task_type: task_type.to_string(),
            strategy: format!("{:?}", strategy),
            attempt_number,
            success,
            error_if_failed: error_if_failed.map(|s| s.to_string()),
            execution_time_ms,
        }
    }

    /// Save log entry to session file
    pub fn save_to_session(&self, session_root: &Path) -> Result<()> {
        let log_dir = session_root.join("strategy_logs");
        std::fs::create_dir_all(&log_dir)?;

        let log_file = log_dir.join("strategy_log.jsonl");
        let json = serde_json::to_string(self)?;

        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_file)?;
        writeln!(file, "{}", json)?;

        Ok(())
    }
}

// ============================================================================
// Strategy Execution Helpers
// ============================================================================

/// Execute a task with a specific strategy
/// Returns a program tailored to the strategy
pub async fn execute_with_strategy(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    strategy: ExecutionStrategy,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<Program> {
    // Build strategy-specific prompt
    let strategy_hint = strategy.hint();

    // Adjust temperature based on strategy
    let temp = match strategy {
        ExecutionStrategy::Direct => orchestrator_cfg.temperature,
        ExecutionStrategy::InspectFirst => orchestrator_cfg.temperature.min(0.3),
        ExecutionStrategy::PlanThenExecute => orchestrator_cfg.temperature.max(0.4),
        ExecutionStrategy::SafeMode => orchestrator_cfg.temperature.min(0.2),
        ExecutionStrategy::Incremental => orchestrator_cfg.temperature,
    };

    // Build prompt with strategy context
    let prompt = format!(
        r#"{}

STRATEGY: {:?}
Guidance: {}

User request: {}
Objective: {}
Formula: {}

Output ONLY valid Program JSON that follows this strategy and satisfies the objective. 
A Program JSON must contain a "steps" array of objects with "id", "type", "cmd" (for shell/search), "instructions" (for read/reply), and "purpose"."#,
        orchestrator_cfg.system_prompt,
        strategy,
        strategy_hint,
        user_message,
        scope.objective,
        formula.primary
    );

    // Generate program with strategy-aware temperature
    let req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage::simple("system", &orchestrator_cfg.system_prompt.clone()),
            ChatMessage::simple("user", &prompt),
        ],
        temperature: temp,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
        grammar: Some(crate::json_program_grammar()),
        tools: None,
    };

    let (program, _) = crate::chat_json_with_repair_text(client, chat_url, &req).await?;
    Ok(program)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strategy_chain_creation() {
        let chain = StrategyChain::new(
            ExecutionStrategy::Direct,
            vec![
                ExecutionStrategy::InspectFirst,
                ExecutionStrategy::PlanThenExecute,
            ],
        );

        assert_eq!(chain.primary, ExecutionStrategy::Direct);
        assert_eq!(chain.fallbacks.len(), 2);
        assert_eq!(chain.total_strategies(), 3);
    }

    #[test]
    fn test_strategy_chain_iteration() {
        let mut chain = StrategyChain::new(
            ExecutionStrategy::Direct,
            vec![ExecutionStrategy::InspectFirst],
        );

        // First call returns primary
        let strategy = chain.next_strategy();
        assert_eq!(strategy, Some(ExecutionStrategy::Direct));
        assert_eq!(chain.attempt(), 1);

        // Second call returns first fallback
        let strategy = chain.next_strategy();
        assert_eq!(strategy, Some(ExecutionStrategy::InspectFirst));
        assert_eq!(chain.attempt(), 2);

        // Third call returns None (exhausted)
        let strategy = chain.next_strategy();
        assert_eq!(strategy, None);
        assert!(chain.is_exhausted());
    }

    #[test]
    fn test_strategy_chain_failure_recording() {
        let mut chain = StrategyChain::new(ExecutionStrategy::Direct, vec![]);

        chain.record_failure("command not found");
        chain.record_failure("permission denied");

        assert_eq!(chain.failures.len(), 2);
    }

    #[test]
    fn test_select_primary_strategy_high_risk() {
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "HIGH".to_string(),
            ..ComplexityAssessment::default()
        };
        let route_decision = create_test_route_decision();

        let primary = select_primary_strategy(&complexity, &route_decision);
        assert_eq!(primary, ExecutionStrategy::SafeMode);
    }

    #[test]
    fn test_select_primary_strategy_open_ended() {
        let complexity = ComplexityAssessment {
            complexity: "OPEN_ENDED".to_string(),
            risk: "MEDIUM".to_string(),
            ..ComplexityAssessment::default()
        };
        let route_decision = create_test_route_decision();

        let primary = select_primary_strategy(&complexity, &route_decision);
        assert_eq!(primary, ExecutionStrategy::PlanThenExecute);
    }

    #[test]
    fn test_select_primary_strategy_high_entropy() {
        let complexity = ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        };
        let mut route_decision = create_test_route_decision();
        route_decision.entropy = 0.9; // High uncertainty

        let primary = select_primary_strategy(&complexity, &route_decision);
        assert_eq!(primary, ExecutionStrategy::InspectFirst);
    }

    fn create_test_route_decision() -> RouteDecision {
        RouteDecision {
            route: "SHELL".to_string(),
            source: "test".to_string(),
            distribution: vec![("SHELL".to_string(), 0.8)],
            margin: 0.6,
            entropy: 0.3,
            speech_act: crate::ProbabilityDecision {
                choice: "INSTRUCTION".to_string(),
                source: "test".to_string(),
                distribution: vec![("INSTRUCTION".to_string(), 0.8)],
                margin: 0.6,
                entropy: 0.3,
            },
            workflow: crate::ProbabilityDecision {
                choice: "EXECUTE".to_string(),
                source: "test".to_string(),
                distribution: vec![("EXECUTE".to_string(), 0.8)],
                margin: 0.6,
                entropy: 0.3,
            },
            mode: crate::ProbabilityDecision {
                choice: "INSPECT".to_string(),
                source: "test".to_string(),
                distribution: vec![("INSPECT".to_string(), 0.8)],
                margin: 0.6,
                entropy: 0.3,
            },
        }
    }
}
