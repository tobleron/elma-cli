# Task 346: Keybinding And Command Mode Customization

## Backlog Reconciliation (2026-05-02)

Defer until Task 482 captures terminal UI regressions and Task 483 decides canonical UI renderer ownership.


**Status:** pending
**Source patterns:** Claude Code command UX, Crush Bubble Tea keymaps, Roo custom modes
**Revives:** `_tasks/postponed/014_Chord_Keybindings_And_Keyboard_Shortcuts.md`

## Summary

Add configurable keybindings and command-mode actions for the terminal UI while preserving Elma's minimal footer and transcript-native operational visibility.

## Why

Power users need fast terminal workflows. Elma's UI is already substantial, but global keybindings and command-mode customization are not yet treated as a first-class, schema-validated user configuration.

## Implementation Plan

1. Define a keybinding config schema with defaults.
2. Add validation for conflicts and unsupported chords.
3. Map keybindings to existing UI commands, not direct state mutations.
4. Surface command-mode errors in the transcript or command palette area, not the footer.
5. Add UI parity tests for default bindings.

## Success Criteria

- [ ] Users can customize common UI actions without recompiling.
- [ ] Invalid keybindings fail with clear config errors.
- [ ] Defaults match current behavior.
- [ ] Footer remains limited to model, token count, and elapsed time.
- [ ] Tests cover conflict detection and default action dispatch.

## Anti-Patterns To Avoid

- Do not add visible instructional text to the main app surface.
- Do not create mode-specific hidden behavior that is not discoverable through config.
- Do not break existing keyboard flows.
