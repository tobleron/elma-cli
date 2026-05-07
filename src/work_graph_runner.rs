//! @efficiency-role: service-orchestrator
//!
//! Work Graph Runner — Tasks 763-769 integration.
//!
//! Bridges the work graph (Goal → SubGoal → Plan → Instruction),
//! approach engine, scope coverage, and objective state into the active
//! tool-calling loop. For MULTISTEP and OPEN_ENDED complexity, the runner
//! walks graph nodes in topological order, enforces sub-goal commitment,
//! derives coverage from graph nodes, and gates finalization on graph
//! completion + coverage satisfaction.
//!
//! Philosophy:
//!   Relaxed  = No pressure to finalize early. Generous budgets.
//!   Laser focused = Commit to a node, resist switching until effort spent.

use crate::*;
use crate::scope_coverage::{CoverageItem, CoverageStatus, ScopeCoverageLedger};
use crate::work_graph::{ApproachId, NodeKind, NodeStatus, WorkGraph, WorkNode};
use std::collections::HashSet;

/// Types re-exported from intel_units for schema population.
use crate::intel_units::{SchemaPhase, WorkGraphSchema};

/// Strategy the runner uses per turn.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RunnerMode {
    /// DIRECT / INVESTIGATE: bypass graph, flat direct tool-calling as before.
    Direct,
    /// MULTISTEP / OPEN_ENDED: graph-driven execution with node commitment.
    GraphDriven,
}

/// Progress state of the current graph node.
#[derive(Debug, Clone)]
pub(crate) struct NodeProgress {
    pub node_id: String,
    pub node_kind: String, // "Goal", "SubGoal", "Instruction"
    pub node_label: String,
    pub iterations_spent: u32,
    pub tool_calls_spent: u32,
    pub consecutive_failures: u32,
}

/// The work graph runner. Created per turn for non-DIRECT complexity.
#[derive(Debug, Clone)]
pub(crate) struct WorkGraphRunner {
    pub mode: RunnerMode,
    /// The execution plan (null for Direct mode).
    pub graph: Option<WorkGraph>,
    /// Coverage ledger derived from graph nodes + discovery.
    pub coverage: crate::scope_coverage::ScopeCoverageLedger,
    /// Current node being worked on (None if no graph).
    pub current_progress: Option<NodeProgress>,
}

impl WorkGraphRunner {
    /// Create for a given complexity level and user objective.
    pub fn new(complexity: &str, raw_objective: &str) -> Self {
        let is_graph = matches!(
            complexity.to_ascii_uppercase().as_str(),
            "MULTISTEP" | "OPEN_ENDED"
        );
        if is_graph {
            let graph = WorkGraph::new(raw_objective.to_string());
            Self {
                mode: RunnerMode::GraphDriven,
                graph: Some(graph),
                coverage: crate::scope_coverage::ScopeCoverageLedger::new(),
                current_progress: None,
            }
        } else {
            Self {
                mode: RunnerMode::Direct,
                graph: None,
                coverage: crate::scope_coverage::ScopeCoverageLedger::new(),
                current_progress: None,
            }
        }
    }

    /// Whether the runner is using graph-driven execution.
    pub fn is_graph_driven(&self) -> bool {
        self.mode == RunnerMode::GraphDriven
    }

    // ── graph population from schema + discovery ─────────────────────────

