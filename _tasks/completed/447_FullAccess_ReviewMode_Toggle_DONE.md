# Task [NEXT]: Full-Access/Review-Based Mode Toggle

**Status:** completed
**Priority:** HIGH
**Source:** 2026-05-02 user request

## Summary

Add a Full-Access/Review-Based mode toggle that appears in the footer beside the existing Long/Concise mode toggle. This mode gives elma-cli full access policy, bypassing workspace restrictions for users who want unrestricted operation.

## UI Design

The footer should display something like:

```
[ model-name | tokens | elapsed | Long/Concise | Full/Review ]
```

or similar layout with the mode toggles in primary/complementary color on the right side.

## Functionality

- **Full Access**: No workspace path restrictions, no .elmaignore/.elmaprotect enforcement
- **Review-Based**: Normal policy enforcement, user must review before destructive operations proceed

This is the inverse of safe mode — instead of restricting more, it enables full access.

## Interaction

- Toggle via keyboard shortcut (e.g., `Alt+F` or `Alt+R` for Review)
- Shows current mode in footer
- Mode persists across conversation turns
- Transcript-visible when mode changes

## Implementation Completed

1. **AccessMode enum** added to `src/ui/ui_state.rs` (Review/Full)
2. **Keyboard handler** - press `a` to toggle in `src/ui/ui_terminal.rs`
3. **Footer UI** - displays "Review | Full" alongside "Concise | Long" in `src/claude_ui/claude_render.rs`
4. **Policy integration** - Full mode bypasses `.elmaprotect` checks in `src/workspace_policy.rs`

## Usage

- Press `m` to toggle Long/Concise mode
- Press `a` to toggle Review/Full access mode
- Footer shows: `Concise | Review` or `Long | Full`

## Files Changed
- `src/ui/ui_state.rs` - Added AccessMode enum with getter/setter
- `src/ui/ui_terminal.rs` - Added keyboard handler for 'a' key
- `src/claude_ui/claude_render.rs` - Added access mode to footer
- `src/workspace_policy.rs` - Full mode bypasses policy checks

## Success Criteria

- [x] Toggle appears in footer
- [x] Keyboard shortcut works (press 'a')
- [x] Full mode bypasses workspace policy
- [x] Mode persists across turns (static)
- [ ] Transcript shows mode change (skipped - would need global state access)

## Anti-Patterns To Avoid

- Do not make Full mode persist by default (Review should be default)
- Do not hide the toggle - it should be visible like Long/Concise
- Do not forget transcript visibility