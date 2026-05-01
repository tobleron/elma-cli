# Task Management

## Task Creation

### Main Project Tasks (Numbered Prefixes)
- Every new task uses the next available numeric prefix across `_tasks/active/`, `_tasks/pending/`, `_tasks/completed/`, and `_tasks/postponed/`.
- Use three-digit padding (e.g., `301`, not `31`).
- Task files must be self-documenting enough that status can be inferred from filename and header.

### Troubleshooting Tasks (T###)
- Use the same numeric sequence with a `T` prefix.
- Create immediately when a real regression or failure class is being investigated.
- Example: `T285` for a regression discovered during Task 285 implementation.

### Dev-System Tasks (D###)
- Stored in `_dev-tasks/`.
- Advisory only — structural guidance for the development tooling.

## Task Lifecycle

```
pending/  →  active/  →  completed/
```

The normal procedure for handling any task:

### Step 1: Start Work (pending → active)
When you begin working on a task:
1. Move the task file from `_tasks/pending/` to `_tasks/active/`
2. Update the task status in the file header if applicable
3. Begin implementation following the task's plan

**Never work on a task while it remains in `pending/`. Always move it to `active/` first.**

### Step 2: Implement
Make the change surgically. Follow the task's implementation plan. Do not expand scope without updating the task file.

### Step 3: Verify
```bash
cargo build
cargo test
```
Verify behavior with the relevant probes and scenario scripts. Ensure all checks pass before proceeding.

### Step 4: Complete Work (active → completed)
When implementation is finished and verified:
1. Ensure `cargo build` and `cargo test` pass
2. Rename the task file with `_DONE` suffix (e.g., `301_My_Task_DONE.md`)
3. Move the file from `_tasks/active/` to `_tasks/completed/`
4. Report completion with a summary of what was done

**Never leave a finished task in `active/`. Always move it to `completed/` when done.**

## Handling Duplicate Work

If new work touches an existing active or pending task:
- Update that task instead of creating a duplicate
- If the scope has changed, update the task file's objective and plan
- Do not create a second task for work already covered

## Troubleshooting During Implementation

If a real bug or regression is discovered during implementation:
1. Create a `T###` troubleshooting task immediately
2. Document the bug with reproduction steps
3. Fix the bug in the same or a follow-up implementation task
4. Reference the `T###` task in the implementation task

## Folder Meanings

| Directory | Purpose |
|-----------|---------|
| `_tasks/active/` | Currently being implemented |
| `_tasks/pending/` | Next approved work |
| `_tasks/completed/` | Finished and archived |
| `_tasks/postponed/` | Deferred, absorbed, or superseded work kept for history |
| `_dev-tasks/` | Analyzer and development tooling guidance |

## Current Master Plan

See [`_masterplan.md`](_masterplan.md) for the current task roadmap and sequencing.
