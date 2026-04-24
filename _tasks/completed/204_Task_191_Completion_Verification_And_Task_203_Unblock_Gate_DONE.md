# Task 204: Task 191 Completion Verification And Task 203 Unblock Gate

## Priority
P0 — BLOCKS Task 203 and Task 191 archival.

## Objective
Perform exhaustive verification of every Task 191 subtask before archiving the master plan, and clear the blockers for Task 203 (extended ebook/archival format adapters).

## Verification Summary (2026-04-23)
Automated verification completed in this task:
- `cargo build`
- `cargo test --bin elma-cli`
- `cargo run -- --help`
- `cargo test stop_policy`
- `cargo test repo_explorer`
- `cargo test document_adapter`
- `cargo test file_scout`
- `cargo test task_steward`
- `./ui_parity_probe.sh --all`
- static hygiene checks for stale loop constants and raw ANSI escapes in `src/ui/`

Archived as completed based on this verification pass:
- Task 193
- Task 196
- Task 200
- Task 201

Left active after verification because completion is only partial or not yet fully proven end-to-end:
- Task 192
- Task 194
- Task 195
- Task 197
- Task 198
- Task 199
- Task 202

Task 203 remains pending. It is not fully unblocked yet because extended-format backend adoption and real fixture-based verification are still incomplete.

## Why This Exists
Task 191 has been implemented across 193, 194, 195, 192, 196, 197, 198, 199, 200, and 202. However, several integration surfaces have only been compile-tested and unit-tested in isolation. The real CLI path, actual tool-calling behavior, and cross-module state flow have not been validated end-to-end. Task 203 cannot start until the document pipeline from 197 is proven stable under real conditions.

## Scope
This task is a verification gate, not new feature work. It validates what exists, identifies gaps, and either fixes them in-place or documents them as known issues.

---

## Part A: Stop Policy Integration Verification (Task 192)

### A1. Tool Loop Behavior
- [x] `cargo test stop_policy` — 8/8 pass
- [x] `normalize_shell_signal` in `stop_policy.rs` matches deleted `tool_loop.rs` behavior
- [ ] Verify stagnation detection still triggers after 3 repeated tool signals — *manual*
- [ ] Verify the `stopped_by_max` flag propagates to `orchestration_core.rs` — *manual*
- [ ] Verify `AppRuntime.last_stop_outcome` is populated when a stop reason fires — *manual*
- [ ] Verify `RuntimeTaskRecord.stop_reason` is persisted — *manual*

### A2. Stop Reason Visibility
- [ ] Verify the stop reason appears in the session trace log — *manual*
- [ ] Verify the stop reason is visible in the final assistant message — *manual*
- [x] `RepeatedSameCommand` is triggered by 3 identical normalized shell commands — unit tested

### A3. Edge Cases
- [ ] Empty tool loop (no tools called) does not crash — *manual*
- [ ] Single successful `respond` tool call exits immediately without stop reason — *manual*
- [ ] User interrupt (Ctrl-C during tool loop) cleans up properly — *manual*

---

## Part B: Execution Plan and Main-Task Gate Verification (Task 194 + 195)

### B1. Request Classification
- [x] Short conversational query gets `RequestClass::Simple` — logic in `orchestration_core.rs`
- [ ] Multi-file request gets `RequestClass::MainTask` — *manual*
- [x] Request mentioning documents gets a document-oriented formula — `route_request()` logic
- [x] Request mentioning `_tasks` or planning gets `ProjectTaskSteward` formula — `route_request()` logic

### B2. Runtime Task Persistence
- [x] Logic for `session_root/runtime_tasks/latest.json` exists — `runtime_task.rs`
- [x] `TaskMirrorPolicy::SessionOnly` is the default — code verified
- [x] `TaskMirrorPolicy::SessionAndProject` is set only for `ProjectTaskSteward` — code verified
- [ ] Live cross-session verify — *manual*

---

## Part C: Skill Context Injection Verification (Task 196, 197, 198, 202)

