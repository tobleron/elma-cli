//! @efficiency-role: storage-state
//!
//! Pyramid work graph types — Task 389.
//! Represents Objective -> Goals -> Sub-Goals -> Plans -> Instructions as
//! a directed acyclic graph in Rust state. Model calls only fill one field
//! set at a time. Each node carries the original user objective for
//! semantic continuity (Task 380 integration point).

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Unique identifier for an approach branch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct ApproachId(pub(crate) String);

impl ApproachId {
    pub fn new() -> Self {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        Self(format!("a_{}_{}", ts.as_secs(), ts.subsec_nanos()))
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl Default for ApproachId {
    fn default() -> Self {
        Self::new()
    }
}

/// Status of an approach branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ApproachStatus {
    Active,
    Succeeded,
    Failed,
    Pruned,
    Superseded,
}

/// Kinds of nodes in the work graph.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum NodeKind {
    Objective,
    Goal,
    SubGoal,
    Plan,
    Instruction,
}

impl NodeKind {
    /// Human-readable label for the node kind.
    pub fn label(&self) -> &'static str {
        match self {
            NodeKind::Objective => "Objective",
            NodeKind::Goal => "Goal",
            NodeKind::SubGoal => "Sub-Goal",
            NodeKind::Plan => "Plan",
            NodeKind::Instruction => "Instruction",
        }
    }
}

/// Execution status of a single node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum NodeStatus {
    Pending,
    InProgress,
    Succeeded,
    Failed,
    Skipped,
}

/// A single node in the pyramid work graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorkNode {
    pub(crate) id: String,
    pub(crate) kind: NodeKind,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) approach_id: ApproachId,
    /// Original user objective, preserved through the entire graph.
    pub(crate) objective: String,
    pub(crate) status: NodeStatus,
    pub(crate) parent_id: Option<String>,
    pub(crate) depth: u8,
}

/// The pyramid work graph — a collection of nodes arranged in a hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorkGraph {
    pub(crate) nodes: HashMap<String, WorkNode>,
    /// The top-level user request that this graph serves.
    pub(crate) root_objective: String,
    pub(crate) approaches: HashMap<String, ApproachStatus>,
    pub(crate) created_at: String,
}

