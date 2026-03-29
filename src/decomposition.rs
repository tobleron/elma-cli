//! Hierarchical Task Decomposition Module (Task 023)
//!
//! This module provides complexity-triggered hierarchical decomposition:
//! - OPEN_ENDED tasks → Full 5-level hierarchy (Goal→Subgoal→Task→Method→Action)
//! - MULTISTEP tasks → 3-level hierarchy (Subgoal→Task→Action)
//! - INVESTIGATE tasks → 2-level hierarchy (Task→Action)
//! - DIRECT tasks → 1-level (Action only, existing behavior)

use crate::*;

/// Determine required hierarchy depth based on complexity assessment
///
/// Returns depth 1-5:
/// - 1: Action only (DIRECT tasks)
/// - 2: Task → Action (simple INVESTIGATE)
/// - 3: Subgoal → Task → Action (MULTISTEP or INVESTIGATE+MEDIUM risk)
/// - 4: Task → Method → Action (HIGH risk)
/// - 5: Goal → Subgoal → Task → Method → Action (OPEN_ENDED)
pub fn get_required_depth(complexity: &str, risk: &str) -> u8 {
    match (complexity, risk) {
        ("DIRECT", _) => 1,
        ("INVESTIGATE", "LOW") => 2,
        ("INVESTIGATE", "MEDIUM") => 3,
        ("INVESTIGATE", "HIGH") => 4,
        ("MULTISTEP", _) => 3,
        ("OPEN_ENDED", _) => 5,
        (_, "HIGH") => 4,
        _ => 2, // Default to Task→Action for unknown cases
    }
}

/// Check if hierarchical decomposition is required
pub fn needs_decomposition(complexity: &ComplexityAssessment) -> bool {
    let depth = get_required_depth(&complexity.complexity, &complexity.risk);
    depth >= 3
}

/// Generate a masterplan for OPEN_ENDED or HIGH risk tasks
///
/// This is the Level 1 (Goal) decomposition that identifies major phases
pub async fn generate_masterplan(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    complexity: &ComplexityAssessment,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<Goal> {
    let system_prompt = format!(
        r#"You are Elma's strategic planner.

For OPEN_ENDED or HIGH risk tasks, generate a MASTERPLAN (Goal level).

Your job:
1. Identify the ultimate goal (final desired state)
2. Break it into 3-5 major phases
3. Define success criteria for each phase
4. Do NOT generate executable steps yet - this is strategic planning only

Output format (valid JSON):
{{
  "id": "goal_1",
  "description": "ultimate objective in one sentence",
  "success_state": "what the world looks like when complete",
  "phases": ["Phase 1 name", "Phase 2 name", "Phase 3 name"]
}}

Rules:
- Keep phases at strategic level (Discovery, Analysis, Synthesis, etc.)
- Do not mention specific commands or files
- Focus on WHAT needs to be achieved, not HOW
- 3-5 phases maximum
- Be specific to the user's request, not generic"#
    );

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "complexity": complexity,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
            },
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: 1024,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };

    let goal: Goal = chat_json_with_repair(client, chat_url, &req).await?;
    Ok(goal)
}

/// Decompose a goal into subgoals (Level 1 → Level 2)
pub async fn decompose_to_subgoals(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    goal: &Goal,
    user_message: &str,
    workspace_facts: &str,
) -> Result<Vec<Subgoal>> {
    let system_prompt = format!(
        r#"You are Elma's tactical planner.

Decompose a Goal into 3-5 Subgoals (milestones).

Each subgoal must:
- Be a measurable intermediate state
- Contribute directly to the parent goal
- Be achievable through multiple tasks
- Have clear success criteria

Output format (valid JSON array):
[
  {{
    "id": "sg_1",
    "parent_goal_id": "{goal_id}",
    "title": "Short title",
    "description": "What this subgoal achieves",
    "success_criteria": "How we know it's complete",
    "phase": "Which parent goal phase this belongs to",
    "completed": false
  }}
]

Rules:
- 3-5 subgoals maximum
- Each should be a significant milestone
- Order them logically (dependencies considered)
- Be specific to the workspace context"#,
        goal_id = goal.id
    );

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "goal": goal,
                    "user_message": user_message,
                    "workspace_facts": workspace_facts,
                })
                .to_string(),
            },
        ],
        temperature: 0.1,
        top_p: 1.0,
        stream: false,
        max_tokens: 2048,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };

    let subgoals: Vec<Subgoal> = chat_json_with_repair(client, chat_url, &req).await?;
    Ok(subgoals)
}

