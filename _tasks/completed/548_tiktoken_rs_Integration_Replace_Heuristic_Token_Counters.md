# Task 499: tiktoken-rs Integration — Replace Heuristic Token Counters

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Primary surfaces:** `src/token_counter.rs` (new), `src/model_capabilities.rs`, `src/auto_compact.rs`, `src/trajectory.rs`, `src/ui/ui_terminal.rs`, `Cargo.toml`
**Depends on:** None
**Related tasks:** Task 343 (exact tokenization and model capability registry), Task 114 (auto compact context window management), Task 269 (advanced context compaction with LLM summarization)

## Objective

Replace all `chars / 3.5` and `text.len() / 4` heuristic token estimators with `tiktoken-rs`, a Rust port of OpenAI's tiktoken library. This eliminates ~±15% token counting error in favor of exact BPE tokenization using the `cl100k_base` encoding (used by GPT-4, GPT-3.5-turbo).

Accurate token counting is essential for:
- Compaction thresholds firing at correct context usage (not too early, not too late)
- Batch read planner (Task 501) which must guarantee each file batch fits within the remaining context window
- Context budget visualization in the status bar
- Session store token_count column accuracy

## Current State

Four separate implementations of `text.len() / 4` or `text.len() as f64 / 3.5` exist:

| Location | Function | Constant | Line |
|---|---|---|---|
| `src/model_capabilities.rs` | `token_count()` | `CHARS_PER_TOKEN_ESTIMATOR = 3.5` | 327-343 |
| `src/auto_compact.rs` | `CompactTracker::estimate_tokens()` | `CHARS_PER_TOKEN = 3.5` | 56-57 |
| `src/trajectory.rs` | `estimate_tokens()` (private) | `CHARS_PER_TOKEN = 3.5` | 183-185 |
| `src/ui/ui_terminal.rs` | `TerminalUI::estimate_tokens()` | inline `text.len() / 4` | 739-741 |

Callers of these functions:

- `auto_compact.rs:64,164-173,239` — `CompactTracker::recalculate()`, `generate_inline_summary()`, internal total
- `tool_loop.rs:702` — `TerminalUI::estimate_tokens(&m.content)` for status bar
- `app_chat_loop.rs:1070` — `TerminalUI::estimate_tokens(&msg.content)` for final answer display
- `trajectory.rs:43,117,240,269,271,334` — trajectory compression token counting
- Various tests in each module asserting specific token counts

`ModelCapabilities` already has a `tokenizer: TokenizerKind` enum (line 24-42) with variants `Tiktoken`, `Cl100kBase`, `Anthropic`, `HuggingFace`, `Estimator`, `None` — all currently map to the same heuristic. This task makes them meaningful.

## Crate Decision

**Use `tiktoken-rs`** (`https://crates.io/crates/tiktoken-rs`). This is the standard Rust port of OpenAI's tiktoken. It supports four BPE encodings:

| Encoding | Used by | Rank file size |
|---|---|---|
| `cl100k_base` | GPT-4, GPT-3.5-turbo, GPT-4o | ~1.8 MB |
| `p50k_base` | text-davinci-003, code-davinci-002 | ~1.5 MB |
| `r50k_base` | GPT-3 davinci | ~1.0 MB |
| `gpt2` | GPT-2 | ~1.0 MB |

Use the `static` feature flag to bundle BPE rank files at compile time — no runtime network downloads, maintaining Elma's offline-first philosophy.

**Alternatives considered and rejected:**
- `tokenizers` (HuggingFace crate): heavier, pulls in Python tokenizer dependencies, no meaningful benefit over tiktoken-rs for OpenAI-compatible models
- Manual port: error-prone, maintenance burden, no benefit over the well-maintained community crate

## Implementation Plan

### Step 1: Add tiktoken-rs dependency

**File:** `Cargo.toml`

```toml
tiktoken-rs = { version = "0.6", features = ["static"] }
```

The `static` feature bundles BPE files directly into the binary at compile time. No network calls at runtime.

### Step 2: Create unified token_counter module

**File:** `src/token_counter.rs` (NEW)

```rust
//! @efficiency-role: util-pure
//!
//! Unified token counter backed by tiktoken-rs.
//! Replaces all chars/3.5 heuristics with exact BPE tokenization.

use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

static CL100K: OnceLock<CoreBPE> = OnceLock::new();
static P50K: OnceLock<CoreBPE> = OnceLock::new();

fn cl100k() -> &'static CoreBPE {
    CL100K.get_or_init(|| {
        tiktoken_rs::cl100k_base()
            .expect("cl100k_base BPE must be available (static feature)")
    })
}

fn p50k() -> &'static CoreBPE {
    P50K.get_or_init(|| {
        tiktoken_rs::p50k_base()
            .expect("p50k_base BPE must be available (static feature)")
    })
}

/// Count tokens using cl100k_base encoding (GPT-4 family).
/// This is the default for all models unless a specific encoding is requested.
pub fn count_tokens(text: &str) -> usize {
    cl100k().encode_with_special_tokens(text).len()
}

/// Count tokens using the encoding appropriate for the given tokenizer kind.
pub fn count_tokens_for_model(text: &str, tokenizer: crate::model_capabilities::TokenizerKind) -> usize {
    let bpe = match tokenizer {
        crate::model_capabilities::TokenizerKind::Cl100kBase
        | crate::model_capabilities::TokenizerKind::Tiktoken
        | crate::model_capabilities::TokenizerKind::Anthropic
        | crate::model_capabilities::TokenizerKind::HuggingFace
        | crate::model_capabilities::TokenizerKind::Estimator
        | crate::model_capabilities::TokenizerKind::None => cl100k(),
    };
    bpe.encode_with_special_tokens(text).len()
}
```

