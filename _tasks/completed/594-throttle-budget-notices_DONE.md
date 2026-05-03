# Task 594: Throttle Budget Notice Frequency

## Session Evidence
Session `s_1777823506_810966000`: Budget notices appeared at every iteration near the limit:
- "Budget Approaching iteration limit (18/20)"
- "Budget Approaching iteration limit (19/20)"
- "Budget Approaching iteration limit (20/20)"
- "StopReason Tool call limit: repeated_same_command"
- "Budget Approaching iteration limit (18/20)" (cycle 2)
- "Budget Approaching iteration limit (19/20)" (cycle 2)
- "Budget Approaching iteration limit (20/20)" (cycle 2)
- "StopReason Budget limit: iteration_limit_reached"

That's 8 operational notices in one session. Combined with stagnation messages and identical-error messages, the user's view is polluted with system internals.

## Problem
Budget notices fire on every iteration within 2 of the limit. This was useful for debugging but overwhelms the user experience. Operational metrics should be available as collapsible transcript rows (per AGENTS.md rule 6), not inline spam.

## Solution
1. Drop budget notices from the inline transcript entirely
2. Instead, surface them as COLLAPSIBLE transcript rows (user presses a key to expand)
3. Only show ONE budget warning per cycle, at the 2/3 mark (not every iteration)
4. Collapse identical-error and stagnation messages into a single expandable row per trigger
5. The footer bar (status bar) should show a progress indicator: `[14/20 iterations]` rather than printing notices

Implementation: Modify `tui.push_budget_notice()` in `ui_terminal.rs` to aggregate and collapse, not print inline.
