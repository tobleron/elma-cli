# Task 201: Task Inventory Normalization And Reprioritization

## Priority
P0

## Objective
Make the task inventory, runtime scaffolding, and repository instructions match reality after the task-first rebaseline.

## Why This Exists
A future session cannot safely work from the task system if high-level instructions, active plans, and implementation state contradict each other. This task keeps the planning substrate trustworthy.

## Required Behavior
- Ensure `_tasks/TASKS.md`, active master plans, and supporting task files point at the same product direction.
- Remove or explicitly mark stale instructions that still imply single-skill routing or strict 166-first product direction where that is no longer true.
- Keep historical references, but make their status obvious.

## Required Cleanup Rules
- fix active/pending/completed/postponed drift
- mark superseded master plans clearly
- move unrelated open work out of the active path
- delete only stale duplicates or pending items already completed elsewhere
- preserve useful historical references when they still contain verification guidance

## Specific Repo Risk To Resolve
- Audit `AGENTS.md`, `_tasks/TASKS.md`, and active task files for any reintroduced wording that makes 166 look canonical again. Historical 166 material should remain a subordinate UI reference only.

## Acceptance Criteria
- The task directories reflect actual status.
- The next engineer can tell what is active, what is postponed, and what is superseded without cross-checking old sessions.
- High-level repo instructions do not contradict the current master plan.
- Historical files remain available where they still provide useful implementation detail.

## Required Tests / Checks
- manual audit of `_tasks/TASKS.md`
- manual audit of active master plan links
- grep check for stale wording like "exactly one skill" in active planning docs


## Completion Note
Completed during Task 204 verification on 2026-04-23.
Verified with the relevant automated checks available in this repo, including `cargo build`, targeted tests, and UI parity or startup checks where applicable.
