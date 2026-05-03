# Task 494: Full Hierarchy Integration, Task Persistence, Workflow Intel Unit

**Status:** pending
**Priority:** HIGHEST
**Estimated effort:** 5-7 days
**Dependencies:** Task 389 (pyramid types), Task 390 (approach engine), Task 380 (semantic continuity)

## Summary

Wire the full decision cascade (Complexity → Route → Formula → Pyramid Graph → Instruction → Step), integrate approaches as sibling branches at the Graph root, persist task state under both `sessions/<id>/runtime_tasks/` and `_elma-tasks/`, and create an intel unit that auto-generates or user-triggers task files.

## The Full Hierarchy

```
                         COMPLEXITY ASSESSMENT (DIRECT/INVESTIGATE/MULTISTEP/OPEN_ENDED)
                              │ determines max graph depth
                              ▼
                         ROUTE DECISION (CHAT/SHELL/PLAN/MASTERPLAN/DECIDE)
                              │
                              ▼
                         FORMULA SELECTION (reply_only/inspect_reply/plan_reply/...)
                              │
          ┌───────────────────┼───────────────────┐
          ▼                   ▼                   ▼
     APPROACH A          APPROACH B          APPROACH C
     (primary)           (fork on fail)      (fork on fail)
          │                   │                   │
     ┌────┴────┐         ┌────┴────┐         ┌────┴────┐
     │  GOAL   │         │  GOAL   │         │  GOAL   │
     ├─────────┤         ├─────────┤         ├─────────┤
     │ SUBGOAL │         │ SUBGOAL │         │ SUBGOAL │
     ├─────────┤         ├─────────┤         ├─────────┤
     │  PLAN   │         │  PLAN   │         │  PLAN   │
     ├─────────┤         ├─────────┤         ├─────────┤
     │INSTRUCT.│         │INSTRUCT.│         │INSTRUCT.│
     │  [task] │         │  [task] │         │  [task] │
     │  [task] │         │  [task] │         │  [task] │
     └────┬────┘         └────┬────┘         └────┬────┘
          ▼                   ▼                   ▼
     PROGRAM STEP        PROGRAM STEP        PROGRAM STEP
     (shell/read/        (shell/read/        (shell/read/
      edit/reply/...)     edit/reply/...)     edit/reply/...)
```

### Complexity → Depth Mapping

| Complexity | Max Depth | Graph Layers Used |
|------------|-----------|-------------------|
| DIRECT | 1 | Instruction only |
| INVESTIGATE | 2 | Goal → Instruction |
| MULTISTEP | 3 | Goal → SubGoal → Plan → Instruction |
| OPEN_ENDED | 4+ | Goal → SubGoal → Plan → Instruction (parallel approaches) |

## Implementation Plan

### Phase 1: Complexity-Gated Graph Builder ✅

1. Wire `ComplexityAssessment` result into `WorkGraphBuilder` to cap max depth.
2. Add `WorkGraphBuilder::from_complexity(complexity, objective)` constructor.
3. When complexity = `DIRECT`, skip the graph entirely → go straight to Instruction → Step.
4. When complexity = `INVESTIGATE/MULTISTEP/OPEN_ENDED`, build graph to allowed depth.
5. Emit graph creation as visible transcript event ("GRAPH: created with depth 3 for MULTISTEP").

### Phase 2: Approaches as Sibling Branches ✅

1. Each `ApproachId` is a sibling root under the Objective.
2. `ApproachEngine` already exists and is wired in `orchestration_retry.rs:584`.
3. Add `ApproachEngine::fork_new_approach(reason)` to create sibling branches explicitly.
4. On `PruneAndRetry`, mark current approach `Pruned`, create new sibling.
5. On `Exhausted`, mark all approaches `Failed`, propagate to session.

### Phase 3: Task Persistence Under Sessions ✅

1. Extend `runtime_task.rs` to store tasks in `sessions/<id>/runtime_tasks/tasks.json`:
   ```json
   {
     "tasks": [
       {
         "id": "1",
         "instruction_id": "instr_001",
         "approach_id": "a_123",
         "description": "Read Cargo.toml",
         "status": "completed",
         "step_type": "read",
         "step_result": "success",
         "created_at": 1714665600,
         "completed_at": 1714665605
       }
     ]
   }
   ```
2. Update on each StepResult: `runtime_task::record_step_completion(task_id, result)`.
3. Load tasks on session resume: `runtime_task::load_pending_tasks(session_root)`.
4. Keep UI `TaskList` in-sync with persisted tasks via `set_task_list`.

