# 564 — Normalize Error Types Across All Modules

- **Priority**: Medium
- **Category**: Error Handling
- **Depends on**: 554 (session-scoped state for error context)
- **Blocks**: None

## Problem Statement

The codebase uses inconsistent error handling patterns:

1. **`anyhow::Result`** — Used broadly with `.context()` and `anyhow::anyhow!()` for ad-hoc errors
2. **`miette::IntoDiagnostic`** — Imported but barely used outside main
3. **String errors** — Many functions return `Result<T, String>` or embed error info in return values
4. **`ToolExecutionResult.ok`** — Boolean success/failure with string content for errors
5. **`thiserror`** — Listed as dependency but minimally used for custom error types
6. **Silent fallbacks** — `unwrap_or_default()`, `unwrap_or_else(|_| default)`, `.ok()` that swallow errors

Examples of inconsistency:
```rust
// String errors (tool_calling.rs)
content: format!("Error parsing arguments: {} | raw: {}", e, detail),

// anyhow errors (orchestration_core.rs)
Err(anyhow::anyhow!("Orchestrator failed after {} attempts", MAX_RETRIES + 1))

// Boolean + content (tool_calling.rs)
ToolExecutionResult { ok: false, content: "Command blocked...", ... }

// Silent fallback (intel_narrative_steps.rs)
let complexity: ComplexityAssessment = serde_json::from_value(output.data.clone())
    .unwrap_or_else(|_| ComplexityAssessment::default());
```

## Why This Matters for Small Local LLMs

Small models produce more errors. When errors are stringly-typed:
- The model can't programmatically determine how to recover
- Repair prompts can't be generated from structured error data
- Error classification for stop policy is fragile (regex matching on error strings)
- The stop policy's `classify_error()` function (`stop_policy.rs:672-691`) has to parse error strings

## Current Behavior

Error handling patterns by module:
| Pattern | Used In | Problems |
|---------|---------|---------|
| `anyhow::Result` | orchestration, llm_provider, tool_loop | Generic, no typed recovery info |
| `Result<T, String>` | tool_calling, shell_preflight | No source tracking, no stack context |
| `ToolExecutionResult.ok` | All tool executors | Binary ok/fail loses error classification |
| `Option<T>` with fallback | intel_narrative, routing_parse | Silent failures hide model output problems |
| `unwrap_or_default()` | ~50 call sites | Data loss on parse failure |

## Recommended Target Behavior

Create a structured error hierarchy:

```rust
// Top-level error enum
#[derive(Debug, Error)]
pub enum ElmaError {
    #[error("Tool execution failed: {0}")]
    Tool(#[from] ToolError),
    
    #[error("Model response error: {0}")]
    Model(#[from] ModelError),
    
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] JsonParseError),
    
    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    
    #[error("Stop policy: {0}")]
    StopPolicy(StopReason),
    
    #[error("Session error: {0}")]
    Session(#[from] SessionError),
    
    #[error("Internal error: {0}")]
    Internal(#[from] anyhow::Error),
}

// Structured tool error
#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Invalid arguments: {field}: {reason}")]
    InvalidArgs { field: String, reason: String },
    
    #[error("Command blocked by preflight: {reason}")]
    PreflightBlocked { reason: String },
    
    #[error("Permission denied")]
    PermissionDenied,
    
    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },
    
    #[error("Execution failed: exit_code={exit_code}, signal={signal:?}")]
    ExecutionFailed { exit_code: Option<i32>, signal: Option<i32>, stderr: String },
    
    #[error("Timed out after {duration}s")]
    Timeout { duration: u64 },
}
```

## Source Files That Need Modification

- `src/errors.rs` (new) — Error type definitions
- `src/tool_calling.rs` — Replace String errors and boolean ok with typed errors
- `src/orchestration_core.rs` — Replace anyhow with typed errors where possible
- `src/tool_loop.rs` — Update error handling
- `src/stop_policy.rs` — Replace `classify_error()` with match on typed error
- `src/shell_preflight.rs` — Replace String returns with typed errors
- `src/intel_trait.rs` — Replace ad-hoc error handling
- All modules with `unwrap_or_default()` on parse results

## New Files/Modules

- `src/errors.rs` — `ElmaError`, `ToolError`, `ModelError`, `JsonParseError`, `ValidationError`, `SessionError`

## Step-by-Step Implementation Plan

1. Create `src/errors.rs` with error type hierarchy
2. Add `impl From<ToolError> for ToolExecutionResult` for backward compatibility
3. Migrate one module at a time, starting with the simplest:
   a. `shell_preflight.rs` — String → `ValidationError`
   b. `tool_calling.rs` — String + bool → `ToolError`
   c. `stop_policy.rs` — String matching → typed error matching
   d. `orchestration_core.rs` — anyhow → ElmaError
   e. `tool_loop.rs` — anyhow → ElmaError
4. For each `unwrap_or_default()` on model output: replace with proper error handling (log + structured error to model)
5. Add `Into<ModelGuidance>` for each error variant (generates repair prompts)
6. Run `cargo test` after each module migration

## Recommended Crates

- `thiserror` — already a dependency; use for derive macros

## Validation/Sanitization Strategy

- All errors implement `Display` and `Error`
- All errors are `Send + Sync`
- Error messages intended for the model are tagged separately from developer errors
- Error serialization preserves structured data (not just Display)

## Testing Plan

1. Test that each error variant produces correct Display output
2. Test error conversion chains (From impls)
3. Test that model-facing error messages don't leak internal paths/URLs
4. Test that error classification in stop policy works with typed errors
5. Snapshot tests for error messages (catch unintended changes)

## Acceptance Criteria

- All modules use typed errors from `src/errors.rs` (no more String errors)
- `classify_error()` in stop_policy matches on typed errors, not strings
- No silent fallbacks on model output parse failures (log+report instead)
- `ToolExecutionResult.ok` is removed or backed by typed error
- Error messages intended for models are distinct from developer-facing errors

## Risks and Migration Notes

- **Very high touch surface**: ~100+ error sites. Do this incrementally, module by module.
- **API breakage**: `ToolExecutionResult` is used widely. Add `impl From<ToolError>` first, then migrate callers.
- **anyhow is still useful**: Keep `anyhow` for truly unexpected errors (IO failures in non-critical paths). Reserve typed errors for recoverable/classifiable errors.
- This task is lower priority than foundational tasks (550-554) but higher than polish tasks.
