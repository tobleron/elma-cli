//! Formula Scores - Cost, Value, Risk metrics for efficiency optimization

use serde::{Deserialize, Serialize};

/// Formula Scores - Metrics for efficiency optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormulaScores {
    /// Formula name
    pub formula: String,

    // Cost Metrics (1-10, lower = cheaper/faster)
    /// Overall cost score (1 = cheap, 10 = expensive)
    pub cost_score: u8,

    /// Expected number of steps
    pub expected_steps: usize,

    /// Expected token usage (rough estimate)
    pub expected_tokens: u32,

    /// Expected time in seconds (rough estimate)
    pub expected_time_sec: u32,

    // Value Metrics (1-10, higher = more valuable/thorough)
    /// Overall value score (1 = minimal, 10 = thorough)
    pub value_score: u8,

    /// Completeness score (how complete is the answer)
    pub completeness_score: u8,

    /// Accuracy score (how accurate is the answer)
    pub accuracy_score: u8,

    // Risk Metrics (1-10, lower = safer)
    /// Overall risk score (1 = safe, 10 = risky)
    pub risk_score: u8,

    /// Verification level (how much verification is done)
    pub verification_level: u8,

    /// Potential for errors (1 = low, 10 = high)
    pub error_potential: u8,

    // Computed Metrics
    /// Efficiency ratio (value / cost) - higher is better
    pub efficiency_ratio: f32,
}

impl FormulaScores {
    /// Calculate efficiency ratio from scores
    pub fn calculate_efficiency(&mut self) {
        self.efficiency_ratio = if self.cost_score > 0 {
            self.value_score as f32 / self.cost_score as f32
        } else {
            self.value_score as f32
        };
    }

    /// Get default scores for all formulas
    pub fn defaults() -> Vec<FormulaScores> {
        vec![
            Self::reply_only(),
            Self::inspect_reply(),
            Self::inspect_summarize_reply(),
            Self::inspect_decide_reply(),
            Self::inspect_edit_verify_reply(),
            Self::plan_reply(),
            Self::masterplan_reply(),
        ]
    }

    /// Find scores by formula name
    pub fn by_name(name: &str) -> Option<FormulaScores> {
        Self::defaults().into_iter().find(|f| f.formula == name)
    }

