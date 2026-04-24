# Task 195: Runtime Task Engine And Dual Ledger

## Priority
P0

## Objective
Persist every main request as a runtime task in the session ledger, while mirroring only the right subset of tasks into project `_tasks`.

## Why This Exists
Main tasks need crash-safe persistence and resumability even when they are not project-management work. Project `_tasks` are for human-visible repo planning, not for every runtime operation Elma performs.

## Required Behavior
- Every `MainTask` request must create a `RuntimeTaskRecord` before execution starts.
- `Simple` requests must not create runtime task state.
- Runtime task progression must be independent of `_tasks` file mutation.
- Task progress must be visible through the existing task/todo UI.

## Required Types
- `RuntimeTaskRecord`
- `TaskMirrorPolicy = SessionOnly | SessionAndProject`
- any helper types needed for stage progress and resume metadata

## Minimum RuntimeTaskRecord Fields
- stable task id
- created/updated timestamps
- user objective
- request class
- formula id
- formula stages snapshot
- current stage index
- mirror policy
- gate reason
- stage notes / progress log
- stop reason
- completed flag

## Persistence Layout
Persist under the session root in a dedicated runtime task directory.
At minimum write:
- `latest.json`
- one history file per runtime task id

## Resume Rules
- On startup/session resume, Elma may load the latest runtime task record.
- Resume should only be used when the record is incomplete and relevant to the session.
- If the persisted task is complete, it may still be shown for history but must not automatically resume.

## Dual-Ledger Rules
- `SessionOnly`: default for normal main tasks.
- `SessionAndProject`: only for planning work or explicit user request to create/update repo task files.
- Runtime task completion must not require project mirror success.

## UI Requirements
- Show whether the current request is a persisted main task.
- Show formula and current stage.
- Keep task UI state derived from runtime task state, not a second disconnected store.

## Acceptance Criteria
- Main tasks survive hangs or restarts with enough persisted state to resume.
- Simple requests do not pollute the runtime task ledger.
- The UI can disclose the active task and formula stage without reading project task files.
- Runtime task persistence works even in folders that have no `_tasks` scaffold.

## Required Tests
- creating a main task writes `latest.json`
- a simple request does not write runtime task files
- stage advancement updates persisted state
- finalization writes completion state and optional stop reason
- startup can load the latest incomplete runtime task cleanly