Add `pub mod token_counter;` to `src/mod.rs`.

Key design decisions:
- `OnceLock` singletons ensure BPE models are loaded exactly once
- `expect()` at init time — if BPE files aren't available (should never happen with `static` feature), the binary panics at first use rather than silently degrading
- All non-OpenAI tokenizer kinds map to `cl100k_base` — this is the best approximation for Anthropic, HuggingFace, and unknown models
- If we later add `p50k_base` support for legacy OpenAI models, the `p50k()` singleton is already in place

### Step 3: Replace `model_capabilities::token_count()`

**File:** `src/model_capabilities.rs`

Replace lines 329-343 with:

```rust
pub(crate) fn token_count(text: &str, capabilities: &ModelCapabilities) -> usize {
    crate::token_counter::count_tokens_for_model(text, capabilities.tokenizer)
}
```

Remove `const CHARS_PER_TOKEN_ESTIMATOR: f64 = 3.5;` on line 327.

Update test `test_token_count_estimator` at line 303:
- Old assertion: `count >= 3 && count <= 4` for "hello world" (11 chars / 3.5 ≈ 3.14)
- New assertion: tiktoken encodes "hello world" as 2 tokens (`hello`, ` world` with the space prefix)
  - Update to: `assert_eq!(token_count("hello world", &caps), 2);`

### Step 4: Replace `CompactTracker::estimate_tokens()`

**File:** `src/auto_compact.rs`

Replace line 56-58:

```rust
pub(crate) fn estimate_tokens(text: &str) -> usize {
    crate::token_counter::count_tokens(text)
}
```

Remove `const CHARS_PER_TOKEN: f64 = 3.5;` on line 15.

Update doc comment on line 55 to: `/// Count tokens using tiktoken-rs cl100k_base encoding.`

All callers (`recalculate()` at line 61, `generate_inline_summary()` at lines 164-173, line 239) call `Self::estimate_tokens()` or `CompactTracker::estimate_tokens()` — no signature change needed, they get accurate counts automatically.

Update any tests that assert specific token counts from the old heuristic.

### Step 5: Replace `TerminalUI::estimate_tokens()`

**File:** `src/ui/ui_terminal.rs`

Replace lines 737-741:

```rust
/// Count tokens using tiktoken-rs cl100k_base encoding.
#[allow(dead_code)]
pub(crate) fn estimate_tokens(text: &str) -> u64 {
    crate::token_counter::count_tokens(text) as u64
}
```

Update test `test_estimate_tokens` at line 2319:
- Old: `assert_eq!(TerminalUI::estimate_tokens("hello"), 1);` (5 chars / 4)
- New: `assert_eq!(TerminalUI::estimate_tokens("hello"), 1);` ("hello" is 1 token in cl100k)
- Old: `assert_eq!(TerminalUI::estimate_tokens("hello world"), 2);` (11 chars / 4 = 2)
- New: `assert_eq!(TerminalUI::estimate_tokens("hello world"), 2);` (2 tokens in cl100k)

(These happen to match by coincidence for these particular strings. Verify with actual tiktoken-rs output.)

### Step 6: Replace `trajectory::estimate_tokens()`

**File:** `src/trajectory.rs`

Replace lines 183-185:

```rust
fn estimate_tokens(text: &str) -> usize {
    crate::token_counter::count_tokens(text)
}
```

Remove the local `const CHARS_PER_TOKEN: f64 = 3.5;`.

All callers (lines 43, 117, 240, 269, 271, 334) call `estimate_tokens()` — no API change.

### Step 7: Remove all remaining heuristic constants

Run verification grep:
```bash
rg "CHARS_PER_TOKEN|chars_per_token|/ 4\)|/ 3\.5" --include='*.rs' src/
```

Expected result: no matches (all constants removed in Steps 3-6).

### Step 8: Update test assertions across all touched modules

Any test that asserts a specific token count must be checked against actual tiktoken-rs output. Common cases:

| Text | Old est. (chars/3.5) | tiktoken cl100k | Change |
|---|---|---|---|
| `"hello world"` | 3 | 2 | ↓1 |
| `"fn main() {}"` | 3 | 5 | ↑2 |
| `"error: file not found"` | 7 | 6 | ↓1 |
| `"```rust\nlet x = 1;\n```"` | 8 | 12 | ↑4 |
| Empty string `""` | 0 | 0 | same |
| 10KB of Rust code | ~2928 | ~3500-4000 | ↑20-30% |

Rust code tends to have more tokens per character than English prose (more special characters == more token boundaries). Expect compaction to fire slightly earlier for code-heavy sessions and slightly later for chat-heavy sessions.

## Acceptance Criteria

1. `cargo build` compiles with no errors
2. `cargo test` passes all tests (with updated assertions)
3. `cargo test test_token_count_estimator` passes with correct tiktoken counts
4. `cargo test test_estimate_tokens` passes
5. No remaining `CHARS_PER_TOKEN`, `text.len() / 4`, or `text.len() as f64 / 3.5` in `src/`
6. `rg "CHARS_PER_TOKEN" src/` returns empty
7. `cargo clippy` produces no new warnings
8. Manual test: run Elma on a multi-file task, verify compaction fires at the right time (not premature, not too late)

## Risk Assessment

- **Build time increase**: The `static` feature embeds ~1.8MB of BPE data. Expected binary size increase: ~2MB. Acceptable.
- **Performance**: Tokenization is CPU-bound but extremely fast (microseconds per KB). No measurable impact.
- **Test breakage**: Tests that assert exact token counts will need updating. This is expected and mechanical.
- **No fallback needed**: With the `static` feature, BPE files are compiled in. There's no runtime download to fail. The `expect()` on initialization is a hard failure that indicates a build problem, not a runtime condition.
