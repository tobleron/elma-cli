# Task 383: Lightweight Local Auxiliary LLM Helper

## Backlog Reconciliation (2026-05-02)

Resume after Task 448 and Task 449. Check duplicate scope against deferred Task 279 before implementation.


**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 3-4 days
**Dependencies:** None
**References:** objectives.md principle 4, AGENTS.md Rule 4, _masterplan.md Task 279

## Problem

Per objectives.md: "Elma produces correct, grounded answers on any model size, with stability, regardless of how many intel unit calls or how much wall-clock time it takes."

The main model wastes context on small tasks:
- Summarizing long tool outputs
- Generating concise titles/labels
- Quick classification decisions (e.g., "is this output an error or success?")
- Shortening verbose thinking traces

These tasks are cognitively trivial but context-expensive. A lightweight auxiliary model can handle them, freeing the main model's context window for actual reasoning.

## Objective

Implement a lightweight auxiliary LLM pipeline for small transformation tasks that the main model shouldn't waste context on. Disabled by default. When enabled, offloads summarization, classification, and compression to a tiny model.

## Use Cases

| Task | Input | Output | Model size needed |
|------|-------|--------|-------------------|
| Summarize tool output | Large read/shell output | 2-3 sentence summary | 0.5B-1B |
| Generate title | Task description | Short title | 0.5B-1B |
| Quick classify | Raw text + categories | Category label | 0.5B-1B |
| Compress thinking | Long reasoning trace | Key points only | 1B-2B |
| Extract JSON | Garbled model output | Clean JSON | 1B-2B |

## Implementation Plan

### Phase 1: Config Section

Add to config system:

```toml
[auxiliary_llm]
enabled = false
endpoint = "http://localhost:11434/api/generate"
model = "qwen2.5:0.5b"
timeout_ms = 10000
max_context_length = 2048
```

### Phase 2: AuxiliaryLLM Module

Create `src/auxiliary_llm.rs`:

```rust
pub(crate) struct AuxiliaryLLM {
    enabled: bool,
    endpoint: String,
    model: String,
    client: reqwest::Client,
    timeout: Duration,
}

impl AuxiliaryLLM {
    pub async fn summarize(&self, text: &str, max_sentences: usize) -> Result<String>;
    pub async fn generate_title(&self, content: &str) -> Result<String>;
    pub async fn classify(&self, text: &str, categories: &[&str]) -> Result<String>;
    pub async fn compress_thinking(&self, thinking: &str) -> Result<String>;
}
```

### Phase 3: Integration Points

| Integration point | Function |
|-------------------|----------|
| `src/tool_result_storage.rs` | Summarize large tool outputs (>5KB) |
| `src/thinking_trace.rs` | Compress thinking traces >2KB |
| `src/narrative_builder.rs` | Generate task/summary titles |
| `src/json_repair.rs` (Task 378) | Extract JSON from garbled output |

### Phase 4: Fallback

If auxiliary is disabled or fails:
- Summarization → truncate to first N chars
- Title generation → use first sentence
- Classification → use deterministic string matching
- JSON extraction → use regex fallback (existing)

## Files to Create/Modify

| File | Action |
|------|--------|
| `src/auxiliary_llm.rs` | CREATE — helper LLM client |
| `config/defaults.toml` | MODIFY — add `[auxiliary_llm]` section |
| `src/config.rs` | MODIFY — parse auxiliary_llm config |

## Suggested Models (Ollama)

```bash
ollama pull qwen2.5:0.5b    # 0.5B, ~1GB VRAM — summarization/titles
ollama pull tinyllama:1.1b  # 1.1B, ~2GB VRAM — classification
```

## Verification

```bash
cargo build
cargo test auxiliary
```

**Manual**: Enable `auxiliary_llm.enabled = true`, run a query with large tool output. Verify summary is injected into context instead of raw output.
