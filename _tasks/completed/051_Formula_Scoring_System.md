# Task 051: Formula Scoring System for Efficiency Optimization

## Priority
**P1 - HIGH** (Enables dynamic efficiency optimization)

## Problem
Elma currently uses formulas without considering:
- **Cost** (steps, time, tokens)
- **Value** (completeness, accuracy)
- **Risk** (potential for errors)
- **Efficiency** (value / cost ratio)

**Result:** Over-engineering simple tasks, under-engineering complex tasks.

## Goal
Add scoring system to formulas so orchestrator can choose the **most efficient formula** for each task based on complexity and efficiency priority.

## Implementation

### 1. Formula Score Definitions

```rust
pub struct FormulaScores {
    // Cost Metrics (1-10, lower = cheaper)
    pub cost_score: u8,           // Steps, time, tokens
    pub expected_steps: usize,    // Number of steps
    
    // Value Metrics (1-10, higher = more valuable)
    pub value_score: u8,          // Completeness, accuracy
    pub completeness_score: u8,   // How thorough is the answer?
    
    // Risk Metrics (1-10, lower = safer)
    pub risk_score: u8,           // Potential for errors
    pub verification_level: u8,   // How much verification?
    
    // Computed
    pub efficiency_ratio: f32,    // value / cost
}
```

### 2. Default Scores Per Formula

| Formula | Steps | Cost | Value | Risk | Efficiency | Best For |
|---------|-------|------|-------|------|------------|----------|
| `reply_only` | 1 | 1 | 3 | 1 | 3.00 | Greetings, simple Q&A |
| `inspect_reply` | 2 | 3 | 6 | 2 | 2.00 | File lookup, quick facts |
| `inspect_summarize_reply` | 3 | 4 | 7 | 2 | 1.75 | Project overview, summaries |
| `inspect_decide_reply` | 3 | 5 | 8 | 3 | 1.60 | Choices, recommendations |
| `inspect_edit_verify_reply` | 4 | 7 | 9 | 5 | 1.29 | Code changes, fixes |
| `plan_reply` | 2-5 | 5 | 8 | 3 | 1.60 | Implementation plans |
| `masterplan_reply` | 5-10 | 9 | 10 | 4 | 1.11 | Complex multi-phase work |

### 3. Orchestrator Formula Selection Logic

```rust
fn select_optimal_formula(
    complexity: &ComplexityAssessment,
    user_efficiency_priority: f32,  // 0.0 = quality, 1.0 = speed
) -> FormulaPattern {
    
    match complexity.complexity.as_str() {
        "DIRECT" => {
            if user_efficiency_priority > 0.7 {
                FormulaPattern::ReplyOnly  // Cost: 1, fast!
            } else {
                FormulaPattern::InspectReply  // Cost: 3, more thorough
            }
        }
        "INVESTIGATE" => {
            FormulaPattern::InspectSummarizeReply  // Balanced
        }
        "MULTISTEP" => {
            if complexity.risk == "HIGH" {
                FormulaPattern::InspectEditVerifyReply  // Thorough + verified
            } else {
                FormulaPattern::PlanReply  // Balanced
            }
        }
        "OPEN_ENDED" => {
            FormulaPattern::MasterplanReply  // Most thorough
        }
        _ => FormulaPattern::InspectReply  // Default
    }
}
```

### 4. Runtime Efficiency Calculation

```rust
pub struct ExecutionMetrics {
    pub actual_steps: usize,
    pub actual_tokens: u32,
    pub execution_time_ms: u64,
    pub user_satisfaction: Option<f32>,  // From feedback
}

pub fn calculate_efficiency(
    formula: &FormulaPattern,
    actual: &ExecutionMetrics,
) -> EfficiencyReport {
    let expected_cost = formula.scores.cost_score as f32;
    let actual_cost = (actual.actual_steps as f32 
                      + actual.execution_time_ms as f32 / 1000.0) 
                      / expected_cost;
    
    let value_achieved = formula.scores.value_score as f32 
                        * actual.user_satisfaction.unwrap_or(1.0);
    
    EfficiencyReport {
        cost_variance: actual_cost - expected_cost,
        value_achieved,
        efficiency_score: value_achieved / actual_cost,
        recommendation: if actual_cost > expected_cost * 1.5 {
            "Consider simpler formula for similar tasks"
        } else if value_achieved < formula.scores.value_score as f32 * 0.7 {
            "Consider more thorough formula for similar tasks"
        } else {
            "Formula choice was appropriate"
        }
    }
}
```

### 5. Learning From History

```rust
pub struct FormulaHistory {
    pub task_type: String,
    pub formula_used: String,
    pub actual_efficiency: f32,
    pub user_satisfaction: f32,
    pub timestamp: u64,
}

// Adjust formula selection based on history
pub fn adjust_formula_recommendations(history: &[FormulaHistory]) {
    // If "inspect_reply" for "code_fix" tasks 
    // consistently has low satisfaction → recommend more thorough formula
}
```

## Files to Create

- `src/formulas/scores.rs` - Formula score definitions
- `src/formulas/efficiency.rs` - Runtime efficiency calculation
- `src/formulas/history.rs` - Execution history tracking
- `src/formulas/selector.rs` - Formula selection logic

## Files to Modify

- `src/formulas/mod.rs` - Add score module
- `src/orchestration.rs` - Use formula selector
- `src/types_core.rs` - Add FormulaScores struct

## Acceptance Criteria

- [ ] All formulas have cost/value/risk scores defined
- [ ] Orchestrator selects formula based on complexity + efficiency priority
- [ ] Runtime efficiency calculated after each execution
- [ ] Formula history tracked (last 100 executions)
- [ ] Simple tasks use low-cost formulas (reply_only for greetings)
- [ ] Complex tasks use high-value formulas (masterplan_reply for strategic work)
- [ ] Efficiency recommendations generated when cost variance > 50%

## Expected Impact

| Metric | Before | After | Improvement |
|--------|--------|-------|-------------|
| Simple task time | ~5 steps | ~1 step | -80% |
| Complex task quality | ~70% | ~90% | +20% |
| Token usage (avg) | ~2000 | ~1200 | -40% |
| User satisfaction | ~75% | ~85% | +10% |

## Dependencies

- Task 001: Abstract Formula Patterns (foundation - formulas as patterns)
- Task 015: Tool Discovery (provides tool registry for orchestrator)

## Verification

- Test simple tasks use `reply_only` (greetings, simple Q&A)
- Test complex tasks use `masterplan_reply` (strategic planning)
- Test efficiency calculation matches expected scores
- Test history tracking and recommendations

## Related Tasks

- Task 001: Revise And Perfect Existing Formulas (foundation)
- Task 015: Autonomous Tool Discovery (tool registry)
- Task 006: Revise Core Formulas Plan Family (formula specifics)
