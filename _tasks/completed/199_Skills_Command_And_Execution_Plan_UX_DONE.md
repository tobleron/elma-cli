# Task 199: Skills Command And Execution-Plan UX

## Priority
P1

## Objective
Add `/skills` and make the selected execution mode visible during each request.

## Why This Exists
The new product direction is only useful if the user can see what Elma decided to do. A fresh session should make it obvious whether a request is simple or a persisted main task, which formula was selected, and which stage is active.

## Required Behavior
- `/skills` must list built-in skills and built-in formulas.
- The live UI must show:
  - `simple` vs `main_task`
  - selected formula
  - current stage for main tasks
- The display must stay synchronized with runtime task persistence.

## UI Requirements
- Reuse the existing message-first and modal architecture.
- Do not add bulky legacy chrome.
- Show execution-plan state compactly in header and transcript-native surfaces.
- The bottom status/footer bar must remain limited to core runtime metrics such as model/context/tokens/timing.
- Do not show execution mode, queue notices, or transient operational notifications in the footer.
- If a document backend is active later, leave space for backend disclosure without redesigning the UI model.

## Transcript Notification Rules
- Queue notices, selected-mode notices, and operational state changes must be rendered into the chat history as transcript-native meta/system rows.
- If a notification is transient, it may auto-collapse or auto-fade after a short timeout, but it must first appear in the transcript rather than only in a footer line.

## `/skills` Content Requirements
- skills and what each is for
- formulas and their stage order
- statement that main tasks are persisted in the session ledger
- clear distinction between runtime tasks and project `_tasks`

## Acceptance Criteria
- `/skills` is available from the command surface and slash picker.
- The user can tell which formula Elma chose for the current request.
- The user can tell when a request has become a persisted main task.
- UI wording does not imply that every main task mutates project `_tasks`.

## Required Tests
- slash picker includes `/skills`
- modal text includes formulas and session-ledger explanation
- active header reflects current formula label
- main-task state changes after selection are visible without restarting the UI
- queue notices and mode-selection notices appear in transcript rows rather than the footer
- footer remains limited to core runtime metrics during queued and in-flight states
