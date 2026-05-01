//! @efficiency-role: domain-logic
//! Hierarchical Decomposition Module
//!
//! For OPEN_ENDED and HIGH complexity tasks, decompose into hierarchy:
//! Goal → Subgoal → Task → Method → Action

use crate::*;

fn ctx_named(name: &'static str) -> crate::dsl::ParseContext {
    crate::dsl::ParseContext {
        dsl_variant: name,
        line: None,
    }
}

fn parse_masterplan_dsl(input: &str) -> crate::dsl::DslResult<Masterplan> {
    let input = input.trim();
    if input.is_empty() {
        return Err(crate::dsl::DslError::empty(ctx_named("masterplan")));
    }
    let parser = crate::dsl::DslBlockParser::new(ctx_named("masterplan"));
    let block = parser.parse_block(input, "END")?;
    if block.command != "MASTERPLAN" {
        return Err(crate::dsl::DslError::unknown_command(
            ctx_named("masterplan"),
            &block.command,
        ));
    }
    let goal = crate::dsl::require_field(&block.fields, "goal", &ctx_named("masterplan"))?;

    let mut phases = Vec::new();
    for line in &block.lines {
        match line {
            crate::dsl::DslLine::Text { text } => {
                if !text.trim().is_empty() {
                    return Err(crate::dsl::DslError::invalid_dsl(
                        ctx_named("masterplan"),
                        "unexpected text line; expected only PHASE commands",
                    ));
                }
            }
            crate::dsl::DslLine::Marker { marker } => {
                return Err(crate::dsl::DslError::invalid_dsl(
                    ctx_named("masterplan"),
                    format!("unexpected marker {marker}"),
                ));
            }
            crate::dsl::DslLine::Command { name, fields } => {
                if name != "PHASE" {
                    return Err(crate::dsl::DslError::unknown_command(
                        ctx_named("masterplan"),
                        name,
                    ));
                }
                let name = crate::dsl::require_field(fields, "name", &ctx_named("masterplan"))?;
                let objective =
                    crate::dsl::require_field(fields, "objective", &ctx_named("masterplan"))?;
                let success =
                    crate::dsl::require_field(fields, "success", &ctx_named("masterplan"))?;
                let deps = fields
                    .iter()
                    .find(|f| f.key == "deps")
                    .map(|f| f.value.as_str())
                    .unwrap_or("");
                let dependencies = deps
                    .split(',')
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>();
                phases.push(Phase {
                    name: name.to_string(),
                    objective: objective.to_string(),
                    success_criteria: success.to_string(),
                    dependencies,
                });
            }
        }
    }
    if phases.is_empty() {
        return Err(crate::dsl::DslError::missing_field(
            ctx_named("masterplan"),
            "PHASE",
        ));
    }
    if phases.len() > 6 {
        return Err(crate::dsl::DslError::invalid_dsl(
            ctx_named("masterplan"),
            "too many phases (max 6)",
        ));
    }
    Ok(Masterplan {
        goal: goal.to_string(),
        phases,
    })
}

fn parse_subgoals_dsl(input: &str) -> crate::dsl::DslResult<Vec<String>> {
    let input = input.trim();
    if input.is_empty() {
        return Err(crate::dsl::DslError::empty(ctx_named("subgoals")));
    }
    let parser = crate::dsl::DslBlockParser::new(ctx_named("subgoals"));
    let block = parser.parse_block(input, "END")?;
    if block.command != "SUBGOALS" {
        return Err(crate::dsl::DslError::unknown_command(
            ctx_named("subgoals"),
            &block.command,
        ));
    }
    let mut out = Vec::new();
    for line in &block.lines {
        match line {
            crate::dsl::DslLine::Text { text } => {
                if !text.trim().is_empty() {
                    return Err(crate::dsl::DslError::invalid_dsl(
                        ctx_named("subgoals"),
                        "unexpected text line; expected only SG commands",
                    ));
                }
            }
            crate::dsl::DslLine::Marker { marker } => {
                return Err(crate::dsl::DslError::invalid_dsl(
                    ctx_named("subgoals"),
                    format!("unexpected marker {marker}"),
                ));
            }
            crate::dsl::DslLine::Command { name, fields } => {
                if name != "SG" {
                    return Err(crate::dsl::DslError::unknown_command(
                        ctx_named("subgoals"),
                        name,
                    ));
                }
                let text = crate::dsl::require_field(fields, "text", &ctx_named("subgoals"))?;
                out.push(text.to_string());
            }
        }
    }
    if out.is_empty() {
        return Err(crate::dsl::DslError::missing_field(
            ctx_named("subgoals"),
            "SG",
        ));
    }
    if out.len() > 8 {
        return Err(crate::dsl::DslError::invalid_dsl(
            ctx_named("subgoals"),
            "too many subgoals (max 8)",
        ));
    }
    Ok(out)
}

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
- Clear ultimate goal (one sentence)
- 3-5 phases with objectives and success criteria
- Dependencies between phases (by phase name)

Return exactly one compact DSL block and nothing else:
MASTERPLAN goal="one sentence"
PHASE name="Phase name" objective="one sentence" success="one sentence" deps="Comma separated phase names or empty"
PHASE name="..." objective="..." success="..." deps="..."
END
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

    let masterplan = parse_masterplan_dsl(&text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse masterplan DSL: {}",
            crate::dsl::render_repair_hint_with_format(
                &e,
                "OBJECTIVE text=\"one line\" risk=low|medium|high\nGOAL text=\"...\" evidence_needed=true|false\nTASK id=N text=\"...\" status=ready|pending\nEND"
            )
        )
    })?;
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

Return exactly one compact DSL block and nothing else:
SUBGOALS
SG text="one subgoal"
SG text="..."
END
"#,
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

    let subgoals = parse_subgoals_dsl(&text).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse subgoals DSL: {}",
            crate::dsl::render_repair_hint_with_format(
                &e,
                "SUBGOALS\nSG text=\"one subgoal description\"\nSG text=\"...\"\nEND"
            )
        )
    })?;
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