    // ========================================================================
    /// Reply Only - Cost: 1, Value: 3, Risk: 1
    /// **Efficiency: 3.00** (highest - use for simple tasks)
    fn reply_only() -> Self {
        let mut scores = FormulaScores {
            formula: "reply_only".to_string(),
            cost_score: 1,
            expected_steps: 1,
            expected_tokens: 200,
            expected_time_sec: 2,
            value_score: 3,
            completeness_score: 3,
            accuracy_score: 8,
            risk_score: 1,
            verification_level: 0,
            error_potential: 1,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }

    /// Inspect Reply - Cost: 3, Value: 6, Risk: 2
    /// **Efficiency: 2.00** (good balance for simple lookups)
    fn inspect_reply() -> Self {
        let mut scores = FormulaScores {
            formula: "inspect_reply".to_string(),
            cost_score: 3,
            expected_steps: 2,
            expected_tokens: 800,
            expected_time_sec: 8,
            value_score: 6,
            completeness_score: 6,
            accuracy_score: 8,
            risk_score: 2,
            verification_level: 1,
            error_potential: 2,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }

    /// Inspect Summarize Reply - Cost: 4, Value: 7, Risk: 2
    /// **Efficiency: 1.75** (good for summaries/overviews)
    fn inspect_summarize_reply() -> Self {
        let mut scores = FormulaScores {
            formula: "inspect_summarize_reply".to_string(),
            cost_score: 4,
            expected_steps: 3,
            expected_tokens: 1200,
            expected_time_sec: 12,
            value_score: 7,
            completeness_score: 7,
            accuracy_score: 8,
            risk_score: 2,
            verification_level: 1,
            error_potential: 2,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }

    /// Inspect Decide Reply - Cost: 5, Value: 8, Risk: 3
    /// **Efficiency: 1.60** (good for recommendations)
    fn inspect_decide_reply() -> Self {
        let mut scores = FormulaScores {
            formula: "inspect_decide_reply".to_string(),
            cost_score: 5,
            expected_steps: 3,
            expected_tokens: 1500,
            expected_time_sec: 15,
            value_score: 8,
            completeness_score: 8,
            accuracy_score: 8,
            risk_score: 3,
            verification_level: 2,
            error_potential: 3,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }

    /// Inspect Edit Verify Reply - Cost: 7, Value: 9, Risk: 5
    /// **Efficiency: 1.29** (use for code changes - thorough but expensive)
    fn inspect_edit_verify_reply() -> Self {
        let mut scores = FormulaScores {
            formula: "inspect_edit_verify_reply".to_string(),
            cost_score: 7,
            expected_steps: 4,
            expected_tokens: 2500,
            expected_time_sec: 25,
            value_score: 9,
            completeness_score: 9,
            accuracy_score: 9,
            risk_score: 5,
            verification_level: 3,
            error_potential: 5,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }

    /// Plan Reply - Cost: 5, Value: 8, Risk: 3
    /// **Efficiency: 1.60** (good for implementation plans)
    fn plan_reply() -> Self {
        let mut scores = FormulaScores {
            formula: "plan_reply".to_string(),
            cost_score: 5,
            expected_steps: 3,
            expected_tokens: 1500,
            expected_time_sec: 15,
            value_score: 8,
            completeness_score: 8,
            accuracy_score: 8,
            risk_score: 3,
            verification_level: 1,
            error_potential: 3,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }

    /// Masterplan Reply - Cost: 9, Value: 10, Risk: 4
    /// **Efficiency: 1.11** (use for complex strategic work - most thorough)
    fn masterplan_reply() -> Self {
        let mut scores = FormulaScores {
            formula: "masterplan_reply".to_string(),
            cost_score: 9,
            expected_steps: 6,
            expected_tokens: 4000,
            expected_time_sec: 40,
            value_score: 10,
            completeness_score: 10,
            accuracy_score: 9,
            risk_score: 4,
            verification_level: 3,
            error_potential: 4,
            efficiency_ratio: 0.0,
        };
        scores.calculate_efficiency();
        scores
    }
}

/// Formula Selection Result - Chosen formula with scores
#[derive(Debug, Clone)]
pub struct FormulaSelectionResult {
    /// Selected formula
    pub formula: String,

    /// Formula scores
    pub scores: FormulaScores,

    /// Why this formula was selected
    pub reason: String,

    /// Efficiency priority used (0.0 = quality, 1.0 = speed)
    pub efficiency_priority: f32,
}

/// Select optimal formula based on complexity, risk, route, and efficiency priority
pub fn select_optimal_formula(
    complexity: &str,
    risk: &str,
    route: &str,
    efficiency_priority: f32, // 0.0 = quality focused, 1.0 = speed focused
) -> FormulaSelectionResult {
    // Task: SELECT route must use evidence-gathering for choices
    if route.eq_ignore_ascii_case("SELECT") {
        return FormulaSelectionResult {
            formula: "inspect_decide_reply".to_string(),
            scores: FormulaScores {
                formula: "inspect_decide_reply".to_string(),
                cost_score: 5,
                expected_steps: 3,
                expected_tokens: 1000,
                expected_time_sec: 15,
                value_score: 8,
                completeness_score: 8,
                accuracy_score: 8,
                risk_score: 3,
                verification_level: 6,
                error_potential: 3,
                efficiency_ratio: 1.6,
            },
            reason: "SELECT route requires grounded evidence before decision".to_string(),
            efficiency_priority,
        };
    }

    let all_scores = FormulaScores::defaults();

    // Filter by complexity
    let candidates: Vec<FormulaScores> = match complexity {
        "DIRECT" => {
            if efficiency_priority > 0.7 {
                // Speed priority - use cheapest
                all_scores
                    .into_iter()
                    .filter(|f| f.cost_score <= 2)
                    .collect()
            } else {
                // Balanced
                all_scores
                    .into_iter()
                    .filter(|f| f.cost_score <= 4)
                    .collect()
            }
        }
        "INVESTIGATE" => all_scores
            .into_iter()
            .filter(|f| f.cost_score >= 3 && f.cost_score <= 5)
            .collect(),
        "MULTISTEP" => {
            if risk == "HIGH" {
                // High risk - use thorough formula
                all_scores
                    .into_iter()
                    .filter(|f| f.value_score >= 8 && f.verification_level >= 2)
                    .collect()
            } else {
                all_scores
                    .into_iter()
                    .filter(|f| f.cost_score >= 4 && f.cost_score <= 7)
                    .collect()
            }
        }
        "OPEN_ENDED" => {
            // Complex - use most thorough
            all_scores
                .into_iter()
                .filter(|f| f.value_score >= 9)
                .collect()
        }
        _ => all_scores,
    };

    // Select best by efficiency priority
    let selected = if efficiency_priority > 0.5 {
        // Speed priority - lowest cost
        candidates
            .into_iter()
            .min_by(|a, b| a.cost_score.cmp(&b.cost_score))
            .unwrap_or_else(FormulaScores::reply_only)
    } else {
        // Quality priority - highest value
        candidates
            .into_iter()
            .max_by(|a, b| a.value_score.cmp(&b.value_score))
            .unwrap_or_else(FormulaScores::inspect_reply)
    };

    FormulaSelectionResult {
        formula: selected.formula.clone(),
        reason: format!(
            "Selected {} for complexity={} with efficiency_priority={}",
            selected.formula, complexity, efficiency_priority
        ),
        scores: selected,
        efficiency_priority,
    }
}

/// Runtime Execution Metrics - Track actual performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    /// Formula used
    pub formula: String,

    /// Actual steps executed
    pub actual_steps: usize,

    /// Actual tokens used
    pub actual_tokens: u32,

    /// Actual execution time in milliseconds
    pub execution_time_ms: u64,

    /// User satisfaction (if available, 0.0 - 1.0)
    pub user_satisfaction: Option<f32>,

    /// Task type (for learning)
    pub task_type: String,

    /// Timestamp
    pub timestamp: u64,
}

/// Efficiency Report - Compare expected vs actual
#[derive(Debug, Clone)]
pub struct EfficiencyReport {
    /// Formula used
    pub formula: String,

    /// Expected cost vs actual cost ratio
    pub cost_variance: f32,

    /// Value actually achieved
    pub value_achieved: f32,

    /// Overall efficiency score
    pub efficiency_score: f32,

    /// Recommendation for improvement
    pub recommendation: String,
}

/// Calculate efficiency from expected scores and actual metrics
pub fn calculate_efficiency(
    expected: &FormulaScores,
    actual: &ExecutionMetrics,
) -> EfficiencyReport {
    let expected_cost = expected.cost_score as f32;
    let actual_cost = (actual.actual_steps as f32 + actual.execution_time_ms as f32 / 1000.0)
        / expected_cost.max(1.0);

    let value_achieved = expected.value_score as f32 * actual.user_satisfaction.unwrap_or(1.0);

    let efficiency_score = value_achieved / actual_cost.max(0.1);

    let recommendation = if actual_cost > expected_cost * 1.5 {
        format!(
            "Consider simpler formula for similar {} tasks",
            actual.task_type
        )
    } else if value_achieved < expected.value_score as f32 * 0.7 {
        format!(
            "Consider more thorough formula for similar {} tasks",
            actual.task_type
        )
    } else {
        "Formula choice was appropriate".to_string()
    };

    EfficiencyReport {
        formula: expected.formula.clone(),
        cost_variance: actual_cost - expected_cost,
        value_achieved,
        efficiency_score,
        recommendation,
    }
}
