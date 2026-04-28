//! @efficiency-role: domain-logic
//! Hierarchical Decomposition Module
//!
//! For OPEN_ENDED and HIGH complexity tasks, decompose into hierarchy:
//! Goal → Subgoal → Task → Method → Action

use crate::*;

/// Unit types in the hierarchy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitType {
    Goal,    // Level 1: Final state
    Subgoal, // Level 2: Intermediate milestone
    Task,    // Level 3: Work unit
    Method,  // Level 4: Decomposition strategy
    Action,  // Level 5: Primitive executable
}

impl UnitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitType::Goal => "goal",
            UnitType::Subgoal => "subgoal",
            UnitType::Task => "task",
            UnitType::Method => "method",
            UnitType::Action => "action",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "goal" => Some(UnitType::Goal),
            "subgoal" => Some(UnitType::Subgoal),
            "task" => Some(UnitType::Task),
            "method" => Some(UnitType::Method),
            "action" => Some(UnitType::Action),
            _ => None,
        }
    }
}

/// Get required decomposition depth based on complexity and risk
pub fn get_required_depth(complexity: &str, risk: &str) -> u8 {
    match (complexity, risk) {
        ("DIRECT", _) => 1,             // Action only
        ("INVESTIGATE", "LOW") => 2,    // Task → Action
        ("INVESTIGATE", "MEDIUM") => 3, // Subgoal → Task → Action
        ("INVESTIGATE", "HIGH") => 3,
        ("MULTISTEP", "LOW") => 3, // Subgoal → Task → Action
        ("MULTISTEP", "MEDIUM") => 3,
        ("MULTISTEP", "HIGH") => 4, // Task → Method → Action
        ("OPEN_ENDED", _) => 5,     // Full hierarchy
        (_, "HIGH") => 4,           // At least Method level
        _ => 2,
    }
}

/// Check if hierarchical decomposition is needed
pub fn needs_decomposition(complexity: &str, risk: &str) -> bool {
    get_required_depth(complexity, risk) >= 3
}

/// Masterplan structure for OPEN_ENDED tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masterplan {
    pub goal: String,
    pub phases: Vec<Phase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub name: String,
    pub objective: String,
    pub success_criteria: String,
    pub dependencies: Vec<String>,
}

/// Generate masterplan for OPEN_ENDED tasks
pub async fn generate_masterplan(
    client: &reqwest::Client,
    chat_url: &Url,
    planner_cfg: &Profile,
    objective: &str,
    ws: &str,
    ws_brief: &str,
) -> Result<Masterplan> {
    let prompt = format!(
        r#"You are Elma's strategic planner.

For this OPEN_ENDED task, generate a MASTERPLAN with strategic overview.

**Objective:** {}

**Workspace Context:**
{}
{}

Generate a masterplan with:
- Clear ultimate goal
- 3-5 major phases
- Success criteria for each phase
- Dependencies between phases

Output JSON format:
{{
  "goal": "ultimate objective",
  "phases": [
    {{"name": "Discovery", "objective": "...", "success_criteria": "...", "dependencies": []}},
    ...
  ]
}}
"#,
        objective,
        ws.trim(),
        ws_brief.trim()
    );

    let req = chat_request_system_user(
        planner_cfg,
        &planner_cfg.system_prompt,
        &prompt,
        ChatRequestOptions::default(),
    );

    let resp = chat_once(client, chat_url, &req).await?;
    let text = extract_response_text(&resp);

    // Parse JSON
    let masterplan: Masterplan = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse masterplan JSON: {}", e))?;

    Ok(masterplan)
}

/// Decompose masterplan into subgoals
pub async fn decompose_to_subgoals(
    client: &reqwest::Client,
    chat_url: &Url,
    planner_cfg: &Profile,
    masterplan: &Masterplan,
) -> Result<Vec<String>> {
    let prompt = format!(
        r#"Decompose this masterplan into subgoals (intermediate milestones).

**Masterplan Goal:** {}

**Phases:**
{}

Generate 3-5 subgoals that represent key milestones.
Each subgoal should be:
- Achievable independently
- Measurable (clear completion criteria)
- Ordered logically

Output as JSON array of subgoal descriptions."#,
        masterplan.goal,
        masterplan
            .phases
            .iter()
            .map(|p| format!("- {}: {}", p.name, p.objective))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let req = chat_request_system_user(
        planner_cfg,
        &planner_cfg.system_prompt,
        &prompt,
        ChatRequestOptions {
            temperature: Some(0.3),
            max_tokens: Some(1024),
            ..ChatRequestOptions::default()
        },
    );

    let resp = chat_once(client, chat_url, &req).await?;
    let text = extract_response_text(&resp);

    // Parse as array of strings
    let subgoals: Vec<String> = serde_json::from_str(&text)
        .map_err(|e| anyhow::anyhow!("Failed to parse subgoals JSON: {}", e))?;

    Ok(subgoals)
}

/// Aggregate results from child units to parent
pub fn aggregate_results(child_results: &[StepResult]) -> String {
    let mut summary = String::new();

    let successful = child_results.iter().filter(|r| r.ok).count();
    let total = child_results.len();

    summary.push_str(&format!(
        "Completed {}/{} child units.\n\n",
        successful, total
    ));

    for (i, result) in child_results.iter().enumerate() {
        summary.push_str(&format!(
            "Step {}: {}\n  Status: {}\n  Summary: {}\n\n",
            i + 1,
            result.id,
            if result.ok { "OK" } else { "FAILED" },
            result.summary
        ));
    }

    summary
}
