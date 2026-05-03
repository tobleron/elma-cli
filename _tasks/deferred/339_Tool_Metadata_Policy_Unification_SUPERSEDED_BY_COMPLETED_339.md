# Task 339: Action And Tool Metadata Policy Unification

**Status:** deferred/superseded
**Deferred reason:** duplicate pending copy; completed implementation lives in `_tasks/completed/339_Action_And_Tool_Metadata_Policy_Unification_DONE.md`.
**Priority:** critical
**Primary surfaces:** `src/agent_protocol/*`, `src/tool_registry.rs`, `src/tool_calling.rs`, `src/streaming_tool_executor.rs`, `elma-tools/src/registry.rs`
**Depends on:** Task 377 (DSL parser/error model), Task 338 (formal action-observation event log) if event rendering is implemented first
**Reframed by:** Tasks 378 and 379

## Objective

Make action/tool policy explicit, testable, and centralized. Every DSL action and any remaining internal tool adapter must declare what it can do, whether it is executable, and how the runtime should gate, schedule, and display it.

The primary model-facing surface after the DSL migration is `AgentAction`, not provider-native function/tool calling. `elma-tools` metadata remains useful for compatibility, internal adapters, and certification while those paths still exist, but it must not preserve JSON model-output as the long-term action protocol.

## Current Code Reality

- `elma-tools::ToolDefinitionExt` currently stores `search_hints`, `deferred`, and `check_fn`.
- It does not store read/write/network/destructive/concurrency/permission metadata.
- `src/types_core.rs::StepCommon` has similar fields for program steps, but tool-call definitions do not.
- `src/streaming_tool_executor.rs::is_concurrency_safe` uses hardcoded tool names.
- `src/tool_calling.rs::execute_tool_call` supports only `tool_search`, `shell`, `read`, `search`, `respond`, `summary`, and `update_todo_list`.
- Several registered non-deferred tools are currently not executable (`edit`, `patch`, `fetch`, `glob`, `ls`, `write` depending on executor state).
- Task 378 will retire provider-native model tool calls from the live action loop and route the model through compact DSL actions.
- Task 379 owns path, command, edit, and shell safety for the model-facing DSL.

## Design Requirements

### Policy Types

Add model-action policy metadata in the main crate for `AgentAction`, and keep `elma-tools` policy metadata only for compatibility/adapters while that crate remains in use.

Suggested shape:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolRisk {
    ReadOnly,
    WorkspaceWrite,
    Destructive,
    Network,
    ExternalProcess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ToolExecutorState {
    Executable,
    DeclarationOnly,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolPolicy {
    pub executor_state: ToolExecutorState,
    pub risks: Vec<ToolRisk>,
    pub requires_permission: bool,
    pub requires_network_enablement: bool,
    pub requires_prior_read: bool,
    pub workspace_scoped: bool,
    pub concurrency_safe: bool,
    pub interrupt_behavior: ToolInterruptBehavior,
}
```

The main crate should have an equivalent or shared action-focused shape, for example `ActionPolicy`, that can represent:

- action kind: read, list, search, symbol search, exact edit, verification command, ask, done
- risk: read-only, workspace write, external process, network, conversation state
- concurrency behavior
- permission behavior
- evidence/transcript behavior
- whether prior file read is required
- whether the action is model-callable, internal-only, disabled, or compatibility-only

Define `ToolInterruptBehavior` in `elma-tools` and map it to `src/types_core.rs::InterruptBehavior` in the main crate.

### Registry Behavior

The model-facing registry must expose DSL actions, not JSON function definitions. Any compatibility registry must not expose declaration-only or disabled tools as callable function definitions.

Rules:

- the live action loop receives exactly one parsed DSL action
- no provider-native function/tool-call schema is advertised as the preferred model-output protocol
- `default_tools()` returns only available executable non-deferred tools.
- `get_tools()` returns only available executable tools.
- `search_and_convert()` may mention unavailable/declaration-only tools only as search results text if the UX explicitly distinguishes them from callable tools.
- `tool_search` must not say "These tools are now loaded and available" for declaration-only tools.

### Executor Validation

Add runtime validation tests that every model-callable DSL action has a matching executor arm and every compatibility `Executable` tool has a matching executor arm or executor registration.

This should fail today until either:

- missing executors are implemented, or
- those tools are marked `DeclarationOnly`/`Disabled`.

### Permission And Scheduling

Refactor consumers to ask metadata instead of matching hardcoded names:

- `X` command permission risk remains based on command preflight, but the action policy declares external-process risk
- future fetch/browser/MCP adapters declare network risk and are disabled unless explicitly enabled
- `E` declares workspace write risk, requires prior read, and is not concurrency-safe
- `R`, `L`, `S`, and `Y` declare read-only behavior and may be concurrency-safe where implementation supports it
- `ASK` and `DONE` are serial because they affect conversation/finalization state
- legacy respond/summary/todo adapters remain serial if retained as internal UI state actions

### Event/Transcript Visibility

When policy changes execution behavior, surface it through tool result text now and Task 338 event rows later:

- disabled tool
- permission required
- network not enabled
- declaration-only tool discovered
- unsafe tool forced to serial execution

Do not add these details to the bottom status bar.

## Implementation Steps

1. Add `ActionPolicy` metadata for every `AgentAction` variant.
2. Add compatibility metadata types and builder methods to `elma-tools/src/registry.rs` only for remaining non-DSL adapters.
3. Update every remaining tool module under `elma-tools/src/tools/` with explicit compatibility policy or mark it obsolete.
4. Add an executable-action list in the DSL dispatcher that can be tested against action policy.
5. Update `tool_search` or its DSL-era replacement so non-executable capabilities are not advertised as callable commands.
6. Replace `streaming_tool_executor::is_concurrency_safe` with a metadata-backed lookup for actions and adapters.
7. Add a compatibility mapping from `ToolInterruptBehavior` to runtime `InterruptBehavior` where old tool metadata still exists.
8. Add tests for default actions, disabled actions/adapters, compatibility-only tools, and executor mismatch.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools tool_registry
cargo test agent_protocol
cargo test tool_registry
cargo test tool_loop
cargo test streaming_tool_executor
cargo build
```

Required coverage:

- every `AgentAction` variant has non-default policy metadata
- every model-callable action has an executor
- every tool has non-default policy metadata
- executable tools appear in default/current tool definitions when available
- disabled tools do not appear in callable definitions
- declaration-only tools are not marked loaded by `tool_search`
- registry rejects or test-fails executable tools without executor support
- `R`/`L`/`S`/`Y` metadata marks concurrency safe where implementation supports it
- respond/update_todo_list metadata keeps them serial
- `E` metadata marks workspace write and requires prior read
- fetch/browser metadata marks network and requires explicit enablement
- `X` metadata marks external process and permission-aware
- existing prompt-cache stable ordering tests still pass

Manual probe:

```bash
rg -n 'ActionPolicy|ToolPolicy|ToolExecutorState|is_concurrency_safe|DeclarationOnly|Executable|AgentAction' elma-tools/src src
```

The probe must show metadata use and no remaining name-only scheduler policy except tests or compatibility shims.

## Done Criteria

- DSL action exposure cannot drift away from executor support.
- Compatibility tool exposure cannot drift away from executor support.
- Scheduling and permission decisions use metadata.
- All actions and remaining tools declare policy.
- Hidden/deferred/disabled states are tested.
- Source prompt remains untouched.

## Anti-Patterns

- Do not infer risk from tool names once metadata exists.
- Do not expose actions/tools as callable before executors are wired.
- Do not weaken shell command preflight.
- Do not encode main-crate runtime types directly into `elma-tools`.
