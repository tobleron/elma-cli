# Task 430: Slim Session Folder Storage Layout

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 2-4 days
**Depends on:** completed Task 381, completed Task 385, completed Task 422
**References:** `ba4eed7d`, `c9d0d4d7`, `620483e6`, `src/session_paths.rs`, `src/session_write.rs`

## Problem

The intended session layout was slim:

```text
sessions/s_{id}/
  session.md
  session.json
  thinking.jsonl
  artifacts/
```

Task 385 then added:

```text
sessions/s_{id}/summaries/
```

The current code still lets multiple subsystems create overlapping session data:

- `workspace.txt` and `workspace_brief.txt` duplicate workspace metadata.
- `session_status.json` and `error.json` duplicate status/error fields that belong in `session.json`.
- `runtime_tasks/latest.json` and per-task runtime JSON duplicate resumable task state.
- `hierarchy/*.json` spreads one decomposition state across many files.
- `display/terminal_transcript.txt`, `artifacts/terminal_transcript.txt`, numbered display captures, final-answer `.txt`, and final-answer `.md` duplicate the user-visible chat history.
- `tool-results/` duplicates the artifact role already covered by `artifacts/`.
- `evidence/{session_id}/ledger.json` adds another nested session state location.

This makes a session folder noisy, harder to inspect, and harder to reload reliably because the authoritative source of truth is spread across many files.

## Objective

Restore a professional minimal session folder without reintroducing the DSL work and without changing Elma's runtime behavior more than necessary.

New sessions should use one chat markdown file, a few dedicated JSON/JSONL files, and one summaries folder:

```text
sessions/
  index.json
  s_{id}/
    session.md
    session.json
    thinking.jsonl          # only when reasoning is persisted
    summaries/             # only when finalized summaries exist
    artifacts/             # only for large/raw external payloads and snapshots
```

`session.md` is the only durable user-visible chat history. `session.json` is the canonical machine-readable reload state. Other files must exist only when they have a dedicated non-duplicative purpose.

## Non-Goals

- Do not edit `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT`.
- Do not reintroduce DSL routing, DSL grammars, or DSL repair infrastructure.
- Do not redesign orchestration, routing, or tool-calling semantics.
- Do not remove raw artifact persistence needed for large outputs, snapshots, crash recovery, or evidence references.
- Do not migrate existing session folders in place unless an explicit migration command is requested later.

## Target Data Contract

### `session.md`

Append-only markdown transcript for durable chat history and human inspection.

Contents:

- User messages.
- Assistant final answers.
- Visible operational rows from Task 381, rendered as compact markdown sections.
- Tool rows with short summaries and artifact references when raw output is stored.
- Compact boundaries and summary notices.

Exclusions:

- Raw thinking bodies.
- Full large tool outputs.
- Duplicated final-answer `.txt`/`.md` files.

### `session.json`

Canonical machine state for reload and indexing.

Suggested top-level schema:

```json
{
  "schema_version": 2,
  "id": "s_...",
  "created_unix_s": 0,
  "updated_unix_s": 0,
  "status": {
    "state": "active|completed|error|interrupted",
    "turns_completed": 0,
    "last_turn": null,
    "error": null
  },
  "runtime": {
    "model": "",
    "base_url": "",
    "workspace_root": "",
    "workspace_brief": "",
    "guidance_snapshot": null
  },
  "goal_state": {},
  "runtime_task": {
    "current": null,
    "history": []
  },
  "hierarchy": null,
  "turn_summaries": {},
  "applied_summaries": [],
  "evidence": {
    "entries": [],
    "claims": []
  },
  "artifacts": []
}
```

Rules:

- `session.json` stores compact state and references, not raw bulky output.
- Fields that grow without bound should use bounded history or artifact references.
- Every writer must update this file through one helper so merge behavior is consistent and atomic.

### `thinking.jsonl`

Append-only JSON Lines reasoning stream.

Rules:

- Create only when thinking/reasoning is persisted.
- Store turn id, timestamp, and content.
- Do not use it as reload-critical state.

### `summaries/`

Markdown artifacts from Task 385.

Rules:

- Store finalized summary markdown files with frontmatter.
- `session.json.turn_summaries` should store compact structured state needed for compaction/reload and may reference the markdown file.
- Avoid storing the same long summary body in both `session.json` and `summaries/*.md` unless it is required for active compaction.

### `artifacts/`

Single location for large or non-chat payloads.

Allowed examples:

- Full shell/tool output too large for `session.md`.
- Snapshots.
- Generated scripts.
- Raw documents or extraction payloads.
- Debug display captures only when a debug/trace flag explicitly requests them.

Disallowed examples:

- Routine final answer duplicates.
- Routine user prompt duplicates.
- A second terminal transcript.
- A separate `tool-results/` root folder.

## Implementation Plan

### Phase 1: Centralize Session State Writes

Add a small session document helper in `src/session_write.rs` or a new focused module:

- `load_session_doc(session_root) -> serde_json::Value`
- `mutate_session_doc(session_root, FnOnce(&mut Value)) -> Result<PathBuf>`
- Atomic write via temp file and rename.
- Schema version initialization.
- Preserve unknown fields for forward compatibility.

Refactor these writers to use the helper:

- `save_goal_state`
- `save_turn_summary`
- `mark_summary_applied`
- `write_session_error`
- `write_session_status`
- `persist_workspace_intel`
- `persist_guidance_snapshot`
- `save_runtime_task_record`
- `advance_runtime_task_stage`
- `finalize_runtime_task`
- `save_hierarchy`
- `save_hierarchy_progress`
- `EvidenceLedger::persist`