/// Decompose a subgoal into tasks (Level 2 → Level 3)
pub async fn decompose_to_tasks(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    subgoal: &Subgoal,
    workspace_facts: &str,
) -> Result<Vec<Task>> {
    let system_prompt = format!(
        r#"You are Elma's operational planner.

Decompose a Subgoal into 2-4 Tasks (work units).

Each task must:
- Be a coherent unit of work
- Contribute directly to the parent subgoal
- Be decomposable into methods/actions
- Have clear completion criteria

Output format (valid JSON array):
[
  {{
    "id": "task_1",
    "parent_subgoal_id": "{subgoal_id}",
    "title": "Short title",
    "description": "What this task achieves",
    "decomposable": true,
    "completed": false
  }}
]

Rules:
- 2-4 tasks per subgoal
- Each should be achievable in one session
- Order logically (dependencies considered)
- Be specific to the workspace context"#,
        subgoal_id = subgoal.id
    );

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "subgoal": subgoal,
                    "workspace_facts": workspace_facts,
                })
                .to_string(),
            },
        ],
        temperature: 0.1,
        top_p: 1.0,
        stream: false,
        max_tokens: 2048,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };

    let tasks: Vec<Task> = chat_json_with_repair(client, chat_url, &req).await?;
    Ok(tasks)
}

/// Generate methods for a task (Level 3 → Level 4)
pub async fn generate_methods(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    task: &Task,
    workspace_facts: &str,
) -> Result<Vec<Method>> {
    let system_prompt = format!(
        r#"You are Elma's method planner.

Generate 1-2 Methods for how to decompose this Task into Actions.

Each method specifies:
- The strategy name (e.g., "systematic_enumeration", "iterative_refinement")
- Ordered list of action descriptions
- Which actions will be needed

Output format (valid JSON array):
[
  {{
    "id": "method_1",
    "parent_task_id": "{task_id}",
    "strategy": "strategy_name",
    "decomposition": ["Step 1 description", "Step 2 description", "Step 3 description"],
    "actions": []
  }}
]

Rules:
- 1-2 methods per task
- Each method should be a complete approach
- Actions will be generated later from decomposition
- Be specific about the approach"#,
        task_id = task.id
    );

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "task": task,
                    "workspace_facts": workspace_facts,
                })
                .to_string(),
            },
        ],
        temperature: 0.1,
        top_p: 1.0,
        stream: false,
        max_tokens: 2048,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };

    let methods: Vec<Method> = chat_json_with_repair(client, chat_url, &req).await?;
    Ok(methods)
}

/// Validate hierarchy integrity
///
/// Ensures:
/// - Every unit has correct parent references
/// - No orphan units exist
/// - Depth levels are consistent
pub fn validate_hierarchy(goal: &Goal, subgoals: &[Subgoal], tasks: &[Task], methods: &[Method]) -> Result<()> {
    // Check all subgoals reference valid parent goal
    for sg in subgoals {
        if sg.parent_goal_id != goal.id {
            anyhow::bail!("Subgoal {} has invalid parent_goal_id: {}", sg.id, sg.parent_goal_id);
        }
    }

    // Check all tasks reference valid parent subgoals
    let subgoal_ids: std::collections::HashSet<&str> = subgoals.iter().map(|s| s.id.as_str()).collect();
    for task in tasks {
        if !subgoal_ids.contains(task.parent_subgoal_id.as_str()) {
            anyhow::bail!("Task {} has invalid parent_subgoal_id: {}", task.id, task.parent_subgoal_id);
        }
    }

    // Check all methods reference valid parent tasks
    let task_ids: std::collections::HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    for method in methods {
        if !task_ids.contains(method.parent_task_id.as_str()) {
            anyhow::bail!("Method {} has invalid parent_task_id: {}", method.id, method.parent_task_id);
        }
    }

    Ok(())
}
