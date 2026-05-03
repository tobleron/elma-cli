# Task 436: `/sessions` Resume Dialog For Slim Sessions

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Depends on:** Task 430
**Related:** Task 413, Task 433
**References:** `src/claude_ui/claude_input.rs`, `src/app_chat_loop.rs`, `src/ui/ui_terminal.rs`, `src/ui/ui_state.rs`, `src/claude_ui/claude_render.rs`, `src/session_index.rs`, `src/session_paths.rs`, `src/session_write.rs`

## Problem

Task 430 makes `session.md` and `session.json` the slim source of truth for session storage and reload.

Elma already exposes `/sessions` in the slash-command list and has a simple `/resume` modal that lists session directory names, but it does not provide a professional session manager or a real resume path based on the new architecture.

Current gaps:

- `/sessions` is advertised but not handled as a command.
- `/resume` opens a basic `ModalState::Select` list of session ids and does not actually load a selected session.
- `Ctrl+L` opens a placeholder sessions modal with only "New session".
- The modal has no session metadata, preview, search/filter, recency sorting, current-session marker, or load failure handling.
- Resume behavior is not tied to the Task 430 contract of `session.md` plus `session.json`.
- Session reload should not depend on old duplicate files such as `display/`, `runtime_tasks/`, `hierarchy/`, or root `session_status.json`, except as legacy fallback.

## Objective

Add a first-class `/sessions` command that opens a professional terminal dialog for browsing recent sessions and resuming one.

The dialog should be keyboard-first, visually consistent with Elma's theme, and backed by the new slim session architecture:

```text
sessions/
  index.json
  s_{id}/
    session.md
    session.json
    thinking.jsonl          # optional
    summaries/             # optional
    artifacts/             # optional
```

Selecting a session must restore enough runtime state to continue the chat reliably without recreating the noisy legacy session folder structure.

## Non-Goals

- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT`.
- Do not reintroduce DSL session state, DSL commands, or DSL repair logic.
- Do not build a full file-browser UI.
- Do not implement arbitrary session editing, deletion, or migration in this task.
- Do not silently mutate legacy sessions during browsing or resume.
- Do not duplicate transcript content into new files just to support the picker.

## UX Contract

### Entry Points

Support these entry points:

- `/sessions`: open the new session manager.
- `/resume`: alias to the same session manager, initially focused on the most recent resumable session.
- `Ctrl+L`: open the same session manager instead of the current placeholder modal.

The slash picker should keep `/sessions` as a visible command with a clear description such as "Browse and resume sessions".

### Dialog Layout

Use a dedicated session picker state instead of overloading the current static `ModalState::Select`.

The dialog should show:

- Title: `Sessions`.
- Search/filter input.
- Recent session list, sorted by `last_modified_unix` descending.
- Current session marker.
- Status marker: `active`, `completed`, `interrupted`, `error`, or `unknown`.
- Relative age or absolute timestamp.
- Model name when available from `session.json.runtime.model`.
- Short preview from `session.md` or compact metadata from `session.json`.
- Footer hints: `Enter` resume, `N` new session, `R` refresh, `Esc` close.

The UI should remain usable in small terminals:

- Clamp width and height to the viewport.
- Keep list rows single-line with ellipsis.
- Show a compact preview only when space allows.
- Never let text overflow or overlap.

### Keyboard Behavior

Required keys:

- `Up` / `Down`: move selection.
- `PageUp` / `PageDown`: move by visible page.
- `Home` / `End`: jump to first/last.
- Printable characters: update filter query.
- `Backspace`: edit filter query.
- `Enter`: resume selected session.
- `N`: start a new session.
- `R`: rebuild/refresh session index.
- `Esc`: close without action.

If there are no sessions, show an empty-state message and keep `N` and `Esc` useful.

## Data Contract

Add a focused session picker/read model, for example:

```rust
pub(crate) struct SessionPickerEntry {
    pub(crate) id: String,
    pub(crate) path: PathBuf,
    pub(crate) status: String,
    pub(crate) created_at_unix: u64,
    pub(crate) last_modified_unix: u64,
    pub(crate) model: Option<String>,
    pub(crate) workspace_root: Option<String>,
    pub(crate) transcript_path: Option<PathBuf>,
    pub(crate) preview: String,
    pub(crate) is_current: bool,
    pub(crate) resumable: bool,
    pub(crate) warning: Option<String>,
}
```

Primary sources:

- `sessions/index.json` for fast listing.
- `s_{id}/session.json` for status, runtime metadata, turn counts, summaries, and reload state.
- `s_{id}/session.md` for human-readable preview and visible transcript restore.

Legacy fallback, read-only:

- `display/terminal_transcript.txt` only if `session.md` is missing.
- `session_status.json` only if `session.json.status` is missing.
- Legacy runtime/hierarchy files only through the backward-compatible loading hooks introduced by Task 430.

## Resume Contract

Implement a resume loader that is independent from the picker UI:

```rust
pub(crate) struct SessionResumeSnapshot {
    pub(crate) session: SessionPaths,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) transcript_items: Vec<TranscriptItem>,
    pub(crate) goal_state: GoalState,
    pub(crate) active_runtime_task: Option<RuntimeTaskRecord>,
    pub(crate) restored_summary_notice: Option<String>,
}
```

Suggested API:

```rust
pub(crate) fn load_session_for_resume(
    sessions_root: &Path,
    session_id: &str,
    current_system_message: ChatMessage,
) -> Result<SessionResumeSnapshot>
```

Rules:

- Validate that `session_id` is a direct child of `sessions_root` and starts with `s_`.
- Never follow arbitrary paths from user input.
- Restore `SessionPaths` using the existing session root and its `artifacts/` directory.
- Prefer compact reload state from `session.json`.
- Use `session.md` to reconstruct visible chat transcript.
- Use summaries from `session.json.applied_summaries` and `summaries/` to keep model context bounded.
- Do not inject huge historical transcripts into the model context when a compact summary is available.
- If the selected session has corrupt or incomplete state, keep the dialog open and show a visible error row instead of panicking.
- Mark the previously active session as `interrupted` before switching when appropriate.

## Implementation Plan

### Phase 1: Session Listing Service

Create a small module such as `src/session_browser.rs`.

Responsibilities:

- Load `SessionIndex`.
- Rebuild the index if missing or stale enough to be obviously unusable.
- Convert index entries into `SessionPickerEntry`.
- Read `session.json` metadata through Task 430 helpers.
- Read bounded previews from `session.md` without loading large files fully.
- Sort by `last_modified_unix` descending.
- Mark the current session.
- Expose a filter function that matches id, status, model, workspace, and preview text.

Avoid hardcoded command keyword routing. Filtering is local UI search only and is allowed because it operates on already loaded session metadata, not behavioral intent.

### Phase 2: Dedicated Session Picker State

Extend UI state with a dedicated session picker model, for example:

```rust
pub(crate) struct SessionPickerState {
    pub(crate) visible: bool,
    pub(crate) query: String,
    pub(crate) selected: usize,
    pub(crate) entries: Vec<SessionPickerEntry>,
    pub(crate) error: Option<String>,
}
```

Integrate with:

- `src/ui/ui_state.rs`
- `src/ui/ui_terminal.rs`
- `src/claude_ui/claude_render.rs`

Do not use the static `ModalState::Select` for session browsing, because it cannot track selection, filtering, preview, refresh, or selected action cleanly.

### Phase 3: Command Wiring

Update `handle_chat_command`:

- `/sessions` opens the session picker.
- `/resume` opens the same picker.
- Existing `/help` text should list `/sessions`.

Update `Ctrl+L` handling:

- Open the same session picker populated from `session_browser`.
- Remove the placeholder "N — New session" modal.

Update slash command metadata:

- Keep `/sessions`.
- Consider changing `/resume` description to make clear it opens the same picker.

### Phase 4: Resume Runtime Switching

When `Enter` is pressed on a selected session:

- Call `load_session_for_resume`.
- Replace `runtime.session`.
- Replace `runtime.messages` with the restored bounded model messages.
- Restore `runtime.goal_state`.
- Restore active runtime task if present.
- Restore visible transcript items in the terminal UI.
- Add a transcript-native operational row such as `session resumed: s_...`.
- Update status/index for both the previous and resumed session.

Starting a new session with `N` should:

- Finalize or interrupt the previous session status as appropriate.
- Create a fresh session through `ensure_session_layout`.
- Reset runtime chat state while keeping global config/model/provider state.
- Add a visible operational row indicating a new session was started.

### Phase 5: Error Handling And Legacy Compatibility

Handle these cases explicitly:

- No sessions directory exists.
- Index is missing or corrupt.
- Session directory disappeared after listing.
- `session.json` is missing.
- `session.md` is missing but legacy transcript exists.
- Both transcript and machine state are missing.
- Selected session is the current session.
- Selected session has a different model/base URL than current runtime.

For model/base URL mismatch:

- Do not silently switch provider in this task.
- Show the stored model/base URL in the preview.
- Resume using the current configured provider unless a later config task adds provider switching.
- Add a visible notice when the resumed session was originally created with a different model.

## Files To Audit

| File | Reason |
|------|--------|
| `src/claude_ui/claude_input.rs` | Slash command catalog already includes `/sessions` |
| `src/app_chat_loop.rs` | Command handling for `/resume`, `/sessions`, `/help` |
| `src/ui/ui_terminal.rs` | Keyboard handling, modal/session picker input, `Ctrl+L` |
| `src/ui/ui_state.rs` | Add session picker state |
| `src/claude_ui/claude_render.rs` | Render professional dialog |
| `src/session_browser.rs` | New listing, filtering, preview, and metadata service |
| `src/session_index.rs` | Index load/rebuild/sort metadata |
| `src/session_paths.rs` | Construct `SessionPaths` for existing session roots |
| `src/session_write.rs` | Load `session.json` and restore goal/runtime state |
| `src/app_bootstrap_core.rs` | Ensure resume state matches runtime bootstrap invariants |
| `src/runtime_task.rs` | Restore active runtime task |
| `src/session_error.rs` | Status transitions for interrupted/resumed/error sessions |

## Success Criteria

- [ ] Typing `/sessions` opens a professional session manager dialog.
- [ ] Typing `/resume` opens the same dialog.
- [ ] Pressing `Ctrl+L` opens the same dialog.
- [ ] The dialog lists recent sessions sorted by last modified time.
- [ ] The current session is marked.
- [ ] Entries show status, timestamp, model when available, and a short preview.
- [ ] The list can be filtered from the keyboard.
- [ ] `Enter` resumes the selected session.
- [ ] `N` starts a new session.
- [ ] `R` refreshes or rebuilds the session list.
- [ ] `Esc` returns to the current chat without side effects.
- [ ] New architecture sessions resume from `session.json` and `session.md`.
- [ ] Legacy sessions remain readable through Task 430 fallback paths.
- [ ] Resume does not create legacy duplicate folders/files.
- [ ] Corrupt sessions produce visible UI errors instead of panics.
- [ ] Resuming a session adds a transcript-native operational row.

## Verification

Run the smallest relevant checks first:

```bash
cargo fmt --check
cargo test session -- --nocapture
cargo test ui -- --nocapture
cargo test session_browser -- --nocapture
```

Then run:

```bash
cargo build
```

Manual probes:

1. Start Elma, create two sessions, and confirm `/sessions` lists both.
2. Resume the older session and confirm the visible transcript is restored.
3. Confirm the model context is bounded by summaries/reload state rather than the full transcript when summaries exist.
4. Confirm `Ctrl+L`, `/resume`, and `/sessions` all open the same dialog.
5. Confirm no new `display/`, `runtime_tasks/`, `hierarchy/`, or root `tool-results/` folders are created by browsing or resuming.
6. Delete or corrupt one session's `session.json` and confirm the picker shows a clear warning without crashing.
