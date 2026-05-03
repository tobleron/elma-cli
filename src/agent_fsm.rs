//! @efficiency-role: data-model
//! Bounded Finite State Machine for agent lifecycle.
//!
//! All valid state transitions are explicitly defined. Every state change
//! must pass through `AgentStateMachine::transition()`.

use serde::{Deserialize, Serialize};

/// All possible agent states during a session lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    Idle,
    Classifying,
    AssessingComplexity,
    BuildingWorkGraph,
    ExecutingToolLoop { iteration: usize },
    Finalizing,
    Compacting,
    Error { reason: String },
    Stopped { reason: String },
    Interrupted,
    Completed,
}

impl AgentState {
    pub fn label(&self) -> &str {
        match self {
            AgentState::Idle => "idle",
            AgentState::Classifying => "classifying",
            AgentState::AssessingComplexity => "assessing",
            AgentState::BuildingWorkGraph => "planning",
            AgentState::ExecutingToolLoop { .. } => "executing",
            AgentState::Finalizing => "finalizing",
            AgentState::Compacting => "compacting",
            AgentState::Error { .. } => "error",
            AgentState::Stopped { .. } => "stopped",
            AgentState::Interrupted => "interrupted",
            AgentState::Completed => "completed",
        }
    }
}

/// Invalid transition error.
#[derive(Debug, Clone)]
pub struct InvalidTransition {
    pub from: AgentState,
    pub to: AgentState,
}

impl std::fmt::Display for InvalidTransition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid transition: {:?} -> {:?}", self.from, self.to)
    }
}

/// Agent state machine with enforced transition rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStateMachine {
    state: AgentState,
    history: Vec<AgentState>,
    max_history: usize,
}

impl AgentStateMachine {
    pub fn new() -> Self {
        Self {
            state: AgentState::Idle,
            history: Vec::new(),
            max_history: 100,
        }
    }

    pub fn current(&self) -> &AgentState {
        &self.state
    }

    pub fn transition(&mut self, to: AgentState) -> Result<(), InvalidTransition> {
        if !self.is_valid_transition(&self.state, &to) {
            return Err(InvalidTransition {
                from: self.state.clone(),
                to,
            });
        }
        if self.history.len() >= self.max_history {
            self.history.remove(0);
        }
        self.history.push(self.state.clone());
        self.state = to;
        Ok(())
    }

    pub fn history(&self) -> &[AgentState] {
        &self.history
    }

    fn is_valid_transition(&self, from: &AgentState, to: &AgentState) -> bool {
        match (from, to) {
            // From Idle
            (AgentState::Idle, AgentState::Classifying)
            | (AgentState::Idle, AgentState::Completed) => true,

            // From Classifying
            (AgentState::Classifying, AgentState::AssessingComplexity)
            | (AgentState::Classifying, AgentState::Error { .. })
            | (AgentState::Classifying, AgentState::Completed) => true,

            // From Assessing
            (AgentState::AssessingComplexity, AgentState::BuildingWorkGraph)
            | (AgentState::AssessingComplexity, AgentState::ExecutingToolLoop { .. })
            | (AgentState::AssessingComplexity, AgentState::Completed)
            | (AgentState::AssessingComplexity, AgentState::Error { .. }) => true,

            // From BuildingWorkGraph / ExecutingToolLoop → can go to each other or finalize/compact
            (AgentState::BuildingWorkGraph, AgentState::ExecutingToolLoop { .. })
            | (AgentState::BuildingWorkGraph, AgentState::Compacting)
            | (AgentState::BuildingWorkGraph, AgentState::Error { .. })
            | (AgentState::ExecutingToolLoop { .. }, AgentState::ExecutingToolLoop { .. })
            | (AgentState::ExecutingToolLoop { .. }, AgentState::Finalizing)
            | (AgentState::ExecutingToolLoop { .. }, AgentState::Compacting)
            | (AgentState::ExecutingToolLoop { .. }, AgentState::Stopped { .. })
            | (AgentState::ExecutingToolLoop { .. }, AgentState::Interrupted)
            | (AgentState::ExecutingToolLoop { .. }, AgentState::Error { .. }) => true,

            // From Finalizing
            (AgentState::Finalizing, AgentState::Completed)
            | (AgentState::Finalizing, AgentState::Error { .. }) => true,

            // From Compacting
            (AgentState::Compacting, AgentState::ExecutingToolLoop { .. })
            | (AgentState::Compacting, AgentState::Finalizing)
            | (AgentState::Compacting, AgentState::Error { .. }) => true,

            // From Error — can recover to Idle or retreat to Stopped
            (AgentState::Error { .. }, AgentState::Idle)
            | (AgentState::Error { .. }, AgentState::Stopped { .. })
            | (AgentState::Error { .. }, AgentState::Completed) => true,

            // From Stopped — can go to Idle (retry) or Completed (done)
            (AgentState::Stopped { .. }, AgentState::Idle)
            | (AgentState::Stopped { .. }, AgentState::Completed) => true,

            // From Interrupted — can go to Idle (resume) or Completed
            (AgentState::Interrupted, AgentState::Idle)
            | (AgentState::Interrupted, AgentState::Completed) => true,

            // Completed is terminal
            _ => false,
        }
    }
}

impl Default for AgentStateMachine {
    fn default() -> Self {
        Self::new()
    }
}