impl WorkGraph {
    pub fn new(objective: String) -> Self {
        Self {
            nodes: HashMap::new(),
            root_objective: objective,
            approaches: HashMap::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn add_node(&mut self, node: WorkNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    pub fn get_node(&self, id: &str) -> Option<&WorkNode> {
        self.nodes.get(id)
    }

    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut WorkNode> {
        self.nodes.get_mut(id)
    }

    /// Returns all direct children of the given parent node.
    pub fn children_of(&self, parent_id: &str) -> Vec<&WorkNode> {
        self.nodes
            .values()
            .filter(|n| n.parent_id.as_deref() == Some(parent_id))
            .collect()
    }

    /// Returns all nodes of a given kind.
    pub fn nodes_by_kind(&self, kind: NodeKind) -> Vec<&WorkNode> {
        self.nodes
            .values()
            .filter(|n| n.kind == kind)
            .collect()
    }

    /// Update the status of a node by id.
    pub fn set_node_status(&mut self, id: &str, status: NodeStatus) {
        if let Some(node) = self.nodes.get_mut(id) {
            node.status = status;
        }
    }

    /// Total depth of the graph (max depth across all nodes).
    pub fn max_depth(&self) -> u8 {
        self.nodes.values().map(|n| n.depth).max().unwrap_or(0)
    }

    /// Number of goals (top-level nodes below objective).
    pub fn goal_count(&self) -> usize {
        self.nodes_by_kind(NodeKind::Goal).len()
    }

    /// Returns node IDs in topological order (parents before children).
    pub fn topological_ids(&self) -> Vec<&str> {
        let mut result: Vec<&str> = Vec::new();
        let mut added = std::collections::HashSet::new();

        for depth in 0..=self.max_depth() {
            for node in self.nodes.values() {
                if node.depth == depth && !added.contains(node.id.as_str()) {
                    result.push(node.id.as_str());
                    added.insert(node.id.as_str());
                }
            }
        }
        result
    }
}

/// Builds a work graph one layer at a time.
pub(crate) struct WorkGraphBuilder {
    graph: WorkGraph,
    current_approach: ApproachId,
}

impl WorkGraphBuilder {
    pub fn new(objective: String) -> Self {
        let approach_id = ApproachId::new();
        let mut graph = WorkGraph::new(objective);
        graph
            .approaches
            .insert(approach_id.0.clone(), ApproachStatus::Active);
        Self {
            graph,
            current_approach: approach_id,
        }
    }

    pub fn graph(&self) -> &WorkGraph {
        &self.graph
    }

    pub fn into_graph(self) -> WorkGraph {
        self.graph
    }

    pub fn approach_id(&self) -> &ApproachId {
        &self.current_approach
    }

    pub fn add_goal(&mut self, id: &str, label: &str, description: &str) -> &mut Self {
        self.graph.add_node(WorkNode {
            id: id.to_string(),
            kind: NodeKind::Goal,
            label: label.to_string(),
            description: description.to_string(),
            approach_id: self.current_approach.clone(),
            objective: self.graph.root_objective.clone(),
            status: NodeStatus::Pending,
            parent_id: None,
            depth: 0,
        });
        self
    }

    pub fn add_sub_goal(
        &mut self,
        id: &str,
        label: &str,
        description: &str,
        parent_id: &str,
    ) -> &mut Self {
        let depth = self
            .graph
            .get_node(parent_id)
            .map(|n| n.depth + 1)
            .unwrap_or(1);

        self.graph.add_node(WorkNode {
            id: id.to_string(),
            kind: NodeKind::SubGoal,
            label: label.to_string(),
            description: description.to_string(),
            approach_id: self.current_approach.clone(),
            objective: self.graph.root_objective.clone(),
            status: NodeStatus::Pending,
            parent_id: Some(parent_id.to_string()),
            depth,
        });
        self
    }

    pub fn add_plan(
        &mut self,
        id: &str,
        label: &str,
        description: &str,
        parent_id: &str,
    ) -> &mut Self {
        let depth = self
            .graph
            .get_node(parent_id)
            .map(|n| n.depth + 1)
            .unwrap_or(2);

        self.graph.add_node(WorkNode {
            id: id.to_string(),
            kind: NodeKind::Plan,
            label: label.to_string(),
            description: description.to_string(),
            approach_id: self.current_approach.clone(),
            objective: self.graph.root_objective.clone(),
            status: NodeStatus::Pending,
            parent_id: Some(parent_id.to_string()),
            depth,
        });
        self
    }

    pub fn add_instruction(
        &mut self,
        id: &str,
        label: &str,
        description: &str,
        parent_id: &str,
    ) -> &mut Self {
        let depth = self
            .graph
            .get_node(parent_id)
            .map(|n| n.depth + 1)
            .unwrap_or(3);

        self.graph.add_node(WorkNode {
            id: id.to_string(),
            kind: NodeKind::Instruction,
            label: label.to_string(),
            description: description.to_string(),
            approach_id: self.current_approach.clone(),
            objective: self.graph.root_objective.clone(),
            status: NodeStatus::Pending,
            parent_id: Some(parent_id.to_string()),
            depth,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = WorkGraph::new("test objective".to_string());
        assert!(graph.nodes.is_empty());
        assert_eq!(graph.root_objective, "test objective");
        assert!(graph.approaches.is_empty());
    }

    #[test]
    fn test_add_and_retrieve_node() {
        let mut graph = WorkGraph::new("obj".to_string());
        let node = WorkNode {
            id: "g1".to_string(),
            kind: NodeKind::Goal,
            label: "Goal 1".to_string(),
            description: "First goal".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::Pending,
            parent_id: None,
            depth: 0,
        };
        graph.add_node(node);
        assert!(graph.get_node("g1").is_some());
        assert_eq!(graph.nodes.len(), 1);
    }

    #[test]
    fn test_builder_creates_approach() {
        let builder = WorkGraphBuilder::new("test".to_string());
        assert_eq!(builder.graph.approaches.len(), 1);
        assert!(builder
            .graph
            .approaches
            .values()
            .any(|s| *s == ApproachStatus::Active));
    }

    #[test]
    fn test_builder_full_hierarchy() {
        let mut builder = WorkGraphBuilder::new("Build feature".to_string());
        builder
            .add_goal("g1", "Setup", "Prepare environment")
            .add_sub_goal("sg1", "Install deps", "Install dependencies", "g1")
            .add_plan("p1", "Run install", "Execute npm install", "sg1")
            .add_instruction("i1", "npm install", "npm install express", "p1");
        let graph = builder.into_graph();
        assert_eq!(graph.nodes.len(), 4);
        assert!(graph.get_node("g1").is_some());
        assert!(graph.get_node("sg1").is_some());
        assert!(graph.get_node("p1").is_some());
        assert!(graph.get_node("i1").is_some());
    }

    #[test]
    fn test_node_depth() {
        let mut builder = WorkGraphBuilder::new("obj".to_string());
        builder
            .add_goal("g1", "G1", "")
            .add_sub_goal("sg1", "SG1", "", "g1")
            .add_plan("p1", "P1", "", "sg1")
            .add_instruction("i1", "I1", "", "p1");
        let graph = builder.into_graph();
        assert_eq!(graph.get_node("g1").unwrap().depth, 0);
        assert_eq!(graph.get_node("sg1").unwrap().depth, 1);
        assert_eq!(graph.get_node("p1").unwrap().depth, 2);
        assert_eq!(graph.get_node("i1").unwrap().depth, 3);
    }

    #[test]
    fn test_children_of() {
        let mut builder = WorkGraphBuilder::new("obj".to_string());
        builder
            .add_goal("g1", "G1", "")
            .add_sub_goal("sg1", "SG1", "", "g1")
            .add_sub_goal("sg2", "SG2", "", "g1");
        let graph = builder.into_graph();
        let children = graph.children_of("g1");
        assert_eq!(children.len(), 2);
    }

    #[test]
    fn test_set_node_status() {
        let mut graph = WorkGraph::new("obj".to_string());
        graph.add_node(WorkNode {
            id: "g1".to_string(),
            kind: NodeKind::Goal,
            label: "G1".to_string(),
            description: "".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::Pending,
            parent_id: None,
            depth: 0,
        });
        graph.set_node_status("g1", NodeStatus::Succeeded);
        assert_eq!(graph.get_node("g1").unwrap().status, NodeStatus::Succeeded);
    }

    #[test]
    fn test_nodes_by_kind() {
        let mut builder = WorkGraphBuilder::new("obj".to_string());
        builder
            .add_goal("g1", "G1", "")
            .add_goal("g2", "G2", "")
            .add_sub_goal("sg1", "SG1", "", "g1");
        let graph = builder.into_graph();
        assert_eq!(graph.nodes_by_kind(NodeKind::Goal).len(), 2);
        assert_eq!(graph.nodes_by_kind(NodeKind::SubGoal).len(), 1);
    }

    #[test]
    fn test_approach_id_uniqueness() {
        let id1 = ApproachId::new();
        let id2 = ApproachId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_topological_order() {
        let mut builder = WorkGraphBuilder::new("obj".to_string());
        builder
            .add_goal("g1", "G1", "")
            .add_sub_goal("sg1", "SG1", "", "g1")
            .add_plan("p1", "P1", "", "sg1")
            .add_instruction("i1", "I1", "", "p1");
        let graph = builder.into_graph();
        let order = graph.topological_ids();
        let g1_pos = order.iter().position(|&id| id == "g1").unwrap();
        let sg1_pos = order.iter().position(|&id| id == "sg1").unwrap();
        let p1_pos = order.iter().position(|&id| id == "p1").unwrap();
        let i1_pos = order.iter().position(|&id| id == "i1").unwrap();
        assert!(g1_pos < sg1_pos);
        assert!(sg1_pos < p1_pos);
        assert!(p1_pos < i1_pos);
    }

    #[test]
    fn test_node_kind_label() {
        assert_eq!(NodeKind::Objective.label(), "Objective");
        assert_eq!(NodeKind::Goal.label(), "Goal");
        assert_eq!(NodeKind::SubGoal.label(), "Sub-Goal");
        assert_eq!(NodeKind::Plan.label(), "Plan");
        assert_eq!(NodeKind::Instruction.label(), "Instruction");
    }

    #[test]
    fn test_max_depth() {
        let mut builder = WorkGraphBuilder::new("obj".to_string());
        builder
            .add_goal("g1", "G1", "")
            .add_sub_goal("sg1", "SG1", "", "g1")
            .add_plan("p1", "P1", "", "sg1")
            .add_instruction("i1", "I1", "", "p1");
        let graph = builder.into_graph();
        assert_eq!(graph.max_depth(), 3);
    }

    #[test]
    fn test_empty_graph_properties() {
        let graph = WorkGraph::new("empty".to_string());
        assert_eq!(graph.max_depth(), 0);
        assert_eq!(graph.goal_count(), 0);
        assert!(graph.topological_ids().is_empty());
    }
}
