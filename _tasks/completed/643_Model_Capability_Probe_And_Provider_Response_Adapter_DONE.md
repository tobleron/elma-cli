# Task 643: Model Capability Probe And Provider Response Adapter

**Status:** pending
**Priority:** CRITICAL
**Type:** Model Robustness / Architecture
**Scope:** `src/llm_provider.rs`, `src/model_capabilities.rs`, `src/tool_loop.rs`, `config/*/model_behavior.toml`
**Source:** user request for modern thinking and dense coder model support; `_knowledge_base` Codex/Roo/Goose provider patterns

## Summary

Probe and adapt model/provider behavior instead of assuming one OpenAI-compatible streaming/tool-calling shape.

## Evidence And Gap

- `src/tool_loop.rs` forces `req.reasoning_format = Some("auto")` in streaming model turns.
- `src/model_capabilities.rs` has support enums, but live request behavior still relies on defaults and static config.
- `llm_provider.rs` normalizes several providers but needs stronger capability truth for tool-call deltas, reasoning fields, streaming support, stop reasons, and context limits.

## Implementation Plan

1. Add startup/runtime probes for streaming, tool-call delta format, `reasoning_format`, max context, stop reason shape, and empty delta behavior.
2. Persist probe results per model/profile and expose them in session diagnostics.
3. Route requests through a provider adapter that selects streaming/non-streaming, reasoning format, and fallback parsing based on proven capability.
4. Add compatibility fixtures for llama.cpp, Ollama/vLLM style OpenAI-compatible APIs, Anthropic, and dense coder local models.

## Acceptance Criteria

- [ ] Unsupported `reasoning_format=auto` is not sent to models that reject it.
- [ ] Tool-call delta parsing is adapter-driven and fixture-tested.
- [ ] Capability probe failures degrade to safe non-streaming/tool-free behavior with visible transcript notices.
- [ ] Session artifacts record model capability assumptions used for the turn.

## Verification Plan

Run provider fixture tests and a real local model smoke prompt that uses tools and a final answer.