    /// Populate the work graph from a model-produced schema.
    /// Converts abstract phases into concrete Goal/SubGoal nodes.
    /// Returns the number of nodes created.
    pub fn populate_from_schema(&mut self, schema: &WorkGraphSchema) -> usize {
        if self.mode == RunnerMode::Direct {
            return 0;
        }
        let graph = match self.graph.as_mut() {
            Some(g) => g,
            None => return 0,
        };

        let mut count = 0;
        let mut phase_num: u32 = 0;

        for phase in &schema.phases {
            phase_num += 1;
            let goal_id = format!("g_{:02}", phase_num);
            match phase.action.as_str() {
                "discover" => {
                    // Single Goal: discover files in a scope
                    graph.add_node(WorkNode {
                        id: goal_id.clone(),
                        kind: NodeKind::Goal,
                        label: phase.label.clone(),
                        description: format!("discover: {}", phase.scope_hint),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: None,
                        depth: 0,
                    });
                    graph.add_node(WorkNode {
                        id: format!("{}_d1", goal_id),
                        kind: NodeKind::SubGoal,
                        label: format!("List {}", phase.scope_hint),
                        description: phase.scope_hint.clone(),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: Some(goal_id.clone()),
                        depth: 1,
                    });
                    count += 2;
                }
                "read_all" => {
                    // Goal: read all files from a previously discovered scope.
                    // SubGoal nodes will be added after discovery provides file paths.
                    graph.add_node(WorkNode {
                        id: goal_id.clone(),
                        kind: NodeKind::Goal,
                        label: phase.label.clone(),
                        description: format!("read_all: {}", phase.scope_hint),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: None,
                        depth: 0,
                    });
                    // Placeholder SubGoal — gets replaced/expanded after discovery
                    graph.add_node(WorkNode {
                        id: format!("{}_sg_placeholder", goal_id),
                        kind: NodeKind::SubGoal,
                        label: format!("Read files from {}", phase.scope_hint),
                        description: "Placeholder — will be expanded after discovery".to_string(),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: Some(goal_id.clone()),
                        depth: 1,
                    });
                    count += 2;
                }
                "read_one" => {
                    graph.add_node(WorkNode {
                        id: goal_id.clone(),
                        kind: NodeKind::Goal,
                        label: phase.label.clone(),
                        description: format!("read_one: {}", phase.scope_hint),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: None,
                        depth: 0,
                    });
                    graph.add_node(WorkNode {
                        id: format!("{}_sg1", goal_id),
                        kind: NodeKind::SubGoal,
                        label: phase.label.clone(),
                        description: phase.scope_hint.clone(),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: Some(goal_id.clone()),
                        depth: 1,
                    });
                    count += 2;
                }
                "shell" => {
                    graph.add_node(WorkNode {
                        id: goal_id.clone(),
                        kind: NodeKind::Goal,
                        label: phase.label.clone(),
                        description: format!("shell: {}", phase.scope_hint),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: None,
                        depth: 0,
                    });
                    count += 1;
                }
                "answer" => {
                    // Answer phases have no sub-goals — synthesis only
                    graph.add_node(WorkNode {
                        id: goal_id.clone(),
                        kind: NodeKind::Goal,
                        label: phase.label.clone(),
                        description: "Synthesize final answer from gathered evidence".to_string(),
                        approach_id: ApproachId::default(),
                        objective: graph.root_objective.clone(),
                        status: NodeStatus::Pending,
                        parent_id: None,
                        depth: 0,
                    });
                    count += 1;
                }
                _ => {}
            }
        }
        count
    }

