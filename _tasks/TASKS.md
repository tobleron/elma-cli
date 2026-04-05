# Task Management - Follow Instructions in Exact Order

## Task Creation Rule

### Main Project Tasks (Numbered 0XX)
- **Mandatory Prefix**: Every new task MUST have a sequential number prefix (e.g., `096_task_name.md`).
- **Sequence Basis**: The sequence number must be the next available number based on the highest existing number across `_tasks/completed/`, `_tasks/pending/`, `_tasks/postponed/`, and `_tasks/active/` folders.
- **Format**: Use three-digit padding (e.g., `001`, `012`, `095`).
- **Detail Requirement**: Every task MUST be self-documenting. Provide enough technical detail, context, and clear objective so that a rename (e.g., `_DONE`) is sufficient to signify completion.

### Dev-System Tasks (Prefixed with D: D001, D002, etc.) ⚙️
- **Location**: All auto-generated tasks reside in `_dev-tasks/`.
- **Source**: Created by `_dev-system/analyzer`.
- **Nature**: Advisory/Architectural. They guide de-bloating (e.g., splitting `src/main.rs`) and structural hygiene.

### Troubleshooting Tasks (Prefixed with T: T001, T002, etc.) 🛠️
- **Trigger**: Created manually when starting a "Phase 0" troubleshooting session.
- **Numbering**: Follows the **Main Project Task** sequence but uses the `T` prefix (e.g., `T042_Fix_Scenario_Parsing.md`).

## Current Master Plan
**Task 095**: Incremental Upgrade Master Plan — 4-phase disciplined rollout with verification gates.
- **Phase 1**: Clean up & stabilize foundation (tasks 1-4)
- **Phase 2**: Reliability core (tasks 5-8)
- **Phase 3**: Efficiency & observability (tasks 9-12)
- **Phase 4**: Advanced capabilities (tasks 13-15)
- **Tier C**: 14 tasks formally postponed until all phases complete

## Workflow Instructions (Must be followed sequentially)

1. **Pickup**: Move the intended task file from `_tasks/pending/` to `_tasks/active/`.
2. **Implement**: Perform surgical edits. Avoid unrelated refactoring.
3. **Verify (Build)**: Run `cargo build`. Ensure zero warnings.
4. **Verify (Behavior)**:
    - Run unit tests: `cargo test`.
    - Run scenario probes: `./run_intention_scenarios.sh` or `./reliability_probe.sh`.
5. **Sign-Off**: Present results to the user while the task is still in `_tasks/active/`.
6. **Archive**: Once approved, move to `_tasks/completed/` and append `_DONE`.

## Folder Structure
- `_tasks/pending/`: Main project tasks waiting to be started (Tier A + Tier B only).
- `_tasks/active/`: The master plan (095) + current sub-tasks being worked on.
- `_tasks/completed/`: Finished tasks.
- `_tasks/postponed/`: Tier C tasks deferred until all 4 phases complete.
- `_dev-tasks/`: Auto-generated architectural guidance.
