# Task 268: Lightweight Local Auxiliary LLM Helper

## Backlog Reconciliation (2026-05-02)

Resume after Task 448 defines model capability metadata and Task 449 removes repeated startup/per-turn scans. Keep this helper optional and local-first.


## Status: PENDING
## Priority: MEDIUM

## Problem Statement
Elma-CLI is local-LLM-only, so multi-agent delegation is impractical. However, there's a legitimate need for a lightweight helper to perform small tasks that the main model shouldn't waste context on:
- Summarizing long tool outputs to key points
- Shortening verbose thinking traces
- Generating concise titles/labels for tasks
- Extracting structured data from unstructured text
- Quick classification or routing decisions

Instead of a full sub-agent, implement a **constant background helper process** using a tiny local LLM.

## Use Cases

### 1. Output Summarization
When tool output exceeds context budget, the main model receives a `<persisted-output>` wrapper. An auxiliary LLM could pre-summarize large outputs before the main model sees them.

### 2. Thinking Truncation
Long thinking traces consume context. A tiny model could compress reasoning to essential points.

### 3. Title Generation
Automatically generate concise titles for tasks, documents, or conversation threads.

### 4. Quick Classification
Fast routing decisions (e.g., "is this a bug report or feature request?") without main model involvement.

## Solution Architecture

### Option A: Constant Background Helper Process (RECOMMENDED)
Spawn a tiny LLM as a persistent background subprocess. Main Elma communicates via stdin/stdout. Benefits:
- **Fast**: No cold-start overhead per request
- **Efficient**: Model stays loaded in memory
- **Simple**: Straightforward pipe-based communication
- **Low latency**: Sub-second response for simple tasks

### Option B: On-Demand Spawn
Spawn the helper LLM only when needed. Simpler but slower (2-5s startup each time).

### Option C: No Helper (Baseline)
Use simple heuristics/truncation instead of LLM. Fastest but less intelligent.

## Implementation Steps (Option A - Recommended)

1. **Config Section**: Add `auxiliary_llm` config
   ```toml
   [auxiliary_llm]
   enabled = false  # Default off
   model = "qwen2.5:0.5b"  # or tinyllama:1.1b, phi2
   endpoint = "http://localhost:11434/api/generate"  # Ollama endpoint
   prompt_template = "Summarize this in 2-3 sentences: {input}"
   timeout_ms = 10000
   ```

2. **Helper Process Manager**: Create `src/auxiliary_llm.rs`
   - Spawn and manage background LLM process
   - Health checking and auto-restart
   - Request queue with timeout handling

3. **Communication Protocol**: Simple JSON over stdin/stdout
   ```json
   {"task": "summarize", "input": "long text...", "id": "req-123"}
   ```

4. **Integration Points**:
   - `src/tool_result_storage.rs`: Optionally summarize large outputs
   - `src/thinking_trace.rs`: Compress thinking if enabled
   - `src/narrative_builder.rs`: Generate titles/summaries

5. **Fallback Handling**: If helper fails or disabled, use simple truncation

## Suggested Lightweight Models

### Top Recommendations

| Model | Size | VRAM | Strengths | Best For |
|-------|------|------|-----------|----------|
| **Qwen2.5-0.5B** | 0.5B | ~1GB | Good reasoning, multilingual | Summarization, titles |
| **TinyLlama-1.1B** | 1.1B | ~2GB | Fast, open license | Quick classification |
| **Phi-2** | 2.7B | ~4GB | Microsoft quality | Slightly complex tasks |
| **SmolLM-1.7B** | 1.7B | ~3GB | Good benchmark scores | Balanced use |
| **Gemma-2B** | 2B | ~4GB | Google's quality | Better reasoning |

### Selection Criteria
- **Must fit in 2-4GB VRAM** (most integrated graphics can handle this)
- **Fast inference** (sub-500ms for short tasks)
- **Good instruction following** for short prompts
- **Small download size** (under 500MB preferred)

### Recommended Setup with Ollama
```bash
# Pull a tiny model
ollama pull qwen2.5:0.5b

# Or for slightly better quality
ollama pull tinyllama:1.1b
```

## API Design

```rust
// src/auxiliary_llm.rs

pub struct AuxiliaryLLM {
    endpoint: String,
    model: String,
    client: reqwest::Client,
}

impl AuxiliaryLLM {
    /// Summarize text to key points
    pub async fn summarize(&self, text: &str, max_sentences: usize) -> Result<String>;
    
    /// Generate a short title
    pub async fn generate_title(&self, content: &str) -> Result<String>;
    
    /// Quick classification
    pub async fn classify(&self, text: &str, categories: &[&str]) -> Result<String>;
    
    /// Compress thinking trace
    pub async fn compress_thinking(&self, thinking: &str) -> Result<String>;
}
```

## Integration Examples

### Example 1: Summarize Large Tool Output
```rust
// In tool_result_storage.rs
if config.auxiliary_llm.enabled {
    let summary = auxiliary.summarize(&content, 3).await?;
    // Include summary in wrapper
}
```

### Example 2: Generate Task Titles
```rust
// In narrative_builder.rs
if config.auxiliary_llm.enabled {
    let title = auxiliary.generate_title(&task_description).await?;
    return title;
}
```

## Success Criteria
- Helper process starts automatically when enabled
- Summaries generate in under 2 seconds
- Graceful fallback when disabled or helper fails
- No impact on normal operation when disabled
- Works with Ollama or similar local LLM servers
- `cargo build` passes

## Files to Create/Modify
- `src/auxiliary_llm.rs` (new - helper process manager)
- `config/defaults.toml` (add auxiliary_llm section)
- `src/tool_result_storage.rs` (optional: add summarization)
- `src/narrative_builder.rs` (optional: add title generation)

## Risk Assessment
- **LOW**: Disabled by default, no impact on existing users
- **LOW**: Simple fallback to heuristics if LLM unavailable
- **MEDIUM**: Need to handle helper process crashes gracefully

## Notes
- This is NOT sub-agent delegation - it's a lightweight utility
- Main model remains in full control
- Auxiliary LLM only handles small transformation tasks
- Can be used alongside Task 267 (when user enables API sub-agents)