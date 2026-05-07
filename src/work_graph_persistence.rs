//! @efficiency-role: storage-state
//!
//! Work graph persistence — saves and loads pyramid work graphs to/from
//! JSON files in the session directory. Provides a flattened view of graph
//! nodes as PersistedGraphNode for listing and inspection.

use crate::work_graph::{ApproachId, NodeKind, NodeStatus, WorkGraph, WorkNode};
use crate::*;

/// Flattened representation of a work graph node for persistence and listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedGraphNode {
    pub(crate) id: String,
    pub(crate) goal: Option<String>,
    pub(crate) subgoal: Option<String>,
    pub(crate) plan: Option<String>,
    pub(crate) instruction: Option<String>,
    pub(crate) status: NodeStatus,
    pub(crate) approach_id: String,
    pub(crate) objective: String,
    pub(crate) depth: u8,
}

impl From<&WorkNode> for PersistedGraphNode {
    fn from(node: &WorkNode) -> Self {
        let (goal, subgoal, plan, instruction) = match node.kind {
            NodeKind::Goal => (Some(node.description.clone()), None, None, None),
            NodeKind::SubGoal => (None, Some(node.description.clone()), None, None),
            NodeKind::Plan => (None, None, Some(node.description.clone()), None),
            NodeKind::Instruction => (None, None, None, Some(node.description.clone())),
            NodeKind::Objective => (None, None, None, None),
        };

        Self {
            id: node.id.clone(),
            goal,
            subgoal,
            plan,
            instruction,
            status: node.status.clone(),
            approach_id: node.approach_id.0.clone(),
            objective: node.objective.clone(),
            depth: node.depth,
        }
    }
}

/// JSON persistence for pyramid work graphs in session directories.
pub(crate) struct WorkGraphPersistence;

impl WorkGraphPersistence {
    /// Save a work graph as JSON in the session directory under `work_graph/graph.json`.
    pub(crate) fn save(graph: &WorkGraph, session_root: &Path) -> Result<()> {
        let dir = session_root.join("work_graph");
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create work_graph dir in {}", session_root.display()))?;
        let path = dir.join("graph.json");
        let json = serde_json::to_string_pretty(graph).with_context(|| "serialize work graph")?;
        std::fs::write(&path, json).with_context(|| format!("write {}", path.display()))?;
        Ok(())
    }

