# Task Management - Follow Instructions in Exact Order

## Task Creation Rule

### Main Project Tasks (Numbered 1XX, 2XX, etc.)
- **Mandatory Prefix**: Every new task MUST have a sequential number prefix (e.g., `189_task_name.md`).
- **Sequence Basis**: The sequence number must be the next available number based on the highest existing number across `_tasks/completed/`, `_tasks/pending/`, `_tasks/postponed/`, and `_tasks/active/` folders.
- **Format**: Use three-digit padding where possible (e.g., `001`, `012`, `123`).
- **Detail Requirement**: Every task MUST be self-documenting. Provide enough technical detail, context, and clear objective so that a rename (e.g., `_DONE`) is sufficient to signify completion.

### Dev-System Tasks (Prefixed with D: D001, D002, etc.) ⚙️
- **Location**: All auto-generated tasks reside in `_dev-tasks/`.
- **Source**: Created by `_dev-system/analyzer`.
- **Nature**: Advisory/Architectural. They guide de-bloating (e.g., splitting `src/main.rs`) and structural hygiene.

### Troubleshooting Tasks (Prefixed with T: T001, T002, etc.) 🛠️
- **Trigger**: Created manually when starting a "Phase 0" troubleshooting session.
- **Numbering**: Follows the **Main Project Task** sequence but uses the `T` prefix (e.g., `T042_Fix_Scenario_Parsing.md`).

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
- `_tasks/pending/`: Main project tasks waiting to be started.
- `_tasks/active/`: The single main project task currently being worked on.
- `_tasks/completed/`: Finished tasks.
- `_tasks/postponed/`: Deferred tasks.
- `_dev-tasks/`: Auto-generated architectural guidance.