    /// After workspace discovery (ls/glob output), expand "read_all" phases
    /// with concrete SubGoal nodes for each discovered file path.
    /// Also seeds coverage items from the expanded nodes.
    pub fn populate_from_discovery(
        &mut self,
        discovered_paths: &[String],
    ) -> usize {
        let graph = match self.graph.as_mut() {
            Some(g) => g,
            None => return 0,
        };
        if discovered_paths.is_empty() {
            return 0;
        }

        // Find read_all goals that have placeholder SubGoal nodes
        let read_all_goals: Vec<String> = graph
            .nodes
            .values()
            .filter(|n| n.kind == NodeKind::Goal && n.description.starts_with("read_all:"))
            .map(|n| n.id.clone())
            .collect();

        // Collect all data we need before mutating the graph.
        struct GoalInfo {
            id: String,
            goal_depth: u8,
            existing_children: Vec<String>,
        }
        let goal_infos: Vec<GoalInfo> = read_all_goals
            .iter()
            .filter_map(|gid| {
                let node = graph.get_node(gid)?;
                let children: Vec<String> = graph
                    .children_of(gid)
                    .iter()
                    .map(|n| n.id.clone())
                    .collect();
                Some(GoalInfo {
                    id: gid.clone(),
                    goal_depth: node.depth,
                    existing_children: children,
                })
            })
            .collect();

        let mut created = 0;
        let mut seen = HashSet::new();

        for goal_info in &goal_infos {
            for path in discovered_paths {
                if !seen.insert(path.clone()) {
                    continue;
                }
                let base = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(path);
                let sg_id = format!("{}_sg_{}", goal_info.id, base.replace('.', "_"));
                if goal_info.existing_children.iter().any(|e| e == &sg_id) {
                    continue;
                }
                let label = format!("Read {}", path);
                graph.add_node(WorkNode {
                    id: sg_id,
                    kind: NodeKind::SubGoal,
                    label,
                    description: path.clone(),
                    approach_id: ApproachId::default(),
                    objective: graph.root_objective.clone(),
                    status: NodeStatus::Pending,
                    parent_id: Some(goal_info.id.clone()),
                    depth: goal_info.goal_depth + 1,
                });
                created += 1;
            }
        }

        // Remove placeholder nodes (they served their purpose)
        let placeholders: Vec<String> = graph
            .nodes
            .iter()
            .filter(|(id, n)| {
                id.contains("_sg_placeholder") && n.kind == NodeKind::SubGoal
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in placeholders {
            graph.nodes.remove(&id);
        }

        // Seed coverage from the newly created SubGoal nodes
        self.seed_coverage_from_graph();

        created
    }

    // ── graph population ─────────────────────────────────────────────────

    /// Add a Goal node to the graph.
    pub fn add_goal(&mut self, id: &str, label: &str, description: &str) {
        if let Some(ref mut graph) = self.graph {
            graph.add_node(WorkNode {
                id: id.to_string(),
                kind: NodeKind::Goal,
                label: label.to_string(),
                description: description.to_string(),
                approach_id: ApproachId::default(),
                objective: graph.root_objective.clone(),
                status: NodeStatus::Pending,
                parent_id: None,
                depth: 0,
            });
        }
    }

    /// Add a SubGoal node with a parent Goal.
    pub fn add_sub_goal(
        &mut self,
        id: &str,
        label: &str,
        description: &str,
        parent_id: &str,
    ) {
        if let Some(ref mut graph) = self.graph {
            let depth = graph.get_node(parent_id).map(|n| n.depth + 1).unwrap_or(1);
            graph.add_node(WorkNode {
                id: id.to_string(),
                kind: NodeKind::SubGoal,
                label: label.to_string(),
                description: description.to_string(),
                approach_id: ApproachId::default(),
                objective: graph.root_objective.clone(),
                status: NodeStatus::Pending,
                parent_id: Some(parent_id.to_string()),
                depth,
            });
        }
    }

    /// Add an Instruction node.
    pub fn add_instruction(
        &mut self,
        id: &str,
        label: &str,
        description: &str,
        parent_id: &str,
    ) {
        if let Some(ref mut graph) = self.graph {
            let depth = graph.get_node(parent_id).map(|n| n.depth + 1).unwrap_or(2);
            graph.add_node(WorkNode {
                id: id.to_string(),
                kind: NodeKind::Instruction,
                label: label.to_string(),
                description: description.to_string(),
                approach_id: ApproachId::default(),
                objective: graph.root_objective.clone(),
                status: NodeStatus::Pending,
                parent_id: Some(parent_id.to_string()),
                depth,
            });
        }
    }

    // ── node traversal ───────────────────────────────────────────────────

    /// Get the next Pending node in topological order, or None if all terminal.
    pub fn next_pending_node(&self) -> Option<(&str, &str, &str, Vec<&str>)> {
        let graph = self.graph.as_ref()?;
        let order = graph.topological_ids();
        for &id in order.iter() {
            let node = graph.get_node(id)?;
            if node.status == NodeStatus::Pending {
                // Collect parent labels for context
                let parent_labels: Vec<&str> = node
                    .parent_id
                    .as_ref()
                    .and_then(|pid| graph.get_node(pid))
                    .map(|p| p.label.as_str())
                    .into_iter()
                    .collect();
                return Some((node.id.as_str(), node.label.as_str(), node.kind.label(), parent_labels));
            }
        }
        None
    }

    /// Commit to working on a specific node.
    pub fn commit_to_node(&mut self, node_id: &str) {
        if let Some(ref mut graph) = self.graph {
            if let Some(node) = graph.get_node_mut(node_id) {
                node.status = NodeStatus::InProgress;
            }
        }
        self.current_progress = Some(NodeProgress {
            node_id: node_id.to_string(),
            node_kind: String::new(),
            node_label: String::new(),
            iterations_spent: 0,
            tool_calls_spent: 0,
            consecutive_failures: 0,
        });
    }

    /// Get the next pending node and commit to it in one call.
    /// Returns Some(label) if committed, None if no pending nodes.
    pub fn advance_to_next_node(&mut self) -> Option<(String, String, Vec<String>)> {
        // Collect node info before mutating self.
        let (node_id, label, kind, parents) = self.next_pending_node()?;
        let label_owned = label.to_string();
        let kind_owned = kind.to_string();
        let parent_labels: Vec<String> = parents.iter().map(|s| s.to_string()).collect();
        let node_id_owned = node_id.to_string();

        if let Some(ref mut graph) = self.graph {
            if let Some(node) = graph.get_node_mut(&node_id_owned) {
                node.status = NodeStatus::InProgress;
            }
        }
        self.current_progress = Some(NodeProgress {
            node_id: node_id_owned,
            node_kind: kind_owned.clone(),
            node_label: label_owned.clone(),
            iterations_spent: 0,
            tool_calls_spent: 0,
            consecutive_failures: 0,
        });
        Some((label_owned, kind_owned, parent_labels))
    }

    /// Mark the current node as Succeeded.
    pub fn mark_current_node_succeeded(&mut self) {
        if let Some(ref progress) = self.current_progress {
            if let Some(ref mut graph) = self.graph {
                graph.set_node_status(&progress.node_id, NodeStatus::Succeeded);
            }
        }
    }

    /// Mark the current node as Failed.
    pub fn mark_current_node_failed(&mut self) {
        if let Some(ref progress) = self.current_progress {
            if let Some(ref mut graph) = self.graph {
                graph.set_node_status(&progress.node_id, NodeStatus::Failed);
            }
        }
    }

    /// Record that the current node has spent another iteration.
    pub fn record_iteration(&mut self) {
        if let Some(ref mut progress) = self.current_progress {
            progress.iterations_spent += 1;
        }
    }

    /// Record a tool call on the current node.
    pub fn record_tool_call(&mut self, success: bool) {
        if let Some(ref mut progress) = self.current_progress {
            progress.tool_calls_spent += 1;
            if !success {
                progress.consecutive_failures += 1;
            } else {
                progress.consecutive_failures = 0;
            }
        }
    }

    /// Whether the current node has spent enough effort to justify switching.
    pub fn current_node_effort_sufficient(&self) -> bool {
        if let Some(ref progress) = self.current_progress {
            progress.iterations_spent >= 3 || progress.tool_calls_spent >= 2
        } else {
            false
        }
    }

    /// Whether the current node is stuck (consecutive failures).
    pub fn current_node_is_stuck(&self) -> bool {
        if let Some(ref progress) = self.current_progress {
            progress.consecutive_failures >= 3
        } else {
            false
        }
    }

    // ── coverage seeding from graph ─────────────────────────────────────

    /// Derive coverage items from SubGoal nodes that reference file paths.
    pub fn seed_coverage_from_graph(&mut self) {
        let graph = match self.graph.as_ref() {
            Some(g) => g,
            None => return,
        };
        for node in graph.nodes.values() {
            if node.kind != NodeKind::SubGoal && node.kind != NodeKind::Instruction {
                continue;
            }
            // Extract file paths from node label/description
            let paths = extract_file_paths_from_text(&node.label);
            let desc_paths = extract_file_paths_from_text(&node.description);
            let mut all_paths: Vec<String> = paths.into_iter().chain(desc_paths).collect();
            all_paths.sort();
            all_paths.dedup();
            if !all_paths.is_empty() {
                self.coverage.register_items(&all_paths, "file");
            }
        }
    }

    /// Derive coverage from ls/glob output that was registered in scope_coverage.
    /// Called after tool execution to sync discovery-only: new items are registered,
    /// but existing item status is NEVER mutated. The runner's coverage tracker is
    /// authoritative — tool_loop's scope_coverage tracks per-call success, not
    /// graph-level completion. Status flows:
    ///   runner.mark_coverage_covered() → runner coverage
    ///   NOT: tool_loop scope_coverage → runner coverage
    pub fn sync_external_coverage(
        &mut self,
        external: &crate::scope_coverage::ScopeCoverageLedger,
    ) {
        for item in &external.items {
            // Only register items not yet tracked — discovery of new paths.
            if !self.coverage.items.iter().any(|i| i.item == item.item) {
                self.coverage.items.push(crate::scope_coverage::CoverageItem {
                    item: item.item.clone(),
                    kind: item.kind.clone(),
                    status: crate::scope_coverage::CoverageStatus::Pending,
                });
            }
            // Status is NOT copied from external. The runner tracks its own
            // coverage state through mark_coverage_covered/failed/skipped.
        }
    }

    /// Mark a graph-derived or discovered coverage item as covered.
    pub fn mark_coverage_covered(&mut self, path: &str) {
        self.coverage.mark_covered(path);
    }

    /// Mark a graph-derived or discovered coverage item as failed.
    pub fn mark_coverage_failed(&mut self, path: &str) {
        self.coverage.mark_failed(path);
    }

    // ── completion and finalization gates ───────────────────────────────

    /// Whether all graph nodes are terminal (Succeeded, Failed, or Skipped).
    pub fn all_graph_nodes_terminal(&self) -> bool {
        match self.graph.as_ref() {
            Some(graph) => {
                if graph.nodes.is_empty() {
                    return true;
                }
                graph.nodes.values().all(|n| !matches!(n.status, NodeStatus::Pending | NodeStatus::InProgress))
            }
            None => true,
        }
    }

    /// Whether all coverage items are terminal.
    /// Vacuously true when no coverage items exist (no scope to cover).
    pub fn all_coverage_terminal(&self) -> bool {
        self.coverage.total() == 0 || self.coverage.all_terminal()
    }

    /// Whether the turn is complete: graph nodes terminal AND coverage terminal.
    pub fn can_finalize(&self) -> bool {
        self.all_graph_nodes_terminal() && self.all_coverage_terminal()
    }

    /// Whether finalization is premature (node remaining or coverage pending).
    pub fn finalization_is_premature(&self) -> bool {
        !self.can_finalize()
    }

    /// Build a context nudge that tells the model to stay focused on the
    /// current node and not finalize early.
    pub fn build_node_focus_context(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        if let Some(ref progress) = self.current_progress {
            parts.push(format!(
                "You are working on: **{}** — {}\nDo NOT finalize until this is complete.",
                progress.node_kind, progress.node_label
            ));
            if progress.consecutive_failures >= 2 {
                parts.push(
                    "The last 2 attempts on this node failed. Consider changing your approach \
                     (different tool, narrower scope) but stay focused on this specific node.".to_string(),
                );
            }
        } else if self.is_graph_driven() {
            parts.push("You have not committed to a graph node yet. Discover what needs to be done.".to_string());
        }

        // Coverage progress
        if self.coverage.total() > 0 {
            parts.push(format!("Progress: {}", self.coverage.render_summary()));
        }

        parts.join("\n\n")
    }

    /// Build a finalization blocker message — used when the model tries to
    /// finalize but the graph/coverage is not terminal.
    pub fn build_relaxed_continuation(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        lines.push("You are not done. Do not finalize yet.".to_string());

        if !self.all_graph_nodes_terminal() {
            if let Some(graph) = self.graph.as_ref() {
                let pending: Vec<&str> = graph
                    .nodes
                    .values()
                    .filter(|n| n.status == NodeStatus::Pending)
                    .map(|n| n.label.as_str())
                    .collect();
                if !pending.is_empty() {
                    lines.push(format!(
                        "Incomplete nodes ({}): {}",
                        pending.len(),
                        pending.iter().take(5).map(|s| format!("`{}`", s)).collect::<Vec<_>>().join(", "),
                    ));
                }
            }
        }

        if !self.all_coverage_terminal() {
            lines.push(format!(
                "Incomplete evidence coverage: {}",
                self.coverage.render_summary()
            ));
        }

        if let Some(ref progress) = self.current_progress {
            lines.push(format!(
                "Continue working on: {} ({} iterations spent, {} tool calls)",
                progress.node_label, progress.iterations_spent, progress.tool_calls_spent
            ));
        }

        lines.push("Continue working. Do NOT provide a final answer until all required work is complete.".to_string());
        lines.join("\n")
    }

    /// Persist the graph and coverage state to the session.
    pub fn persist(&self, session_root: &std::path::Path) {
        self.coverage.persist(session_root);
        if let Some(ref graph) = self.graph {
            let dir = session_root.join("work_graph");
            let _ = std::fs::create_dir_all(&dir);
            if let Ok(json) = serde_json::to_string_pretty(graph) {
                let _ = std::fs::write(dir.join("graph.json"), &json);
            }
        }
    }

    /// Build a summary of graph execution for the turn summary.
    pub fn render_completion_summary(&self) -> String {
        let mut lines: Vec<String> = Vec::new();
        if let Some(ref graph) = self.graph {
            let succeeded = graph.nodes.values().filter(|n| n.status == NodeStatus::Succeeded).count();
            let failed = graph.nodes.values().filter(|n| n.status == NodeStatus::Failed).count();
            let total = graph.nodes.len();
            lines.push(format!("Work graph: {succeeded}/{total} succeeded, {failed} failed"));
        }
        lines.push(format!("Coverage: {}", self.coverage.render_summary()));
        lines.join("; ")
    }
}

/// Extract file-path-like substrings from text (used for seeding coverage).
fn extract_file_paths_from_text(text: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for word in text.split_whitespace() {
        let clean = word.trim_matches(|c: char| c == '`' || c == '"' || c == '\'' || c == ',' || c == '.' || c == ')');
        if clean.contains('/') && clean.contains('.') && clean.len() > 4 && clean.len() < 200 {
            paths.push(clean.to_string());
        }
    }
    paths
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_direct_mode() {
        let runner = WorkGraphRunner::new("DIRECT", "hello");
        assert!(!runner.is_graph_driven());
        assert!(runner.can_finalize());
    }

    #[test]
    fn test_runner_graph_driven_mode() {
        let runner = WorkGraphRunner::new("MULTISTEP", "read all docs");
        assert!(runner.is_graph_driven());
        // Empty graph with no nodes is vacuously terminal — no work items exist.
        assert!(runner.can_finalize());
        // Adding a pending node makes it non-terminal.
        let mut runner = WorkGraphRunner::new("MULTISTEP", "read all docs");
        runner.add_goal("g1", "Read docs", "read all documentation files");
        assert!(!runner.can_finalize());
    }

    #[test]
    fn test_add_goal_and_sub_goal() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "read all docs");
        runner.add_goal("g1", "Discover files", "List docs directory");
        runner.add_sub_goal("sg1", "Read ARCHITECTURAL_RULES.md", "Read the architecture rules", "g1");
        runner.add_sub_goal("sg2", "Read SOUL.md", "Read the soul doc", "g1");

        let (label, kind, _) = runner.advance_to_next_node().unwrap();
        assert_eq!(label, "Discover files");
        assert_eq!(kind, "Goal");

        let graphs = runner.graph.as_ref().unwrap();
        assert_eq!(graphs.nodes.len(), 3);
    }

