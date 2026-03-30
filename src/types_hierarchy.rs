//! @efficiency-role: data-model
//!
//! Types - Hierarchy Support (Task 023)

use serde::{Deserialize, Serialize};

/// Hierarchical unit types for task decomposition
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum HierarchyUnitType {
    #[serde(rename = "goal")]
    Goal,
    #[serde(rename = "subgoal")]
    Subgoal,
    #[serde(rename = "task")]
    Task,
    #[serde(rename = "method")]
    Method,
    #[serde(rename = "action")]
    Action,
}

/// Level 1: GOAL - The ultimate desired state
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Goal {
    pub id: String,
    pub description: String,
    pub success_state: String,
    pub phases: Vec<String>,
    #[serde(default)]
    pub subgoals: Vec<String>,
    #[serde(default)]
    pub metadata: Option<HierarchyMetadata>,
}

/// Level 2: SUBGOAL - Intermediate milestone toward goal
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Subgoal {
    pub id: String,
    pub parent_goal_id: String,
    pub title: String,
    pub description: String,
    pub success_criteria: String,
    pub phase: String,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub metadata: Option<HierarchyMetadata>,
}

/// Level 3: TASK - Work unit that achieves a subgoal
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Task {
    pub id: String,
    pub parent_subgoal_id: String,
    pub title: String,
    pub description: String,
    pub decomposable: bool,
    #[serde(default)]
    pub methods: Vec<String>,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub metadata: Option<HierarchyMetadata>,
}

/// Level 4: METHOD - Specifies how to decompose a task into actions
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Method {
    pub id: String,
    pub parent_task_id: String,
    pub strategy: String,
    pub decomposition: Vec<String>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub metadata: Option<HierarchyMetadata>,
}

/// Metadata for hierarchy units
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HierarchyMetadata {
    #[serde(default)]
    pub estimated_complexity: Option<String>,
    #[serde(default)]
    pub estimated_duration: Option<String>,
    #[serde(default)]
    pub risk_level: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub notes: String,
}

/// Progress tracking for hierarchical execution
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct HierarchyProgress {
    pub goal_id: Option<String>,
    pub goal_complete: bool,
    #[serde(default)]
    pub subgoals_total: u32,
    #[serde(default)]
    pub subgoals_complete: Vec<String>,
    #[serde(default)]
    pub current_subgoal: Option<String>,
    #[serde(default)]
    pub current_task: Option<String>,
    #[serde(default)]
    pub actions_executed: u32,
    #[serde(default)]
    pub actions_total: u32,
    #[serde(default)]
    pub last_action_result: Option<String>,
}

/// Decomposition result
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Decomposition {
    pub parent_id: String,
    pub parent_type: String,
    pub children: Vec<HierarchyUnit>,
    pub strategy_used: String,
}

/// Unified hierarchy unit enum
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "unit_type")]
pub enum HierarchyUnit {
    #[serde(rename = "goal")]
    Goal(Goal),
    #[serde(rename = "subgoal")]
    Subgoal(Subgoal),
    #[serde(rename = "task")]
    Task(Task),
    #[serde(rename = "method")]
    Method(Method),
}

impl HierarchyUnit {
    pub fn id(&self) -> &str {
        match self {
            HierarchyUnit::Goal(g) => &g.id,
            HierarchyUnit::Subgoal(s) => &s.id,
            HierarchyUnit::Task(t) => &t.id,
            HierarchyUnit::Method(m) => &m.id,
        }
    }

    pub fn depth(&self) -> u8 {
        match self {
            HierarchyUnit::Goal(_) => 1,
            HierarchyUnit::Subgoal(_) => 2,
            HierarchyUnit::Task(_) => 3,
            HierarchyUnit::Method(_) => 4,
        }
    }
}
