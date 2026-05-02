# 155 LSP Client Integration

## Backlog Reconciliation (2026-05-02)

Superseded by Task 464. Do not implement LSP before the repo-map baseline in Task 463 and stale-file behavior in Task 456 are available.


## Continuation Checklist
- [ ] Re-read this task and all linked source/task references before editing.
- [ ] Confirm the task is still valid against current `_tasks/TASKS.md`, `AGENTS.md`, and active master plans.
- [ ] Move or keep this task in `_tasks/active/` before implementation work begins.
- [ ] Inspect the current code/config/docs touched by this task and note any drift from the written plan.
- [ ] Implement the smallest coherent change set that satisfies the next unchecked item.
- [ ] Add or update focused tests, probes, fixtures, or snapshots for the changed behavior.
- [ ] Run `cargo fmt --check` and fix formatting issues.
- [ ] Run `cargo build` and resolve all build errors or warnings introduced by this task.
- [ ] Run targeted `cargo test` commands and any task-specific probes listed below.
- [ ] Run real CLI or pseudo-terminal verification for any user-facing behavior.
- [ ] Record completed work, verification output, and remaining gaps in this task before stopping.
- [ ] Ask for sign-off before moving this task to `_tasks/completed/`.

## Summary
Implement Language Server Protocol client for diagnostics and code intelligence.

## Reference
- OpenCode: `internal/lsp/client.go`, `internal/llm/tools/diagnostics.go`

## Implementation

### 1. LSP Types
File: `src/lsp/types.rs` (new)
- `Position`, `Range`, `Location`
- `Diagnostic`, `PublishDiagnosticsParams`
- `TextDocumentPositionParams`

### 2. LSP Client
File: `src/lsp/client.rs` (new)
```rust
pub struct LspClient {
    process: Child,
    stdin: Mutex<ChildStdin>,
    stdout: Mutex<ChildStdout>,
    pending: Arc<Mutex<HashMap<String, oneshot::Sender<serde_json::Value>>>>,
}
```

### 3. Protocol
- `initialize()` - start LSP server
- `initialized` - server ready notification
- `textDocument/didOpen()` - file opened
- `textDocument/didChange()` - file modified
- `textDocument/diagnostics()` - get diagnostics

### 4. Diagnostics Tool
File: `src/lsp/diagnostics.rs` (new)
- Tool: `diagnostics`
- Returns LSP diagnostics for file
- Shows errors/warnings in UI

### 5. Server Config
File: `config/lsp.toml`
```toml
[[servers]]
name = "rust"
command = "rust-analyzer"
args = []

[[servers]]
name = "ts"
command = "typescript-language-server"
args = ["--stdio"]
```

### 6. Server Management
File: `src/lsp/manager.rs` (new)
- Auto-detect language from file
- Lazy start servers
- Timeout on init

## Verification
- [ ] `cargo build` passes
- [ ] LSP starts correctly
- [ ] Diagnostics returned