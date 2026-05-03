# 565 — Standardize Tool Result Envelope

- **Priority**: Medium
- **Category**: Tool Calling
- **Depends on**: 552 (split tool_calling.rs)
- **Blocks**: None

## Problem Statement

Tool execution results are currently represented as `ToolExecutionResult`:

```rust
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: String,       // mixed content + error messages
    pub ok: bool,              // binary success/failure
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub signal_killed: Option<i32>,
}
```

Problems:
1. **Missing fields**: No `stderr` separate from `content`, no structured metadata
2. **Mixed content**: Error messages and output are combined in `content`
3. **Binary success**: `ok: bool` loses nuance (partial success, warning, empty result)
4. **Inconsistent usage**: Some tools set `exit_code`, most leave it `None`
5. **No size metadata**: Callers can't know result size without scanning `content`

The `docs/_proposals/007-structured-tool-result-envelope.md` proposal was accepted, advocating for adding `exit_code`, `timed_out`, `signal_killed` — these fields were added but the proposal envisioned more structure.

## Why This Matters for Small Local LLMs

Small models need clear, structured feedback about what happened:
- "This command succeeded" vs "This command succeeded but returned no output" vs "This command failed with exit code 1"
- The model needs to know WHY a tool failed to formulate a recovery strategy
- Context budget management needs to know result size to decide whether to truncate

## Current Behavior

```rust
// Read tool: content is file contents (or error)
ToolExecutionResult { ok: true, content: file_contents, exit_code: None, ... }

// Shell tool failure: content mixes error info
ToolExecutionResult { ok: false, content: "Command failed (exit code 1):\n...", exit_code: Some(1), ... }

// Shell tool success: content is command output
ToolExecutionResult { ok: true, content: command_output, exit_code: Some(0), ... }
```

The model sees `content` and has to parse whether it's an error or result from the text itself.

## Recommended Target Behavior

```rust
pub struct ToolExecutionResult {
    pub tool_call_id: String,
    pub tool_name: String,
    
    // Structured status
    pub status: ToolStatus,
    
    // Separate output channels
    pub output: Option<String>,    // successful output (None = no output)
    pub error: Option<ToolError>,  // structured error (None = success)
    
    // Execution metadata
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub signal_killed: Option<i32>,
    pub duration_ms: u64,
    pub output_size_bytes: usize,
    
    // Model-facing summary (auto-generated or explicit)
    pub model_facing_summary: String,
}

pub enum ToolStatus {
    Success,
    SuccessEmpty,           // completed but no output
    PartialSuccess,         // completed with warnings
    Failed,                 // execution error
    Blocked,                // preflight/permission blocked
    TimedOut,
    NotFound,               // tool not found
}
```

### Model-Facing Summary

Instead of the model seeing raw output, each result includes a context-aware summary:

```
Tool: read
Status: Success
Summary: Read 45 lines from src/main.rs (3.2 KB)
First 200 chars: use std::collections::HashMap; ...
[Full output available via evidence ledger entry e_042]
```

## Source Files That Need Modification

- `src/tool_calling.rs` → `src/tools/mod.rs` — Update `ToolExecutionResult`
- `src/tool_loop.rs` — Update result handling, model-facing message construction
- `src/tool_result_storage.rs` — Update budget/caching based on new structure
- `src/evidence_ledger.rs` — Update evidence entry construction
- `src/event_log.rs` — Update tool event recording
- All 30+ tool executors — Update to populate new fields

## New Files/Modules

- `src/tool_result.rs` — `ToolExecutionResult`, `ToolStatus` enum, result formatting

## Step-by-Step Implementation Plan

1. Create `src/tool_result.rs` with new types
2. Add `impl From<OldToolExecutionResult> for ToolExecutionResult` for backward compat
3. Add `fn to_model_message(&self) -> String` that generates the model-facing summary
4. Update `tool_loop.rs` to use `to_model_message()` when injecting tool results into conversation
5. Migrate tool executors one at a time:
   a. Set `status` instead of `ok`
   b. Set `error` for failures instead of embedding in `content`
   c. Set `output` for success instead of `content`
6. Update `evidence_ledger.rs` to use structured data
7. Update `stop_policy.rs` to match on `ToolStatus` enum
8. Remove old `ok` field and `content` field
9. Run full test suite

## Recommended Crates

- `humansize` — already a dependency; for size formatting in summaries

## Validation/Sanitization Strategy

- `output` field is size-capped (configurable max, default 100KB)
- `error` field never contains raw shell output (use `model_facing_summary` for that)
- `model_facing_summary` is always populated (auto-generated if tool doesn't provide one)

## Testing Plan

1. Unit test `to_model_message()` for each `ToolStatus` variant
2. Integration test: verify model receives structured result message
3. Test that evidence ledger still works with new structure
4. Test that stop policy correctly classifies new status variants
5. Test backward compatibility during migration

## Acceptance Criteria

- `ToolExecutionResult` has separate `output` and `error` fields
- `ToolStatus` enum replaces `ok: bool`
- All 30+ tool executors populate the new fields correctly
- Model-facing summary is always populated
- Evidence ledger and event log work with new structure
- Stop policy matches on `ToolStatus` instead of string parsing

## Risks and Migration Notes

- **Very high touch surface**: Every tool executor and every consumer of `ToolExecutionResult`. Do incrementally.
- **Model prompt impact**: Changing how tool results are presented to the model will change behavior. Run scenario tests before/after.
- **Size budget**: Adding structured metadata to every result increases memory usage. Keep metadata compact.
- Build on Task 552 (split tool_calling.rs) so each executor file can be updated independently.