### Phase 4: Two Task Types in `_elma-tasks/` ✅

#### Type A: Auto-Generated (Workflow-Driven)

- Created by intel unit during workflow execution.
- Files named: `NNN_Task_Name.md` where NNN = auto-incremented sequence.
- Generated when an Instruction node resolves to a Step.
- Contains: task title, description, status, linked session, graph node reference.

```markdown
# Task 001: Read Cargo.toml
- **Status:** completed
- **Session:** sess_2026-05-02_abc123
- **Approach:** a_1714665600_0
- **Graph node:** instr_001 (Instruction, depth 3)
- **Created:** 2026-05-02 11:42:00 UTC

Read the project's Cargo.toml to understand dependencies and configuration.
```

#### Type B: User-Initiated

- Triggered by direct user request ("add a task for X").
- Same intel unit handles creation, same NNN naming.
- Marked with `source: user` vs `source: workflow`.
- Otherwise identical format.

### Phase 5: Task Intel Unit ✅

Create `intel_units/intel_units_task_management.rs`:

1. **Role:** Task creation, naming, content generation.
2. **Input:** Instruction node + Step context (or user request for Type B).
3. **Output:**
   - Valid task filename (`NNN_Task_Name.md`)
   - Task content (title, description, status, session ref, graph ref)
   - Writes to `_elma-tasks/` directory
4. **Naming rules:**
   - NNN = auto-incremented from existing `_elma-tasks/` files
   - Task_Name = human-readable slug from Instruction label

### Phase 6: Instruction → Task → Step Wiring ✅

```
InstructionNode { label: "Read Cargo.toml" }
    │
    ├─→ TaskItem { id:1, description: "Read Cargo.toml", status: InProgress }
    │       └─→ sessions/<id>/runtime_tasks/tasks.json (persisted)
    │
    ├─→ Step::Read { path: "Cargo.toml" }
    │       └─→ StepResult::Success / Failure
    │
    └─→ TaskItem status updates from StepResult
            └─→ sessions/<id>/runtime_tasks/tasks.json (updated)
            └─→ _elma-tasks/001_Read_Cargo.toml.md (Type A, auto-generated)
```

## Files To Create

| File | Purpose |
|------|---------|
| `src/work_graph_bridge.rs` | Complexity → graph depth wiring |
| `src/intel_units/intel_units_task_management.rs` | Task creation intel unit |
| `src/task_persistence.rs` | Read/write `_elma-tasks/` and session task state |
| `_elma-tasks/` | Directory (created on first use) |

## Files To Modify

| File | Change |
|------|--------|
| `src/approach_engine.rs` | Add `fork_new_approach()`, wire to graph depth |
| `src/work_graph.rs` | Add `from_complexity()` constructor |
| `src/orchestration_core.rs` | Pass complexity to graph builder |
| `src/orchestration_retry.rs` | Wire approach engine decisions to task state |
| `src/runtime_task.rs` | Add per-step task persistence |
| `src/claude_ui/claude_tasks.rs` | Link UI tasks to persisted tasks |
| `src/execution_steps.rs` | Update task status on StepResult |
| `src/main.rs` | Register new modules |

## Verification

```bash
cargo test work_graph
cargo test approach_engine
cargo test runtime_task
cargo test task_persistence
cargo build
```

Manual probes:
1. Simple request → no graph, no tasks persisted.
2. Complex request → graph built, tasks persisted under `sessions/<id>/`.
3. Session resume → tasks reload from `sessions/<id>/runtime_tasks/tasks.json`.
4. Auto-gen task files appear in `_elma-tasks/001_Task_Name.md`.
5. User-requested task → same format, `source: user`.

## Success Criteria

- [ ] Complexity assessment gates graph depth.
- [ ] Approaches work as sibling graph branches.
- [ ] Tasks are persisted per-session under `sessions/<id>/runtime_tasks/`.
- [ ] `_elma-tasks/NNN_Task_Name.md` files auto-generate for each Instruction node.
- [ ] User-initiated tasks use same intel unit with different trigger.
- [ ] Task state survives session close and resume.
- [ ] Instruction → Step wiring updates task status automatically.
- [ ] No regression: existing tests pass.

## Anti-Patterns To Avoid

- Do not bloat `prompt_core.rs` with task-related prompts.
- Do not use keyword triggers for task creation routing.
- Do not store task state only in memory.
- Do not change canonical system prompt without explicit approval.
