# 151 Tool Interface Pattern Refactor

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
Refactor tools to use consistent interface pattern like OpenCode.

## Reference
- OpenCode: `internal/llm/tools/bash.go` - implements `BaseTool` interface

## Implementation

### 1. Tool Interface
File: `src/tools/trait.rs` (new)
```rust
pub trait Tool: Send + Sync {
    fn info(&self) -> ToolInfo;
    fn run(&self, ctx: Context, call: ToolCall) -> Result<ToolResponse>;
}

pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters: Schema,
}

pub struct ToolResponse {
    pub content: String,
    pub is_error: bool,
}
```

### 2. Register Tools
File: `src/tools/mod.rs`
- Implement `Tool` trait for all tools
- Register in `tool_registry()`

### 3. Permission Hook
- Each tool checks permissions in `run()`
- Return error if denied

### 4. LSP Notification
- After edits, notify LSP client
- File: `src/lsp/mod.rs`

## Verification
- [ ] `cargo build` passes
- [ ] All tools implement trait
- [ ] Permission checks work