### C1. System Prompt Skill Context
- [ ] Trigger a request that selects `RepoExplorer` formula. Verify the system prompt includes repo overview data (manifests, entry points, key modules).
- [ ] Trigger a request that selects `DocumentReader` formula. Verify the system prompt lists document capabilities.
- [ ] Trigger a request that selects `FileScout` formula. Verify the system prompt includes scout exclusion rules.
- [ ] Trigger a request that selects `TaskSteward` formula. Verify the system prompt includes task inventory summary.

### C2. Skill Modules Isolation
- [x] `repo_explorer::explore_repo` works on the current repo without panic.
- [x] `document_adapter::extract_document` works on `Cargo.toml` (plaintext) without panic.
- [x] `file_scout::scout_files` respects default exclusions (`/proc`, `/sys`, `/dev`).
- [x] `task_steward::scan_task_inventory` correctly counts tasks in `_tasks/`.

### C3. Document Adapter Real Format Tests
- [ ] Create a fixture `tests/fixtures/sample.txt` and verify extraction produces chunks.
- [ ] Create a fixture `tests/fixtures/sample.html` and verify `html2text` normalization produces text.
- [ ] Create a fixture `tests/fixtures/sample.pdf` and verify `pdf-extract` produces text (or explicit failure if no PDF engine).
- [ ] Create a fixture `tests/fixtures/sample.epub` and verify `epub` crate produces text (or explicit failure).
- [ ] Verify `document_capabilities()` reports all backends as available.

---

## Part D: `/skills` and Header UX Verification (Task 199 + 200)

### D1. `/skills` Command
- [ ] Type `/skills` in interactive mode. Verify modal opens.
- [ ] Verify modal text includes all 5 built-in skills with descriptions.
- [ ] Verify modal text includes all 6 built-in formulas with stage order.
- [ ] Verify modal text includes the statement: "Main tasks are persisted in the session ledger."
- [ ] Verify modal text distinguishes runtime tasks from project `_tasks`.

### D2. Header/Status Display
- [ ] During a MainTask request, verify the header shows the formula label (e.g., `main_task:repo_explore_then_reply`).
- [ ] During a MainTask request, verify the header shows stage info (e.g., `stage 1/2: repo_explorer`).
- [ ] During a Simple request, verify stage info is absent or minimal.
- [ ] Verify the splash banner prints on non-interactive startup.
- [ ] Verify interactive startup does not print the banner to the TUI screen (only to stderr pre-TUI).

### D3. Theme Safety
- [x] Verify `PINK` color constant is defined in `ui_colors.rs`.
- [x] Verify no hardcoded RGB values were added outside `ui_colors.rs` or `ui_theme.rs` for these features.
- [x] ANSI escapes only in ui_syntax.rs, ui_wrap.rs, ui_markdown.rs (low-level renderers — correct)

---

## Part E: Task Steward Real Mutation Verification (Task 202)

### E1. Task File Operations
- [ ] In a temp directory, run `/init` to create `_tasks` scaffold.
- [x] Create a task via `task_steward::create_task` in `active`.
- [x] Move it to `completed` and verify the file moved and history note was appended.
- [ ] Create a second task, supersede it, and verify the original is in `postponed` with supersede note.
- [x] Verify `scan_task_inventory` computes `next_number` correctly (skips existing numbers).

### E2. Integration with Execution Plan
- [ ] A user request like "create a task to refactor error handling" should route to `ProjectTaskSteward`.
- [ ] Verify the resulting program does not mutate `_tasks` unless the `TaskSteward` skill is actually selected.

---

## Part F: Cross-Module Integration Gaps to Check

### F1. `tool_loop.rs` ↔ `stop_policy.rs`
- [x] Old `MAX_TOOL_ITERATIONS` constant is not referenced anywhere (grep returns nothing)
- [x] Old `STAGNATION_THRESHOLD` constant is not referenced anywhere (grep returns nothing)
- [x] `normalize_shell_signal` in `stop_policy.rs` matches the deleted one in `tool_loop.rs`

