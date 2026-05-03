# 579 — Add Offline Capability Detection and Graceful Degradation

- **Priority**: Medium
- **Category**: Architecture
- **Depends on**: None
- **Blocks**: None

## Problem Statement

Elma's philosophy mandates "offline-first behavior by default, with network use only when truly necessary" (AGENTS.md). However, there's no systematic offline capability detection:

1. **Network-dependent tools** (fetch, run_python, run_node, job_start) are always included in tool definitions sent to the model, even when offline
2. **No offline status indicator** in the TUI footer or transcript
3. **Network failures** return generic errors that don't tell the model "you're offline, use local tools"
4. **Prerequisite checks** (`check_fn` on tools) exist but aren't consistently used for network availability

## Why This Matters for Small Local LLMs

Small models need explicit guidance about what tools are available. If `fetch` is listed but fails on every attempt, the model wastes iterations trying to use it. Offline detection should:
1. Remove network tools from the available toolset when offline
2. Inform the model clearly that it's in offline mode
3. Surface offline status in the TUI

## Current Behavior

Tools like `fetch` are defined with `ImplementationKind::Network` and `not_workspace_scoped()`, but there's no runtime check for network availability. The tool is always sent to the model.

The `check_fn` field on `ToolDefinitionExt` supports prerequisite checks but is only used for system binary availability (e.g., `python3`, `node`), not network connectivity.

## Recommended Target Behavior

1. Add `NetworkStatus` detection:
   ```rust
   pub enum NetworkStatus {
       Online,
       Offline,
       Unknown,  // couldn't determine
   }
   
   pub fn detect_network_status() -> NetworkStatus {
       // Try DNS resolution, HTTP HEAD to known endpoint, or check interface status
   }
   ```
2. Filter tools based on network status:
   - Offline → remove `ImplementationKind::Network` tools from available set
   - Show model a system message: "Network access is unavailable. Use local tools only."
3. Add offline indicator to TUI footer (subtle, per Rule 5)
4. Add graceful degradation: if network drops mid-session, tools start failing with clear offline errors

## Source Files That Need Modification

- `src/tool_registry.rs` — Add network-aware tool filtering
- `elma-tools/src/registry.rs` — Add `requires_network()` to `ToolDefinitionExt`
- `src/llm_provider.rs` — Add network status detection
- `src/prompt_core.rs` or system prompt assembly — Add offline mode notice
- TUI footer — Add offline indicator

## Acceptance Criteria

- Network tools filtered when offline
- Model receives clear offline notice
- Network status detected at startup and monitored during session
- TUI shows offline status

## Risks and Migration Notes

- Network detection can be unreliable (firewall blocking ICMP but allowing HTTP). Use multiple detection methods.
- Some users may want to use network tools even with unreliable connectivity. Add `--force-online` flag.
