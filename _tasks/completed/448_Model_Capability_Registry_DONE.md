# Task 448: Model Capability Registry And Token Budgeting

**Status:** completed
**Priority:** HIGH
**Estimated effort:** 3-5 days
**Depends on:** completed Task 430, completed Task 343 context, completed Task 446 (Phase 2 policy adapter)
**References:** `src/llm_config.rs`, `src/app_bootstrap_core.rs`, `src/auto_compact.rs`, `src/tool_result_storage.rs`, `src/types_core.rs`

## Problem

Elma currently has runtime request caps in `src/llm_config.rs`, profile-level `max_tokens`, a footer context percentage, and several places that estimate context size or clamp output. These limits are not backed by a single model capability contract.

That makes context behavior fragile:

- Unknown model context windows fall back to approximate assumptions.
- Output token caps are profile-local instead of capability-aware.
- Reasoning controls are handled by model behavior probing, runtime defaults, and ad hoc request construction.
- Tool-loop, compaction, finalization, and tool-result persistence can drift from each other.

This task should reintroduce the useful part of the old model-capability registry without reviving DSL-specific code.

## Objective

Add a current-architecture model capability registry that gives Elma one trusted source for:

- context window
- maximum output tokens
- reasoning format support
- tool-calling support
- streaming support
- logprobs support
- tokenizer availability
- provider quirks and safe fallbacks

Use it to make context budgeting, compaction thresholds, output limits, and request construction more predictable.

## Non-Goals

- Do not edit `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT`.
- Do not add DSL grammar or action protocol behavior.
- Do not make tokenizer failure fatal.
- Do not require network access to identify local models.
- Do not put extra capability details in the footer beyond model name, token count, and elapsed time.

## Design

Add a focused module such as `src/model_capabilities.rs`.

Suggested types:

```rust
pub(crate) struct ModelCapabilities {
    pub model_id: String,
    pub provider_family: ProviderFamily,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub supports_tools: CapabilitySupport,
    pub supports_streaming: CapabilitySupport,
    pub supports_logprobs: CapabilitySupport,
    pub supports_reasoning_format_auto: CapabilitySupport,
    pub supports_reasoning_format_none: CapabilitySupport,
    pub tokenizer: TokenizerKind,
    pub source: CapabilitySource,
}
```

Use explicit `Unknown` states rather than pretending the registry knows everything.

Capability sources should be ordered:

1. User override in TOML.
2. Existing model config folder metadata.
3. Built-in pattern table for known model families.
4. Provider probe results already collected by model behavior setup.
5. Conservative fallback.

Patterns are acceptable for model family detection, but do not route behavior from arbitrary user-message keywords.

## Implementation Completed

### Phase 1: Registry And Loading
- Added `src/model_capabilities.rs` with `ModelCapabilities` struct, `CapabilitySupport`, `CapabilitySource`, `TokenizerKind` enums
- Added `resolve_model_capabilities()` function with TOML override support (`config/model_capabilities.toml`)
- Added built-in patterns for GPT-4, GPT-3.5, Claude-3, Llama-3, Qwen
- Added `clamp_max_tokens()` for request construction
- Tests: 8 passed

### Phase 2: Token Counter
- Added `token_count()` and `context_window_tokens()` functions using estimator fallback
- Tests pass

### Phase 3: Request Construction
- Wired `clamp_max_tokens` into `chat_request_from_profile` in `src/llm_config.rs`

### Phase 4: Session Persistence
- Skipped - depends on Task 430

## Files Changed
- `src/model_capabilities.rs` - NEW (300 lines)
- `src/main.rs` - Added module declaration
- `src/llm_config.rs` - Added max_tokens clamping
- `src/auto_compact.rs` - Uses capability-resolved context window

## Verification
```
cargo build        ✓
cargo test model_capabilities  ✓ (8 passed)
cargo test auto_compact    ✓ (7 passed)
```

## Files To Audit

| File | Reason |
|------|--------|
| `src/llm_config.rs` | Request construction and runtime caps |
| `src/app_bootstrap_core.rs` | Model/config resolution and startup probing |
| `src/app_bootstrap_profiles.rs` | Profile sync and defaults |
| `src/types_core.rs` | Shared profile/model behavior types |
| `src/auto_compact.rs` | Context threshold calculations |
| `src/tool_result_storage.rs` | Large output budget decisions |
| `src/claude_ui/claude_render.rs` | Footer token/context display |
| `src/tuning_support.rs` | Runtime-default max token tuning boundaries |

## Success Criteria

- [ ] The active model has a resolved capability record at startup.
- [ ] Unknown models get conservative fallback capabilities and a visible transcript row.
- [ ] Request `max_tokens` cannot exceed model/provider capability caps.
- [ ] Context percentage and compaction thresholds use the registry-backed token counter.
- [ ] Tool-result persistence uses the same budget source as compaction.
- [ ] Existing model behavior probing still works and is not replaced wholesale.
- [ ] No additional per-session file is introduced for capabilities.

## Verification

```bash
cargo build
cargo test model_capabilities
cargo test auto_compact
cargo test tool_result_storage
cargo test llm_config
```

Manual smoke:

1. Start with a known local model and verify capability resolution source.
2. Start with an unknown model and verify conservative fallback plus visible notice.
3. Configure a TOML override and verify it wins over built-in defaults.
4. Send a long prompt and verify compaction/output caps remain consistent.

## Anti-Patterns To Avoid

- Do not hardcode behavior in prompts.
- Do not assume one tokenizer fits every model.
- Do not silently overrun context when capabilities are unknown.
- Do not make model family matching part of user-request routing.