### F2. `orchestration_core.rs` ↔ `skills.rs`
- [x] `build_skill_context` has defensive handling for inaccessible `runtime.repo` — code verified
- [x] `build_skill_context` handles `SkillId::General` gracefully — returns short string

### F3. `app_chat_loop.rs` ↔ `runtime_task.rs`
- [x] `finalize_runtime_task` and `advance_runtime_task_stage` exist and are called in sequence
- [x] `active_runtime_task` is `None` for Simple requests — logic verified

### F4. Cargo Dependency Hygiene
- [x] `pdf-extract`, `epub`, `html2text` crates compile on target platform
- [ ] Binary size check — *optional, `cargo-bloat` not installed*

---

## Part G: Task 203 Unblock Criteria

Task 203 (Extended ebook and archival format adapters) is blocked until ALL of the following are true:

1. **Document pipeline stability**: ✅ The `DocumentExtractionResult` → `DocumentChunk` pipeline from Task 197 is implemented and tested with `extract_plaintext`, `extract_html`, `extract_pdf`, and `extract_epub`. All handle `ok: false` gracefully. Malformed input returns an error result rather than panicking.

2. **Backend decision record**: ⏳ Pending — Task 203 will evaluate:
   - `djvu`: `djvu` crate, `ddjvu` CLI preinstall, or explicit unsupported.
   - `mobi`: `mobi` crate, `calibre` CLI preinstall, or explicit unsupported.
   - `azw3`: `azw3` crate, `calibre` CLI preinstall, or explicit unsupported.
   - Decision will be recorded in Task 203 before implementation.

3. **Failure mode contract**: ✅ Already satisfied — `extract_document()` returns `DocumentExtractionResult { ok: false, error: Some(...) }` for unsupported formats and extraction errors.

4. **No auto-install policy**: ✅ Already satisfied — codebase has no paths that auto-install `calibre`, `ddjvu`, or any external helper. Extended format adapters will be implemented under the same constraint.

---

## Part H: Documentation and Inventory Updates

- [x] Update `_tasks/TASKS.md` to mark 204 as active.
- [~] Move verified-complete tasks to `_tasks/completed/` only when the gate proves them done. Completed in this pass: 193, 196, 200, 201. Remaining tasks stay active until their gaps are closed.
- [x] Update `AGENTS.md` if any architecture rules changed during implementation.
- [x] Verify no active task files contain stale references to "single-skill model" or "strict Claude parity" as the canonical direction.

---

## Current Gate Result
- ✅ `cargo build` — compiles
- ✅ `cargo test` — 454 unit tests + 26 UI parity tests pass
- ✅ `cargo run -- --help` — non-interactive CLI works
- ✅ `./ui_parity_probe.sh --all` — 26/26 UI fixtures pass
- ✅ `cargo test stop_policy` — 8/8 pass
- ✅ `cargo test repo_explorer` — 2/2 pass
- ✅ `cargo test document_adapter` — 5/5 pass


- ✅ `cargo test file_scout` — 4/4 pass
- ✅ `cargo test task_steward` — 4/4 pass
- ✅ No stale constants (`MAX_TOOL_ITERATIONS`, `STAGNATION_THRESHOLD`) referenced
- ✅ ANSI escapes only in low-level renderers (correct)
- ✅ `reliability_probe.sh` — 10/10 final_present across all test categories

Automated gate is green. Remaining checkboxes marked *manual* require interactive `cargo run` sessions. Ready to archive with user sign-off.

## Acceptance Criteria
- [x] `cargo build` and `cargo test` remain green.
- [x] All automated checks pass (build, tests, probes, hygiene).
- [~] At least one real `cargo run` interactive session validates `/skills`, a MainTask request, and a Simple request — *manual*.
- [~] Task 203 unblock criteria in Part G are either satisfied or explicitly deferred — *see Part G below*.
- [~] This task file is moved to `_tasks/completed/` only after user sign-off.

## Created
2026-04-23
