# Task 014: Multi-Turn Goal Persistence

## Status
PENDING

## Problem
Elma doesn't maintain goals across turns. Each message is treated independently, losing context for multi-step tasks.

## Evidence
For multi-step tasks like "summarize AGENTS.md and create AGENTS_para.md":
- No tracking of which subgoals are completed
- No persistence of objective across conversation turns
- Each turn starts fresh

## Goal
Add goal state tracking that persists across conversation turns.

## Implementation Steps

1. **Create goal state structure** in `src/types.rs`:
   ```rust
   #[derive(Debug, Clone, Default)]
   pub struct GoalState {
       pub active_objective: Option<String>,
       pub completed_subgoals: Vec<String>,
       pub pending_subgoals: Vec<String>,
       pub blocked_reason: Option<String>,
       pub created_at: u64,
       pub last_updated: u64,
   }
   ```

2. **Add to AppRuntime** in `src/app.rs`:
   ```rust
   pub struct AppRuntime {
       // ... existing fields
       pub goal_state: GoalState,
   }
   ```

3. **Update chat loop** to maintain goal state:
   ```rust
   // In app_chat.rs
   if goal_state.active_objective.is_none() {
       // New task - extract objective from user input
       goal_state.active_objective = Some(extract_objective(&line));
   }
   
   // After execution
   goal_state.completed_subgoals.extend(newly_completed);
   goal_state.pending_subgoals = remaining_pending;
   
   if goal_state.pending_subgoals.is_empty() {
       goal_state.active_objective = None; // Task complete
   }
   ```

4. **Add goal state to orchestrator context**:
   ```rust
   let program = build_program(
       ...,
       Some(&runtime.goal_state),  // Pass goal context
   ).await;
   ```

5. **Add commands for goal management**:
   - `/goals` - Show current goal state
   - `/reset-goals` - Clear goal state
   - `/continue` - Continue with pending goals

6. **Persist goal state to session**:
   - Save to `sessions/{id}/goal_state.json`
   - Load on session resume

## Acceptance Criteria
- [ ] Goal state persists across conversation turns
- [ ] Completed/pending subgoals are tracked
- [ ] `/goals` command shows current state
- [ ] Goal state is saved to session files
- [ ] Orchestrator uses goal context in program generation

## Files to Modify
- `src/types.rs` - Add GoalState struct
- `src/app.rs` - Add goal_state to AppRuntime
- `src/app_chat.rs` - Update chat loop to maintain goals
- `src/orchestration.rs` - Use goal context
- `src/session.rs` - Add goal state persistence

## Priority
HIGH - Enables complex multi-turn tasks

## Dependencies
- None blocking
