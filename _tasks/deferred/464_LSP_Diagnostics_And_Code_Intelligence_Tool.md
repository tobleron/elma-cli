# Task 464: LSP Diagnostics And Code Intelligence Tool

**Status:** pending
**Priority:** medium-high
**Primary surfaces:** `elma-tools/src/tools/`, `src/tool_calling.rs`, new `src/lsp_manager.rs`
**Depends on:** Task 456 (file context tracker) for best stale-file behavior
**Related tasks:** Task 463 (symbol-aware repo map), completed Task 339 (tool metadata policy), completed Task 362 (parallel read/search execution)

## Objective

Add an optional, local-only LSP diagnostics capability that helps Elma inspect code health without requiring a full build and without making any language server mandatory.

The first deliverable is diagnostics. Definitions, references, hover, and workspace symbols should be designed for extension but not fully implemented unless the diagnostics path is stable.

## Current Code Reality

- There is no LSP manager module today.
- `which` is already available in dependencies and can be used for language server discovery.
- Tool declarations live in `elma-tools/src/tools/`, while execution happens in `src/tool_calling.rs`.
- `src/streaming_tool_executor.rs` currently treats read/search/respond as concurrency-safe by name; LSP must eventually use metadata instead.
- File reading and document adaptation already exist, but diagnostics should not reuse document adapters for source parsing.

## Design Requirements

### Scope For This Task

Implement only:

- optional language server discovery
- lifecycle for starting/stopping one server per workspace/language
- `lsp_diagnostics` tool
- structured diagnostic output for a file or workspace
- graceful unavailable/degraded behavior

Do not implement code actions, auto-fixes, or model-driven refactors in this task.

### Tool Schema

Add `elma-tools/src/tools/lsp_diagnostics.rs`.

Inputs:

- `path`: optional file path, workspace-relative preferred
- `language`: optional string, for example `rust`
- `timeout_ms`: optional integer, default 3000, max 10000

The tool should be deferred unless at least one supported language server is discoverable or metadata can describe it as unavailable.

### Manager Design

Add `src/lsp_manager.rs` with a small API:

```rust
pub(crate) struct LspManager;

pub(crate) enum LspAvailability {
    Available { language: String, command: String },
    Unavailable { language: String, reason: String },
}

pub(crate) struct Diagnostic {
    pub path: PathBuf,
    pub range: DiagnosticRange,
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub message: String,
    pub source: Option<String>,
}
```

Implementation constraints:

- discover `rust-analyzer` first; add other servers through declarative metadata later
- use `tokio::process::Command`
- communicate with JSON-RPC over stdio
- apply startup, initialize, didOpen/didChange, publishDiagnostics, and shutdown timeouts
- kill child processes on drop or session end
- never block normal chat if startup fails

### Diagnostics Semantics

- For `path`, canonicalize and verify it stays inside workspace.
- If the file was read previously and Task 456 exists, refresh stale state before diagnostics.
- If LSP is unavailable, return `ok=true` with a degraded message only when the request is informational; return `ok=false` if the model explicitly called diagnostics expecting results.
- Include severity, file, line, column, code, source, and message.
- Cap output to a configured diagnostic count, default 100.

### UI And Transcript

LSP startup, unavailability, timeout, and diagnostics summary must be transcript-visible through the existing tool result path. Do not put LSP status in the bottom footer.

## Implementation Steps

1. Add LSP tool declaration in `elma-tools`.
2. Add `src/lsp_manager.rs` with discovery, JSON-RPC framing, startup, request/notification helpers, and shutdown.
3. Add `exec_lsp_diagnostics` in `src/tool_calling.rs`.
4. Add session/workspace-level manager ownership so processes do not leak.
5. Use completed Task 339 metadata: read-only, local-process, not destructive, concurrency-safe only per manager limits.
6. Use completed Task 338 event-log integration, or at minimum structured tool start/result rows now.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools lsp
cargo test lsp_manager
cargo test tool_calling
cargo build
```

Tests must not require `rust-analyzer` to be installed. Use a fake JSON-RPC LSP process fixture for deterministic tests.

Required coverage:

- unavailable server returns clear degraded result
- fake server initialize succeeds
- fake server publishDiagnostics response is parsed
- path outside workspace is rejected
- timeout kills or tears down the child process
- malformed JSON-RPC message returns structured error
- diagnostics output is capped
- manager shutdown cleans up child process
- `execute_tool_call` can dispatch `lsp_diagnostics`

Optional environment-gated probe:

```bash
ELMA_RUN_REAL_LSP_TESTS=1 cargo test lsp_real_rust_analyzer -- --ignored
```

This ignored test may run only when `rust-analyzer` is present.

## Done Criteria

- All non-ignored verification tests pass without installing external language servers.
- No LSP child process leaks after tests.
- Missing LSP support degrades cleanly.
- The tool is not exposed as usable unless its executor is wired.
- No source prompt changes are included.

## Anti-Patterns

- Do not block the main tool loop on unbounded LSP startup.
- Do not make `rust-analyzer` a required dependency for Elma.
- Do not treat LSP diagnostics as a replacement for build/test verification.
- Do not add code-action mutation paths in this task.
