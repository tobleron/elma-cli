# Task 068: Multi-Turn Goal Persistence

## Status
Absorbed by Task 195. Historical reference only.

## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Priority
**P1 - RELIABILITY CORE (Tier A)**
**Was Blocked by:** P0-1, P0-2, P0-3 — **NOW UNBLOCKED** (P0 pillars substantially complete per Task 058)

## Status
**PENDING** — Ready to start

## Renumbering Note
- **Old:** Task 014
- **New:** Task 013 (per REPRIORITIZED_ROADMAP.md)

---

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
