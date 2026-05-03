# 562 — Create Bounded Finite State Machine for Agent Lifecycle

- **Priority**: High
- **Category**: State Management
- **Depends on**: 554 (session-scoped state), 556 (replace keyword matchers)
- **Blocks**: 575

## Problem Statement

The agent's execution lifecycle is managed implicitly through control flow in `tool_loop.rs`, `app_chat_orchestrator.rs`, and `orchestration_core.rs`. There is no explicit representation of what state the agent is in at any point. State transitions happen through deeply nested function calls, async task spawning, and global mutable state.

Current implicit states (inferred from control flow):
- **Idle**: Waiting for user input
- **Classifying**: Running intent analysis / classification
- **Assessing Complexity**: Running execution ladder assessment
- **Planning**: Building work graph or program
- **Executing**: Running tool loop
- **Finalizing**: Generating final answer
- **Error**: Handling failures, retrying, or falling back
- **Compacting**: Running auto-compaction
- **Stopped**: Hit budget/timeout/stagnation limit
- **Interrupted**: User cancelled

These states exist only in code flow, making it impossible to:
- Resume from any intermediate state
- Serialize/deserialize agent progress
- Test state transitions in isolation
- Add new states without touching multiple files
- Reason about what transitions are valid

## Why This Matters for Small Local LLMs

Small models cause more state transitions (more errors → more retries → more compaction → more stops). Without explicit state tracking:
- A compaction that happens during finalization can lose evidence
- A retry after classification failure may skip reassessment
- A user interrupt during shell execution may leave workspace in inconsistent state

## Current Behavior

State is tracked implicitly through:
- `StopPolicy` fields (iteration count, stagnation runs, etc.)
- `messages` vector (conversation history)
- Global `SESSION_LEDGER`, `CURRENT_TURN_ID`, etc.
- `AppRuntime` fields (last_stop_outcome, execution_plan, goal_state)
- Control flow position (which function is executing)

## Recommended Target Behavior

Define an explicit `AgentState` enum and `AgentStateMachine`:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Classifying,
    AssessingComplexity,
    BuildingWorkGraph,
    ExecutingToolLoop { iteration: usize },
    Finalizing { reason: FinalizationReason },
    Compacting { trigger: CompactTrigger },
    Error { error: AgentError, recovery: RecoveryStrategy },
    Stopped { reason: StopReason },
    Interrupted,
    Completed,
}

pub struct AgentStateMachine {
    state: AgentState,
    history: Vec<AgentState>,  // for debugging
    transitions: HashMap<(AgentState, AgentState), bool>,  // allowed transitions
}

impl AgentStateMachine {
    pub fn transition(&mut self, to: AgentState) -> Result<(), InvalidTransition> {
        if self.is_valid_transition(&self.state, &to) {
            self.history.push(self.state.clone());
            self.state = to;
            Ok(())
        } else {
            Err(InvalidTransition { from: self.state.clone(), to })
        }
    }
}
```

### Valid Transitions (example subset)
```
Idle → Classifying
Classifying → AssessingComplexity | Error
AssessingComplexity → BuildingWorkGraph | ExecutingToolLoop | Completed
ExecutingToolLoop → Finalizing | Compacting | Stopped | Interrupted | Error
Finalizing → Completed | Error
Compacting → ExecutingToolLoop | Finalizing
Error → Idle | ExecutingToolLoop | Finalizing (depending on recovery strategy)
Stopped → Finalizing | Idle
```

## Source Files That Need Modification

- `src/tool_loop.rs` — Integrate FSM, emit state transitions
- `src/app_chat_orchestrator.rs` — Integrate FSM for classification/assessment/planning
- `src/stop_policy.rs` — Simplify; many checks become FSM transitions
- `src/orchestration_core.rs` — Integrate FSM state checks
- `src/final_answer.rs` — Integrate Finalizing state
- `src/auto_compact.rs` — Integrate Compacting state

## New Files/Modules

- `src/agent_fsm.rs` — `AgentState`, `AgentStateMachine`, `InvalidTransition`
- `src/agent_fsm_transitions.rs` — Transition validation logic

## Step-by-Step Implementation Plan

1. Define `AgentState` enum with all known states
2. Define `AgentStateMachine` with transition validation
3. Define valid transition map (bidirectional whitelist)
4. Add `state_machine: AgentStateMachine` to `SessionState` (Task 554)
5. Instrument `tool_loop.rs` to emit state transitions:
   ```rust
   state_machine.transition(AgentState::ExecutingToolLoop { iteration: i })?;
   ```
6. Instrument classification/assessment pipeline
7. Validate that every state exit point transitions to a valid next state
8. Add transcript-row emission for state transitions (per AGENTS.md Rule 6)
9. Add serialization for state machine (for session resume)
10. Run full test suite

## Recommended Crates

- `strum` — already a dependency; use `EnumIter` for exhaustive transition validation
- `serde` — for state serialization (session resume)

## Validation/Sanitization Strategy

- All transitions are validated at runtime (debug mode) or compile time (with `#[cfg(debug_assertions)]`)
- Invalid transitions panic in debug, log error in release
- Transition history is bounded (last 100 states) to prevent memory leaks
- State machine is `Send + Sync` for async compatibility

## Testing Plan

1. Test all valid transitions succeed
2. Test all known invalid transitions fail
3. Test that the FSM correctly models a full agent lifecycle (Idle → Completed)
4. Test error recovery transitions
5. Test compaction during execution
6. Test user interrupt during any state
7. Test that state serialization/deserialization roundtrips
8. Property test: no sequence of valid transitions leads to a dead-end (all states have exit transitions except Completed)

## Acceptance Criteria

- All 10+ agent states are explicitly modeled
- Every state transition in the codebase goes through `AgentStateMachine::transition()`
- Invalid transitions are caught (panic in debug, error in release)
- State transitions appear as transcript rows (per Rule 6)
- State machine serializes for session resume
- FSM replaces implicit state tracking in StopPolicy and tool_loop

## Risks and Migration Notes

- **Very invasive change**: Touches almost every execution path. Do this incrementally — add FSM alongside existing control flow, then gradually remove implicit tracking.
- **Performance**: Transition validation adds a HashMap lookup per state change. Negligible overhead (~10ns per transition).
- **Session resume**: Serializing mid-execution state requires careful handling of async state (in-flight futures, open streams). Start with simple states (Idle, Completed, Stopped) and add intermediate states later.
- Build on Task 554 (session-scoped state) and Task 556 (keyword matchers cleanup) for clean integration points.
