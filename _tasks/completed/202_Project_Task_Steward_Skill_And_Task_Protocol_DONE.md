# Task 202: Project Task Steward Skill And `_tasks` Protocol

## Priority
P0

## Objective
Give Elma a dedicated project-task skill that operates on `_tasks` using `AGENTS.md` and `_tasks/TASKS.md` as mandatory guidance.

## Why This Exists
Project task management is a special domain. It must respect repo task protocol, numbering, active master plans, and historical state. That should not be mixed into generic runtime task persistence.

## Required Behavior
- Read project guidance before mutating `_tasks`.
- Allocate the next task number correctly across active/pending/completed/postponed.
- Create, move, postpone, supersede, and archive task files consistently.
- Update the relevant master plan rather than creating duplicate planning.
- Mirror only project-planning work into `_tasks`; keep generic runtime tasking out of this skill.

## Required Operations
- create new task
- create troubleshooting task
- move pending to active
- move active to completed/postponed
- mark superseded with explicit replacement target
- update `_tasks/TASKS.md`
- update relevant master plan references

## Required Safety Rules
- never overwrite unrelated task history silently
- never allocate a number without scanning the actual task directories
- never create duplicate master-plan threads for the same initiative
- preserve historical references when superseding

## Acceptance Criteria
- Elma can manage `_tasks` structure consistently in a portable project.
- Numbering and file moves stay grounded in actual task inventory.
- Generic runtime task persistence does not require `_tasks` mutation.
- A future session can understand why a task was superseded or postponed.

## Required Tests
- next-number allocation across all task folders
- pending to active move
- active to postponed move with preserved history note
- supersede operation updates filenames/content consistently
- `_tasks/TASKS.md` remains coherent after steward operations
