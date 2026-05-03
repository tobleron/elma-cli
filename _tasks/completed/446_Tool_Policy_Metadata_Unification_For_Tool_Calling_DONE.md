# Task 446: Tool Policy Metadata Unification For Tool Calling

**Status:** completed

**Note:** Phase 1 (registry metadata) is complete. Phases 2-4 deferred to future tasks.
**Priority:** HIGH
**Estimated effort:** 3-5 days
**Depends on:** completed Task 393, Task 441, Task 443, Task 444, Task 445
**References:** `elma-tools/src/registry.rs`, `src/tool_registry.rs`, `src/tool_calling.rs`, `src/streaming_tool_executor.rs`, `src/permission_gate.rs`, `src/safe_mode.rs`

## Problem

Elma has a dynamic tool registry with useful metadata:

- implementation kind
- workspace-scoped flag
- shell equivalents
- availability checks
- deferred/default loading

But policy decisions still live in scattered code paths:

- Permission behavior is in `permission_gate.rs`, `safe_mode.rs`, shell preflight, and tool executors.
- Concurrency safety is represented on older `StepCommon` fields and ad hoc scheduler logic.
- Prior-read requirements for edits are enforced at execution time but not declared at the registry boundary.
- Network/external-process risk is implicit in individual tools.
- Native-over-shell preference exists as metadata, but not all policy consumers use the same source.

This makes it easy for tool exposure, permission gates, and scheduling to drift.

## Objective

Extend the current `ToolDefinitionExt` registry contract so every model-callable tool has explicit policy metadata that can be consumed by:

- tool declaration/exposure
- permission gates
- safe mode
- parallel execution
- stale-read and workspace policy tasks as downstream consumers
- transcript-visible tool policy rows

This is the tool-calling equivalent of the old action/tool metadata idea, without any DSL action layer.

## Non-Goals

- Do not reintroduce `AgentAction` or DSL command policies.
- Do not route user requests by keyword.
- Do not bypass existing permission checks while migrating metadata.
- Do not make network tools default-enabled.
- Do not add policy text to `TOOL_CALLING_SYSTEM_PROMPT`.

## Design

Extend `elma-tools/src/registry.rs` with explicit policy metadata.

Suggested types:

```rust
pub enum ToolRisk {
    ReadOnly,
    WorkspaceWrite,
    ExternalProcess,
    Network,
    ConversationState,
    DestructivePotential,
}

pub enum ExecutorState {
    PureRust,
    RustWithSystemDependency,
    ShellBacked,
    NetworkBacked,
    ExtensionBacked,
}

pub struct ToolPolicy {
    pub risks: Vec<ToolRisk>,
    pub executor_state: ExecutorState,
    pub requires_permission: bool,
    pub requires_prior_read: bool,
    pub workspace_scoped: bool,
    pub concurrency_safe: bool,
    pub creates_artifacts: bool,
    pub mutates_workspace: bool,
}
```

Reuse existing fields where possible instead of duplicating them. The public shape can differ, but each policy concept must have one canonical source.

## Implementation Completed

### Phase 1: Registry Metadata ✓
1. **Policy types added** to `elma-tools/src/registry.rs`:
   - `ToolRisk` enum (ReadOnly, WorkspaceWrite, ExternalProcess, Network, ConversationState, DestructivePotential)
   - `ExecutorState` enum (PureRust, RustWithSystemDependency, ShellBacked, NetworkBacked, ExtensionBacked)  
   - `ToolPolicy` struct with all policy fields

2. **Builder methods added** to `ToolDefinitionExt`:
   - `with_policy()`, `with_risks()`, `requires_permission()`, `requires_prior_read()`, `concurrency_safe()`, `with_executor_state()`, `mutates_workspace()`, `creates_artifacts()`

3. **All tools annotated with policy metadata**:
   - read, write, edit, shell, search, glob, patch, observe, respond, summary, todo, ls, fetch, tool_search

4. **Tests added** for policy coverage

### Remaining (absorbed into existing tasks)
- **Phase 2 (Policy adapter)**: Absorbed into Task 448 (Model Capability Registry) — adapter should be part of capability budget
- **Phase 3 (Consumer integration)**: Absorbed into Task 471 (Tool Calling Certification) — verify policy metadata is used
- **Phase 4 (Transcript visibility)**: Absorbed into Task 470 (Action Observation Event Log) — add policy decision events

## Success Criteria

- [x] Policy types and fields exists in registry
- [x] Builder methods available for tool registration
- [x] Every model-callable tool has complete policy metadata
- [ ] Policy adapter in main crate (deferred)
- [ ] Consumer integration (deferred)

## Verification

```bash
cargo build
cargo test tool_registry
cargo test tool_calling
cargo test permission_gate
cargo test safe_mode
cargo test streaming_tool_executor
```

Manual smoke:

1. Ask for a read/search task and verify read-only tools can run without permission prompts.
2. Ask for an edit and verify prior-read/workspace-write policy is enforced.
3. Ask for a shell command that needs permission and verify the prompt names the policy reason.
4. Ask for independent metadata/read tasks and verify only policy-safe tools can parallelize.

## Anti-Patterns To Avoid

- Do not use tool-name string checks where registry metadata is available.
- Do not trust policy metadata as the only safety layer for destructive operations.
- Do not make policy metadata user-message dependent.
- Do not hide policy blocks in trace-only logs.
