# Task 174: Todo Tool And Claude-Style Task List

## Status
Completed.

## Completion Notes (2026-04-22)
- Added `render_ratatui()` method to `TaskList` with native ratatui `Line`/`Span` rendering.
- Completed items now show dim + strikethrough styling with `✓` checkmark.
- In-progress items show bold Pink accent with `◐` marker.
- Pending items show normal text with `○` marker.
- Blocked items show dim italic with `◌` marker.
- Added hidden count indicator ("... X more hidden") when tasks exceed `max_visible`.
- Replaced `task_lines: Vec<String>` in `ClaudeRenderer` with `task_list: Option<TaskList>`.
- `TerminalUI::draw_claude()` now passes full `TaskList` to renderer instead of pre-rendered ANSI strings.
- All 429 tests pass (404 unit + 25 parity).

## Progress Notes (2026-04-21)
- Added `update_todo_list` tool shape in tool-calling flow with actions:
  - `add`, `update`, `in_progress`, `completed`, `blocked`, `remove`, `list`.
- Added interactive task list state and renderer wiring in `TerminalUI`:
  - task add/update/start/complete/block/remove
  - Ctrl-T toggle
  - task rows rendered in Claude renderer task section.
- Added parity fixtures:
  - `todo-create`
  - `todo-progress-checkmark`
  - `todo-toggle`
- Current verification:
  - `cargo test --test ui_parity` passes.
  - `./ui_parity_probe.sh --all` passes.
- Remaining:
  - strengthen fixtures/assertions for explicit checkmark/progress glyph transitions.
  - confirm persistent session semantics for todo items.

## Objective
Add a local single-LLM Todo tool and render its state through a Claude Code-style task list that appears, updates, and checkmarks automatically during work.

## Existing Work To Absorb
This task absorbs:

- `_tasks/pending/007_Add_UpdateTodoList_Tool.md`
- `_tasks/pending/100_Interactive_Task_Progress_Tree.md`

Do not implement an agent/subagent tree for this scope. Claude parity requires a task/todo list display, not Elma-specific nested agent visualization.

## Claude Source References
- `_stress_testing/_claude_code_src/components/TaskListV2.tsx`
- Ctrl-T behavior referenced by Claude keybindings.
- Tool/task update behavior visible through the Claude message pipeline.

## Tool Requirements
Create a Todo tool suitable for small local models:

- Tool name: `update_todo_list` or the closest Claude-compatible name chosen during implementation.
- Actions:
  - add item,
  - update item text,
  - mark in progress,
  - mark completed,
  - mark blocked,
  - remove item,
  - list current items.
- Stable item IDs.
- Optional owner/activity fields only if they do not imply multi-agent delegation.
- Persistence within the active session.
- Compact JSON schema with clear model instructions.

## UI Requirements
Render tasks like Claude Code:

- Completed item: checkmark and dim/strikethrough where terminal support allows.
- In-progress item: strong active marker and bold text.
- Pending item: empty marker and normal text.
- Blocked item: dim marker and reason if available.
- Auto display when a task list exists and work is active.
- Ctrl-T toggles visibility.
- Truncate intelligently based on terminal height.
- Show hidden count when rows are omitted.
- Update checkmarks in place as tool events arrive.
- Use Pink for active attention and Cyan for progress/metadata as defined by Task 168.

## Small-Model Prompting
The tool prompt must be principle-first and compact:

- Use the tool when a task has multiple visible steps or the user explicitly asks for progress tracking.
- Keep item text short and user-facing.
- Update statuses as work completes.
- Do not create artificial subtasks for trivial chat.

Avoid long examples or keyword-based routing.

## Files To Inspect Or Change
- `src/tool_calling.rs`
- `src/types_core.rs`
- `src/app_chat_loop.rs`
- `src/ui_state.rs`
- new UI renderer modules from Task 169.
- session persistence modules.

## Acceptance Criteria
- The model can create and update todo items through a tool call.
- The task list appears automatically during multi-step work.
- Items visibly transition from pending to in-progress to completed.
- Ctrl-T toggles the list.
- The implementation does not add subagent delegation or multi-model assumptions.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test todo -- --nocapture
cargo test ui_parity_task_list -- --nocapture
./ui_parity_probe.sh --fixture todo-create
./ui_parity_probe.sh --fixture todo-progress-checkmark
./ui_parity_probe.sh --fixture todo-toggle
```

The final verification must run a real CLI fixture where the fake model issues Todo tool calls and the pseudo-terminal snapshot shows automatic task list updates.