### Phase 2: Make `session.md` The Transcript Source

Replace durable transcript/display duplication with one markdown append path:

- Add `append_session_markdown(session, entry)` with typed entry variants.
- Wire `TerminalUI`/Claude renderer message pushes to append to `session.md`.
- Update `session_flush::append_to_transcript` to append to `session.md` or remove it if superseded.
- Stop writing `display/terminal_transcript.txt`.
- Stop writing `artifacts/terminal_transcript.txt`.
- Stop writing numbered user prompt/final answer display captures by default.

Keep the terminal UI rendering unchanged; this is storage consolidation only.

### Phase 3: Consolidate Dedicated Folders

Move or retarget current per-feature folders:

- `tool_result_storage.rs`: write persisted large outputs under `artifacts/tool-results/` or `artifacts/tool_{id}.txt`, not root `tool-results/`.
- `session_hierarchy.rs`: store compact hierarchy/progress under `session.json.hierarchy`.
- `runtime_task.rs`: store current runtime task and bounded history under `session.json.runtime_task`.
- `session_error.rs`: store status and last error under `session.json.status`.
- `app_bootstrap_core.rs` and `app_chat_helpers.rs`: store workspace facts/brief under `session.json.runtime`.
- `project_guidance.rs`: store guidance snapshot under `session.json.runtime.guidance_snapshot` unless the full source must remain external.
- `evidence_ledger.rs`: store compact ledger entries under `session.json.evidence`; put raw evidence in `artifacts/` only when large.

### Phase 4: Backward-Compatible Loading

New sessions must not create legacy paths, but loaders should tolerate old sessions:

- If `session.md` is missing, read legacy `display/terminal_transcript.txt` or `artifacts/terminal_transcript.txt` for display/resume.
- If `session.json.status` is missing, read `session_status.json`.
- If `session.json.status.error` is missing, read `error.json`.
- If `session.json.runtime_task.current` is missing, read `runtime_tasks/latest.json`.
- If `session.json.hierarchy` is missing, read `hierarchy/*.json`.
- If artifact references point to root `tool-results/`, keep reading them.

Do not rewrite old sessions automatically.

### Phase 5: Index And Cleanup

Update `session_index.rs`:

- `transcript_path` should point to `s_{id}/session.md`.
- `status` should read from `session.json.status.state`.
- Artifact count should count only `artifacts/` files.
- Index rebuild should not expect `display/`.

Update `session_paths.rs`:

- Document the new minimal layout including `summaries/`.
- Tests should assert that new session creation does not create `display/`, `hierarchy/`, `runtime_tasks/`, `tool-results/`, or `evidence/`.
- `artifacts/`, `summaries/`, and `thinking.jsonl` may be lazy-created instead of always present if that keeps empty sessions slimmer.

## Files To Audit

| File | Reason |
|------|--------|
| `src/session_paths.rs` | Session layout contract and tests |
| `src/session_write.rs` | Canonical JSON and markdown write helpers |
| `src/session_display.rs` | Numbered display capture duplication |
| `src/session_flush.rs` | Legacy display transcript path |
| `src/session_index.rs` | Resume/index transcript and status lookup |
| `src/session_error.rs` | Status/error JSON consolidation |
| `src/session_hierarchy.rs` | Multi-file hierarchy consolidation |
| `src/runtime_task.rs` | Runtime task folder consolidation |
| `src/tool_result_storage.rs` | Root `tool-results/` consolidation |
| `src/evidence_ledger.rs` | Nested evidence folder consolidation |
| `src/app_bootstrap_core.rs` | Workspace metadata persistence |
| `src/app_chat_helpers.rs` | Workspace metadata persistence refresh |
| `src/project_guidance.rs` | Guidance snapshot persistence |
| `src/claude_ui/claude_render.rs` | Terminal transcript duplication |

## Success Criteria

- [ ] A new normal chat session creates `session.md` and `session.json`, with no legacy root files.
- [ ] A session with finalized summaries creates `summaries/*.md`.
- [ ] A session with large tool output stores raw payloads only under `artifacts/`.
- [ ] No new session creates `display/`, `hierarchy/`, `runtime_tasks/`, root `tool-results/`, root `workspace.txt`, root `workspace_brief.txt`, `session_status.json`, or `error.json`.
- [ ] Reload/resume reads `session.md` plus `session.json` for new sessions.
- [ ] Legacy sessions remain readable without automatic mutation.
- [ ] Session index points to `session.md` and reads status from `session.json`.
- [ ] Existing transcript-native operational visibility remains visible to the user.
- [ ] No behavior change to routing, tool selection, permission gates, or final answer generation.

## Verification

```bash
cargo build
cargo test session
cargo test runtime_task
cargo test evidence_ledger
cargo test transcript
```

Manual smoke:

1. Start a new session and send a plain chat message.
2. Verify the session folder contains only `session.md`, `session.json`, and any lazily required files/folders.
3. Run a command with short output and verify it appears once in `session.md`.
4. Run a command with large output and verify the transcript contains a short artifact reference and the raw output exists only in `artifacts/`.
5. Trigger a turn summary and verify `summaries/*.md` exists without duplicating the full body unnecessarily in multiple places.
6. Rebuild the session index and verify it points to `session.md`.
7. Open an older session and verify legacy loading still works.

## Anti-Patterns To Avoid

- Do not create another parallel transcript file.
- Do not store the same large content in both markdown and JSON.
- Do not scatter state into feature-specific root folders.
- Do not make debug traces a default user-facing session artifact.
- Do not solve this by deleting persistence that is needed for reload, crash recovery, or evidence grounding.