    #[test]
    fn test_node_lifecycle() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        runner.add_goal("g1", "Goal 1", "desc");
        runner.add_sub_goal("sg1", "SubGoal 1", "read docs/ARCHITECTURAL_RULES.md", "g1");

        // Advance to Goal 1
        let (label, kind, _) = runner.advance_to_next_node().unwrap();
        assert_eq!(label, "Goal 1");

        // Record 3 iterations
        runner.record_iteration();
        runner.record_iteration();
        runner.record_iteration();
        assert!(runner.current_node_effort_sufficient());

        // Mark succeeded
        runner.mark_current_node_succeeded();

        // Next node should be SubGoal 1
        let (label, kind, _) = runner.advance_to_next_node().unwrap();
        assert_eq!(label, "SubGoal 1");
        assert_eq!(kind, "Sub-Goal");
    }

    #[test]
    fn test_coverage_seeded_from_graph() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "read all docs");
        runner.add_sub_goal("sg1", "Read docs/ARCHITECTURAL_RULES.md", "Read architecture rules", "g1");
        runner.add_sub_goal("sg2", "Read docs/SOUL.md", "Read soul doc", "g1");
        runner.add_sub_goal("sg3", "Read docs/SKILL_SYSTEM.md", "Read skill system", "g1");
        runner.seed_coverage_from_graph();

        assert_eq!(runner.coverage.total(), 3);
        assert!(runner.coverage.has_pending());
    }

    #[test]
    fn test_finalization_blocked_by_nodes() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        runner.add_goal("g1", "Goal 1", "desc");
        runner.add_sub_goal("sg1", "SubGoal 1", "read file.md", "g1");
        runner.seed_coverage_from_graph();

        // Nothing committed or done
        assert!(!runner.can_finalize());
        assert!(runner.finalization_is_premature());
    }

    #[test]
    fn test_finalization_allowed_when_done() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        runner.add_goal("g1", "Goal 1", "desc");
        runner.add_sub_goal("sg1", "SubGoal 1", "read docs/ARCHITECTURAL_RULES.md", "g1");
        runner.seed_coverage_from_graph();

        // Complete all nodes
        runner.advance_to_next_node(); // Goal 1
        runner.mark_current_node_succeeded();
        runner.advance_to_next_node(); // SubGoal 1
        // Mark the coverage item that was seeded from graph
        runner.mark_coverage_covered("docs/ARCHITECTURAL_RULES.md");
        runner.mark_current_node_succeeded();

        assert!(runner.can_finalize());
        assert!(!runner.finalization_is_premature());
    }

    #[test]
    fn test_current_node_is_stuck_detection() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        runner.add_goal("g1", "Goal 1", "desc");
        runner.advance_to_next_node();
        runner.record_tool_call(false);
        runner.record_tool_call(false);
        runner.record_tool_call(false);
        assert!(runner.current_node_is_stuck());
    }

    #[test]
    fn test_current_node_not_stuck_after_success() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        runner.add_goal("g1", "Goal 1", "desc");
        runner.advance_to_next_node();
        runner.record_tool_call(false);
        runner.record_tool_call(false);
        runner.record_tool_call(true); // success resets
        assert!(!runner.current_node_is_stuck());
    }

    #[test]
    fn test_build_relaxed_continuation() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "read docs");
        runner.add_goal("g1", "Discover docs", "desc");
        runner.add_sub_goal("sg1", "Read ARCHITECTURAL_RULES.md", "read docs/ARCHITECTURAL_RULES.md", "g1");
        runner.seed_coverage_from_graph();
        runner.advance_to_next_node(); // commit to Goal 1

        let msg = runner.build_relaxed_continuation();
        assert!(msg.contains("Do not finalize"));
        assert!(msg.contains("Incomplete nodes"));
        assert!(msg.contains("Continue working"));
    }

    #[test]
    fn test_build_node_focus_context() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        runner.add_goal("g1", "Read files", "read all the docs");
        runner.advance_to_next_node();

        let ctx = runner.build_node_focus_context();
        assert!(ctx.contains("Read files"));
        assert!(ctx.contains("Do NOT finalize"));
    }

    #[test]
    fn test_sync_external_coverage() {
        let mut runner = WorkGraphRunner::new("MULTISTEP", "test");
        let mut external = crate::scope_coverage::ScopeCoverageLedger::new();
        external.register_items(&["a.md".to_string(), "b.md".to_string()], "file");
        external.mark_covered("a.md");

        runner.sync_external_coverage(&external);
        assert_eq!(runner.coverage.total(), 2);
        assert!(runner.coverage.has_pending()); // b.md still pending
    }

    #[test]
    fn test_graph_runner_empty_graph_is_terminal() {
        let runner = WorkGraphRunner::new("MULTISTEP", "empty");
        // No nodes added means graph is vacuously terminal
        assert!(runner.all_graph_nodes_terminal());
    }
}
