# Task 173: Prompt Input, Slash Picker, File Mentions, And Command Modes

## Status
Completed.

## Completion Notes (2026-04-22)
- Added `picker_state` and `input_mode` fields to `ClaudeRenderer`.
- Wired `/` prefix to open slash command picker with fuzzy filtering.
- Wired `@` prefix to open file quick-open picker with workspace file discovery.
- Wired `!` prefix to enter bash mode with visible Cyan indicator (`! ` instead of `> `).
- Added picker rendering in `render_ratatui` with Pink selection, Cyan file accents, grey metadata.
- Added Up/Down navigation, Enter to select, Esc to cancel for both pickers.
- Picker handling integrated into both normal and busy-input paths.
- Old autocomplete dropdown still present in `ui_autocomplete.rs` but superseded by picker for slash commands.
- All 429 tests pass (404 unit + 25 parity).

## Progress Notes (2026-04-21)
- Added pseudo-terminal fixture coverage for slash/file/mode/keybinding paths:
  - `slash-picker`
  - `file-picker`
  - `bash-mode`
  - `double-escape-clear`
  - `double-ctrl-c-exit`
- Parity harness now supports raw control-key input steps (`send_enter: false`) needed for double-key semantics.
- Current verification:
  - `cargo test --test ui_parity` passes with these fixtures.
  - `./ui_parity_probe.sh --all` passes.
- Remaining:
  - tighten assertions to validate picker-specific visible state, not just non-empty output.
  - confirm command mode indicators and picker affordances match Task 173 acceptance wording.
  - add explicit fixture/assertion coverage for queued input submission while a turn is active.

## Objective
Implement Claude Code-style prompt input, footer help, slash command picker, file mention picker, command modes, and keybindings.

## Existing Work To Absorb
This task absorbs:

- `_tasks/pending/013_Smart_Input_Prefixes_And_Command_Modes.md`
- `_tasks/pending/014_Chord_Keybindings_And_Keyboard_Shortcuts.md`
- `_tasks/pending/015_Chat_Undo_Buffer_And_Conversation_History.md` for input undo/history behavior.
- `_tasks/pending/104_Intelligent_Clipboard_Detection.md`
- `_tasks/pending/105_Integrated_Context_Aware_Hints.md`

## Claude Source References
- `components/PromptInput/PromptInput.tsx`
- `components/PromptInput/PromptInputFooter.tsx`
- `components/PromptInput/PromptInputHelpMenu.tsx`
- `components/design-system/FuzzyPicker.tsx`
- `components/QuickOpenDialog.tsx`
- `hooks/useTextInput.ts`
- `commands.ts`
- `keybindings/defaultBindings.ts`

## Prompt Editor Requirements
Support the Claude-style editing model:

- Normal text input at the bottom prompt.
- Multiline with Shift/Meta Enter and backslash Enter behavior where practical.
- Ctrl-A/Ctrl-E line navigation.
- Ctrl-B/Ctrl-F character navigation.
- Alt-B/Alt-F word navigation.
- Ctrl-U/Ctrl-K/Ctrl-W editing.
- Ctrl-Y paste/yank behavior where supported.
- Ctrl-_ undo.
- Up/Down history navigation.
- Double Esc clears the prompt and shows a brief notification.
- Double Ctrl-C or Ctrl-D exits.
- Ctrl-O toggles transcript expansion.
- Ctrl-T toggles the task list.
- Ctrl-G opens the prompt in `$EDITOR` if feasible.
- Ctrl-Z suspend where platform support is safe.
- Focus/paste handling must not corrupt terminal state.

## Command Modes
Implement Claude-like prefix modes:

- `/` slash command picker.
- `!` bash mode.
- `@` file/path mention quick open.
- `&` background mode only if Elma has an equivalent safe background execution path; otherwise show a parity-styled unavailable notice.
- `/btw` or side-question mode only if there is a single-LLM equivalent; otherwise defer explicitly.

Do not implement cloud/team/agent features that contradict the single local LLM scope.

## Slash Commands
Implement or adapt a core Claude-like command set:

- `/help`
- `/clear`
- `/compact`
- `/context`
- `/cost` or `/usage`
- `/diff`
- `/doctor`
- `/memory`
- `/model`
- `/permissions` or `/approve`
- `/resume`
- `/session`
- `/status`
- `/tasks`
- `/theme`
- `/vim`
- `/exit`

Commands that are not meaningful for Elma's local single-model mode should appear only if they have a truthful local behavior or a clearly styled "not available in local mode" response.

## Picker Requirements
Slash and file pickers must match Claude Code's interaction style:

- Search field.
- Fuzzy ranked results.
- Keyboard navigation with arrows and Ctrl-P/Ctrl-N where applicable.
- Enter selects.
- Tab and Shift-Tab secondary actions where the source supports them.
- Esc cancels.
- Preview pane for file quick-open when terminal width allows.
- Compact bottom preview when width is narrow.
- Pink active selection, Cyan file/tool accents, grey metadata.

## Suggested Dependencies
- `tui-textarea` or a custom editor if it gets closer to Claude behavior.
- `nucleo-matcher` for fuzzy matching.
- `ignore` for workspace file discovery, already present.
- `arboard` for clipboard detection and paste affordances.

## Acceptance Criteria
- Typing `/` opens a Claude-style fuzzy command picker.
- Typing `@` opens a Claude-style file quick-open picker.
- Typing `!` enters shell command mode with visible mode indication.
- Help/footer hints match Claude behavior and do not become permanent clutter.
- Keybindings work in the real TUI, including double Esc and double Ctrl-C/Ctrl-D semantics.
- Old Elma autocomplete dropdown visuals are removed from the active path.

## Verification
Run:

```bash
cargo fmt --check
cargo build
cargo test input -- --nocapture
cargo test keybinding -- --nocapture
cargo test ui_parity_prompt -- --nocapture
./ui_parity_probe.sh --fixture slash-picker
./ui_parity_probe.sh --fixture file-picker
./ui_parity_probe.sh --fixture bash-mode
./ui_parity_probe.sh --fixture double-escape-clear
./ui_parity_probe.sh --fixture double-ctrl-c-exit
```

The final verification must drive the real CLI in a pseudo-terminal, press keys, and assert visible picker/footer state changes.
