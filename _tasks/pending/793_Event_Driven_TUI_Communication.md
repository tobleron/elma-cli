# Task 793: Implement Event-Driven TUI Communication

## Status
- **Priority:** Medium
- **Assignee:** Unassigned
- **Status:** Pending

## Objective
Replace polling-based UI updates with a central event bus.

## Requirements
- Utilize or expand `src/pubsub.rs`.
- Decouple orchestration logic from `TerminalUI` direct calls.
- Ensure the UI remains responsive and correctly renders all current events.

## Verification
- Manual UI walkthrough.
- Check for race conditions in event delivery.
