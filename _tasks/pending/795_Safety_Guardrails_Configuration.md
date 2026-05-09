# Task 795: Implement Safety Options & Guardrails Configuration

## Status
- **Priority:** Critical
- **Assignee:** Unassigned
- **Status:** Pending

## Objective
Provide a centralized interface for configuring safety preflights, tool approval policies, and command budgets to prevent unintended destructive actions.

## Requirements
- Create a specific modal for "Safety Settings".
- **Tool Approval Policy:** Off (YOLO), Ask (Prompt for destructive), On (Review all).
- **Preflight Guards:** Toggle specific checks (e.g., block shell redirection, block path escapes).
- **Command Budget:** Configure maximum per-turn or per-session shell/write operations.
- **Confirmation Cache:** View and clear recently approved command patterns.

## Manageable Sub-tasks
1. Implement the Safety Modal UI.
2. Link `/approve` to the new Safety Modal instead of blind cycling.
3. Add toggles for `shell_preflight` restrictions.
4. Integrate `command_budget` visibility and limits.