    /// Load a work graph from a session directory.
    pub(crate) fn load(session_root: &Path) -> Option<WorkGraph> {
        let path = session_root.join("work_graph").join("graph.json");
        let json = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&json).ok()
    }

    /// List all persisted graph nodes as flattened PersistedGraphNode records,
    /// ordered by depth (shallowest first).
    pub(crate) fn list(session_root: &Path) -> Vec<PersistedGraphNode> {
        let graph = match Self::load(session_root) {
            Some(g) => g,
            None => return Vec::new(),
        };
        let mut nodes: Vec<PersistedGraphNode> = graph
            .nodes
            .values()
            .map(|n| PersistedGraphNode::from(n))
            .collect();
        nodes.sort_by(|a, b| a.depth.cmp(&b.depth));
        nodes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::work_graph::ApproachStatus;
    use tempfile::TempDir;

    fn create_test_graph() -> WorkGraph {
        let mut graph = WorkGraph::new("test objective".to_string());
        graph
            .approaches
            .insert("a1".to_string(), ApproachStatus::Active);

        graph.add_node(WorkNode {
            id: "g1".to_string(),
            kind: NodeKind::Goal,
            label: "Goal 1".to_string(),
            description: "Prepare environment".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "test objective".to_string(),
            status: NodeStatus::Pending,
            parent_id: None,
            depth: 0,
        });

        graph.add_node(WorkNode {
            id: "i1".to_string(),
            kind: NodeKind::Instruction,
            label: "Install".to_string(),
            description: "npm install express".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "test objective".to_string(),
            status: NodeStatus::Pending,
            parent_id: Some("g1".to_string()),
            depth: 1,
        });

        graph
    }

    #[test]
    fn test_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let graph = create_test_graph();

        WorkGraphPersistence::save(&graph, tmp.path()).unwrap();
        let loaded = WorkGraphPersistence::load(tmp.path()).unwrap();

        assert_eq!(loaded.root_objective, "test objective");
        assert_eq!(loaded.nodes.len(), 2);
    }

    #[test]
    fn test_load_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let loaded = WorkGraphPersistence::load(tmp.path());
        assert!(loaded.is_none());
    }

    #[test]
    fn test_list() {
        let tmp = TempDir::new().unwrap();
        let graph = create_test_graph();
        WorkGraphPersistence::save(&graph, tmp.path()).unwrap();

        let nodes = WorkGraphPersistence::list(tmp.path());
        assert_eq!(nodes.len(), 2);

        let goal = nodes.iter().find(|n| n.id == "g1").unwrap();
        assert_eq!(goal.goal.as_deref(), Some("Prepare environment"));
        assert_eq!(goal.approach_id, "a1");

        let instr = nodes.iter().find(|n| n.id == "i1").unwrap();
        assert_eq!(instr.instruction.as_deref(), Some("npm install express"));
        assert_eq!(instr.status, NodeStatus::Pending);
    }

    #[test]
    fn test_list_empty_session() {
        let tmp = TempDir::new().unwrap();
        let nodes = WorkGraphPersistence::list(tmp.path());
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_persisted_graph_node_from_work_node_goal() {
        let node = WorkNode {
            id: "g1".to_string(),
            kind: NodeKind::Goal,
            label: "G".to_string(),
            description: "Setup project".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::Pending,
            parent_id: None,
            depth: 0,
        };
        let pgn = PersistedGraphNode::from(&node);
        assert_eq!(pgn.goal.as_deref(), Some("Setup project"));
        assert!(pgn.subgoal.is_none());
        assert!(pgn.plan.is_none());
        assert!(pgn.instruction.is_none());
    }

    #[test]
    fn test_persisted_graph_node_from_work_node_subgoal() {
        let node = WorkNode {
            id: "sg1".to_string(),
            kind: NodeKind::SubGoal,
            label: "SG".to_string(),
            description: "Install deps".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::InProgress,
            parent_id: Some("g1".to_string()),
            depth: 1,
        };
        let pgn = PersistedGraphNode::from(&node);
        assert!(pgn.goal.is_none());
        assert_eq!(pgn.subgoal.as_deref(), Some("Install deps"));
        assert_eq!(pgn.status, NodeStatus::InProgress);
        assert_eq!(pgn.depth, 1);
    }

    #[test]
    fn test_persisted_graph_node_from_work_node_plan() {
        let node = WorkNode {
            id: "p1".to_string(),
            kind: NodeKind::Plan,
            label: "P".to_string(),
            description: "Run install".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::Succeeded,
            parent_id: Some("sg1".to_string()),
            depth: 2,
        };
        let pgn = PersistedGraphNode::from(&node);
        assert_eq!(pgn.plan.as_deref(), Some("Run install"));
        assert_eq!(pgn.status, NodeStatus::Succeeded);
    }

    #[test]
    fn test_persisted_graph_node_from_work_node_instruction() {
        let node = WorkNode {
            id: "i1".to_string(),
            kind: NodeKind::Instruction,
            label: "I".to_string(),
            description: "npm install".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::Failed,
            parent_id: Some("p1".to_string()),
            depth: 3,
        };
        let pgn = PersistedGraphNode::from(&node);
        assert_eq!(pgn.instruction.as_deref(), Some("npm install"));
        assert_eq!(pgn.status, NodeStatus::Failed);
    }

    #[test]
    fn test_list_ordering_by_depth() {
        let tmp = TempDir::new().unwrap();
        let mut graph = create_test_graph();
        graph.add_node(WorkNode {
            id: "i2".to_string(),
            kind: NodeKind::Instruction,
            label: "I2".to_string(),
            description: "test".to_string(),
            approach_id: ApproachId::from_str("a1"),
            objective: "obj".to_string(),
            status: NodeStatus::Pending,
            parent_id: Some("p1".to_string()),
            depth: 3,
        });
        WorkGraphPersistence::save(&graph, tmp.path()).unwrap();
        let nodes = WorkGraphPersistence::list(tmp.path());
        assert!(nodes[0].depth <= nodes[1].depth);
        assert!(nodes[1].depth <= nodes[2].depth);
    }
}
