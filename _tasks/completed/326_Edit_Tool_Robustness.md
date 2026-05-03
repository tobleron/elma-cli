# Task 326: Exact Edit Engine Robustness

**Status:** completed — core engine in `src/dsl/safety.rs::apply_exact_edit` with atomic write, stale-read gate, 8MB limit, binary/UTF-8 validation, zero/multiple-match errors, session snapshots. See `src/tool_loop.rs:819-868` for DSL `E` dispatch.
**Priority:** high
**Primary surfaces:** `src/edit_engine.rs`, `src/agent_protocol/*`, `src/execution_steps_edit.rs`, `src/program_utils.rs`
**Depends on:** Task 377 (DSL parser/error model), Task 379 (DSL path/command/edit safety)
**Related tasks:** Task 337 (file context tracker), Task 361 (workspace policy files), Task 378 (action DSL cutover)

## Objective

Make Elma's exact replacement edit path production-grade for the compact action DSL `E` command and any legacy internal edit paths that remain during migration.

The final result must prevent stale writes, ambiguous replacements, unsafe path writes, encoding loss, partial writes, and silent model-facing failure modes. The model must never provide arbitrary JSON edit arguments; it provides only parsed DSL `E` blocks that become typed Rust edit requests after validation.

## Current Code Reality

- Task 378 will replace provider-native model tool calls with `AgentAction::EditFile { path, old, new }`.
- `src/execution_steps_edit.rs` implements `write_file`, `append_text`, and `replace_text` for generated program steps.
- `src/program_utils.rs::resolve_workspace_edit_path` only accepts relative paths and protects the workspace boundary.
- `encoding_rs` and `tempfile` already exist in the root `Cargo.toml`.
- There is no canonical file-context/read-state gate shared by read, edit, write, and patch yet.
- Legacy `elma-tools` edit declarations may remain only as compatibility/adapters until Task 384 removes dead model-output JSON/tool-call code.

## Design Requirements

### Shared Edit Engine

Create a shared implementation module, for example `src/edit_engine.rs`, that owns the actual filesystem mutation semantics. The DSL action dispatcher and `src/execution_steps_edit.rs` must call this module instead of duplicating logic.

The engine should expose explicit request/response types:

```rust
pub(crate) enum EditOperation {
    ReplaceExact { old_string: String, new_string: String },
    WriteFile { content: String, create_only: bool },
    AppendText { content: String },
}

pub(crate) struct EditRequest {
    pub path: String,
    pub operation: EditOperation,
    pub require_prior_read: bool,
}

pub(crate) struct EditOutcome {
    pub path: PathBuf,
    pub files_changed: usize,
    pub bytes_written: usize,
    pub lines_added: usize,
    pub lines_removed: usize,
    pub warning: Option<String>,
}
```

Use structured error variants with stable codes. Do not return only free-form strings from the engine.

### DSL Wiring

Wire `AgentAction::EditFile` in the action DSL dispatcher. It must:

- accept only the already parsed DSL fields: `path`, `old`, `new`
- reject absolute paths before canonicalization
- prefer root-relative paths in all responses
- return compact observations such as `EDIT_OK`, `INVALID_EDIT`, or `UNSAFE_PATH`
- emit transcript/event rows through the action-observation event path

Do not modify `src/prompt_core.rs` for this task.

### Stale Read Gate

If Task 337 is complete, use its file context tracker. If it is not complete, implement the minimal tracker functions required for this task behind a small interface that Task 337 can later expand:

```rust
trait FileReadState {
    fn last_read_fingerprint(&self, path: &Path) -> Option<FileFingerprint>;
    fn record_edit(&self, path: &Path, fingerprint: FileFingerprint);
}
```

Edit operations that modify existing files must fail if the file has not been read in the current session or if its fingerprint changed after the last read. A fingerprint must include at least size, mtime where available, and a content hash for files under the edit size limit.

### Matching Rules

For replace operations:

- exact match is the primary path
- `old_string == new_string` is an error
- zero matches is an error with a short hint
- one match is valid
- multiple matches are always an error for DSL `E`
- report up to five line numbers where matches occur
- tell the model to reread the file before retrying when `OLD` is not found

Whitespace or quote normalization may be offered only as a warning/hint. It must never silently change which bytes are replaced.

### Encoding And Atomic Writes

Implement byte-preserving read/decode/write behavior:

- detect UTF-8, UTF-8 BOM, UTF-16LE BOM, and UTF-16BE BOM
- preserve the original BOM and line endings where practical
- reject unsupported binary input before mutation
- write through a temp file in the same directory and rename into place
- perform a second fingerprint check immediately before commit

The operation must not await or perform unrelated work between final read/fingerprint validation and rename.

### Safety Checks

Add explicit checks for:

- path escapes and symlink escapes after canonicalization
- existing directory targets
- files larger than a configured limit, default 16 MiB for normal edits
- common credential material in `new_string` or full file output
- protected paths once Task 361 lands

Secret detection is a safety guard, not routing logic. Keep patterns conservative and documented in code.

## Implementation Steps

1. Add `src/edit_engine.rs` with pure validation helpers and filesystem mutation functions.
2. Add focused unit tests for the engine before wiring the tool.
3. Wire `AgentAction::EditFile` in the DSL action dispatcher from Task 378.
4. Refactor `src/execution_steps_edit.rs` to use the shared engine while preserving existing internal output shape.
5. Add file-context gating or the minimal compatibility interface described above.
6. Add structured user-facing error formatting.
7. Update action/tool metadata after Task 379 exists: `E` is write-capable, destructive, not concurrency-safe, workspace-filesystem-scoped, and requires prior read.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test agent_protocol
cargo test edit_engine
cargo test execution_steps_edit
cargo test program_utils
cargo test tool_loop
cargo build
```

Required unit/integration coverage:

- exact single replacement succeeds
- `old_string == new_string` fails
- missing `old_string` fails
- multiple matches fail with line hints
- file not read fails
- stale fingerprint fails
- simulated TOCTOU mutation before commit fails
- absolute path fails
- symlink escape fails
- UTF-8 BOM round trip preserves BOM
- UTF-16LE and UTF-16BE round trips preserve encoding
- binary file is rejected
- oversized file is rejected without reading whole content into memory
- secret-like new content is blocked
- `AgentAction::EditFile` returns compact repair observations for invalid edits
- legacy internal `replace_text`, `write_file`, and `append_text` still pass existing behavior tests

Manual probe:

```bash
rg -n 'AgentAction::EditFile|INVALID_EDIT|edit_engine|ReplaceExact' src
```

The probe must show a DSL action dispatcher arm, compact edit errors, and shared engine use from every edit entry point.

## Done Criteria

- All verification commands pass.
- DSL `E` edits execute only through the typed edit engine.
- No edit path can mutate a file that has stale or missing read context.
- No source prompt changes are included.
- Failure output is precise enough for a small model to recover without guessing.

## Anti-Patterns

- Do not silently replace the first match when there are multiple matches.
- Do not rely only on mtime for stale detection.
- Do not hand-roll shell commands for edits.
- Do not store backups beside source files after success.
- Do not add broad fuzzy matching that changes edit semantics.
