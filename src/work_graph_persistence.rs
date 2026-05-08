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
    pub(crate) objective: Option<String>,
    pub(crate) subgoal: Option<String>,
    pub(crate) instruction: Option<String>,
    pub(crate) status: NodeStatus,
    pub(crate) approach_id: String,
    pub(crate) objective_text: String,
    pub(crate) depth: u8,
}

impl From<&WorkNode> for PersistedGraphNode {
    fn from(node: &WorkNode) -> Self {
        let (objective, subgoal, instruction) = match node.kind {
            NodeKind::Objective => (Some(node.description.clone()), None, None),
            NodeKind::SubGoal => (None, Some(node.description.clone()), None),
            NodeKind::Instruction => (None, None, Some(node.description.clone())),
        };

        Self {
            id: node.id.clone(),
            objective,
            subgoal,
            instruction,
            status: node.status.clone(),
            approach_id: node.approach_id.0.clone(),
            objective_text: node.objective.clone(),
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
        id: "obj1".to_string(),
        kind: NodeKind::Objective,
        label: "O1".to_string(),
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
        parent_id: Some("obj1".to_string()),
        depth: 2,
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

    let obj = nodes.iter().find(|n| n.id == "obj1").unwrap();
    assert_eq!(obj.objective.as_deref(), Some("Prepare environment"));
    assert_eq!(obj.approach_id, "a1");

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
fn test_persisted_graph_node_from_work_node_objective() {
    let node = WorkNode {
        id: "obj1".to_string(),
        kind: NodeKind::Objective,
        label: "O".to_string(),
        description: "Setup project".to_string(),
        approach_id: ApproachId::from_str("a1"),
        objective: "obj".to_string(),
        status: NodeStatus::Pending,
        parent_id: None,
        depth: 0,
    };
    let pgn = PersistedGraphNode::from(&node);
    assert_eq!(pgn.objective.as_deref(), Some("Setup project"));
    assert!(pgn.subgoal.is_none());
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
        parent_id: Some("obj1".to_string()),
        depth: 1,
    };
    let pgn = PersistedGraphNode::from(&node);
    assert_eq!(pgn.subgoal.as_deref(), Some("Install deps"));
    assert_eq!(pgn.status, NodeStatus::InProgress);
    assert_eq!(pgn.depth, 1);
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
        parent_id: Some("sg1".to_string()),
        depth: 2,
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
        description: "I2 desc".to_string(),
        approach_id: ApproachId::from_str("a1"),
        objective: "test objective".to_string(),
        status: NodeStatus::Pending,
        parent_id: Some("obj1".to_string()),
        depth: 2,
    });
    WorkGraphPersistence::save(&graph, tmp.path()).unwrap();

    let nodes = WorkGraphPersistence::list(tmp.path());
    assert_eq!(nodes.len(), 3);
    // Check ordering by depth
    assert_eq!(nodes[0].depth, 0); // obj1
    assert_eq!(nodes[1].depth, 2); // i1
    assert_eq!(nodes[2].depth, 2); // i2
}
}
