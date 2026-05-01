//! @efficiency-role: domain-logic
//!
//! Decomposition Pyramid — objective → goals → tasks → next action.
//!
//! For complex requests the pyramid keeps small local models on-track by
//! narrowing each decision: what is the ONE objective, what are the bounded
//! sub-goals, what tasks serve each goal, and what is the next atomic action.

use serde::Serialize;

/// A goal within the pyramid: a bounded sub-objective.
#[derive(Debug, Clone)]
pub(crate) struct PyramidGoal {
    pub text: String,
    pub evidence_needed: bool,
}

/// A task within the pyramid: a concrete unit of work under a goal.
#[derive(Debug, Clone)]
pub(crate) struct PyramidTask {
    pub id: u32,
    pub text: String,
    pub status: String, // ready | active | completed | blocked
}

/// A single next-action selection from the pyramid.
#[derive(Debug, Clone, Serialize)]
pub(crate) struct NextAction {
    pub task_id: u32,
    pub action: String, // read | list | search | shell | edit | ask | done
    pub reason: String,
}

/// The full decomposition pyramid produced for a complex request.
#[derive(Debug, Clone)]
pub(crate) struct DecompositionPyramid {
    pub objective: String,
    pub risk: String, // low | medium | high
    pub goals: Vec<PyramidGoal>,
    pub tasks: Vec<PyramidTask>,
    pub next_action: Option<NextAction>,
}

impl DecompositionPyramid {
    /// Render a compact context snippet for the tool-loop system prompt.
    pub(crate) fn render_context(&self) -> String {
        let mut out = format!("OBJECTIVE: {}\nRISK: {}\n", self.objective, self.risk);

        if let Some(ref na) = self.next_action {
            out.push_str(&format!(
                "NEXT: task_id={} action={} — {}\n",
                na.task_id, na.action, na.reason
            ));
        }

        // Show active/ready tasks
        let active: Vec<&PyramidTask> = self
            .tasks
            .iter()
            .filter(|t| t.status == "ready" || t.status == "active")
            .collect();
        if !active.is_empty() {
            out.push_str("PENDING TASKS:\n");
            for t in &active {
                out.push_str(&format!("  task_id={} — {}\n", t.id, t.text));
            }
        }

        out
    }

    /// Render a compact repair-hint string listing available tasks.
    pub(crate) fn render_task_menu(&self) -> String {
        let mut out = String::from("AVAILABLE TASKS:\n");
        for t in &self.tasks {
            let marker = if t.status == "ready" || t.status == "active" {
                "→"
            } else {
                "·"
            };
            out.push_str(&format!(
                "  {} task_id={} status={} — {}\n",
                marker, t.id, t.status, t.text
            ));
        }
        let actions = "read | list | search | shell | edit | ask | done";
        out.push_str(&format!(
            "Reply with NEXT task_id=<id> action=<{}> reason=\"...\"\n",
            actions
        ));
        out
    }
}

impl Default for DecompositionPyramid {
    fn default() -> Self {
        Self {
            objective: String::new(),
            risk: String::from("low"),
            goals: Vec::new(),
            tasks: Vec::new(),
            next_action: None,
        }
    }
}
