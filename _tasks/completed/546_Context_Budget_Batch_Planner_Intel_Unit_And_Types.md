# Task 501: Context-Budget Batch Planner — Intel Unit, Types, and Planning Integration

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 3-4 days
**Primary surfaces:** `src/intel_units/intel_units_batch_planner.rs` (new), `src/types_core.rs`, `src/types_api.rs`, `src/orchestration_planning.rs`, `src/intel_units/mod.rs`
**Depends on:** Task 499 (tiktoken-rs — for accurate token estimation), Task 500 (workspace_info — so model discovers project before batch planning)
**Related tasks:** Task 389 (work graph types), Task 390 (approach engine), Task 494 (full hierarchy integration), Task 258 (context budget document work planner), Task 260 (hybrid document retrieval)

## Objective

Create a Plan-level intel unit that decomposes large investigation tasks into context-budget-aware batches. Each batch of items (files, shell output segments, search result pages, etc.) fits within the remaining context window after accounting for system prompt, conversation history, and response buffer. This allows small models with limited context (4K–16K tokens) to process large datasets through structured batch processing with per-batch summarization.

The core algorithm is greedy bin-packing with token estimates: sort items by estimated token count, fill batches until they approach the budget limit, start a new batch. The output is a `BatchPlan` — a sequence of `BatchGroup` groups ready for execution. The planner is **source-agnostic** — it operates on `estimated_tokens` regardless of whether the data comes from files, shell output, or search results. Data-type awareness lives in the executor's `ItemKind` discriminator, not in the planner.

## Architecture Context

Where the batch planner sits in the orchestration pyramid:

```
Complexity Assessment (existing — app_chat_loop.rs:813)
    │  Determines if batch planning is needed (INVESTIGATE+ with needs_evidence)
    ▼
Work Graph: Goal → SubGoal
    │
    ▼
Plan: "Read codebase to answer user question"
    │
    ├─ Phase A: Item Discovery (existing)
    │     file_scout.rs:45 scout_files() — discovers candidate files
    │     repo_map.rs — symbol-aware repo map
    │     workspace_info tool (Task 500) — project structure
    │
    ├─ Phase B: Batch Planning ← THIS TASK ← THIS TASK ← THIS TASK
    │     BatchPlannerUnit: estimates tokens, groups items into batches
    │     Output: BatchPlan { batches: Vec<BatchGroup> }
    │
    └─ Phase C: Batched Execution (Task 502)
          Each batch: Acquire items → Summarize
          Progressive summarization across batches
          Final aggregation → answer
```

The planner fires **after** item discovery but **before** step execution. It transforms a flat "process N items" objective into a structured multi-batch plan. Future sources (shell output, search results) add `ItemKind` variants without changing the planner.

## Current State

No batch planner exists. Files are read one-at-a-time by the model via the `read` tool in the tool loop. The model manages its own context within the tool loop — it decides which files to read and when. For small models, this leads to:

1. **Context overflow**: Model reads too many large files, hits compaction
2. **Shallow analysis**: Model reads one file, answers from partial evidence
3. **Stagnation loops**: Model re-reads same files, hits `stop_policy.rs:75` after 3 cycles

Existing infrastructure we can leverage:
- `src/model_capabilities.rs:329` — `token_count()` (will use tiktoken-rs after Task 499)
- `src/model_capabilities.rs:346` — `context_window_tokens()` returns available window
- `src/file_scout.rs:45` — `scout_files()` discovers candidate files
- `src/intel_units/` — existing intel unit pattern with `IntelUnit`, `IntelContext`, `IntelOutput`
- `src/execution_steps_read.rs` — existing multi-file read via `paths: Option<&[String]>`
- `src/types_core.rs:665` — `Step` enum with 14 variants
- `src/types_api.rs:214` — `StepResult` for execution results

## Implementation Plan

### Step 1: Define batch planner types

**File:** `src/types_api.rs` (add to the types_api module)

Add new types after `StepResult` (line 233):

```rust
/// Discriminator for data-type-specific acquisition logic.
/// The planner is agnostic to this — only the executor branches on it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ItemKind {
    /// Read from a workspace-relative file path.
    FilePath(String),
    /// Segment of a shell command's output (cmd hash + byte offset + length).
    ShellOutput { command_hash: String, offset_bytes: u64, length_bytes: u64 },
    /// Page of search results (query + file + start line + match count).
    SearchPage { query: String, file: String, start_line: u32, match_count: usize },
    /// Arbitrary text blob (web page, API response, etc.).
    TextBlock { source_label: String },
}

/// A source-agnostic item that can be batched for context-budget processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchableItem {
    /// How to acquire this item's content (file read, shell output segment, etc.).
    pub source_kind: ItemKind,
    /// Estimated token count of the content (from tiktoken-rs).
    pub estimated_tokens: usize,
    /// Human-readable description (why this item was selected).
    pub description: String,
}

/// Input to the batch planner intel unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPlannerInput {
    /// The user's objective for this investigation.
    pub objective: String,
    /// Items to be batched, each with source info and token estimate.
    pub items: Vec<BatchableItem>,
    /// Available tokens per batch (total window - system - conversation - margin).
    pub available_budget_per_batch: usize,
    /// Response buffer: tokens reserved per batch for model's summarization output.
    pub response_buffer_tokens: usize,
    /// Maximum items per batch (safety cap).
    pub max_items_per_batch: usize,
}

/// A group of items that together fit within one context window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchGroup {
    /// 1-indexed batch number.
    pub batch_number: usize,
    /// Source URIs for each item in this batch.
    pub item_uris: Vec<String>,
    /// Source kinds matching item_uris (same order, used by executor).
    pub item_kinds: Vec<ItemKind>,
    /// Total estimated tokens for all items in this batch.
    pub estimated_tokens: usize,
    /// Instructions for the summarization step (what to focus on).
    pub summary_prompt: String,
    /// Whether this batch's summary should reference prior batch summaries.
    pub depends_on_previous: bool,
}

/// The complete batched execution plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPlan {
    /// Ordered list of item batches.
    pub batches: Vec<BatchGroup>,
    /// Total number of items across all batches.
    pub total_items: usize,
    /// Total estimated tokens across all items.
    pub total_estimated_tokens: usize,
    /// Number of batches.
    pub batch_count: usize,
    /// Estimated total tokens for the full investigation (items + LLM summarization).
    pub estimated_total_cost_tokens: usize,
}
```

### Step 2: Add `Step::Batch` variant to the Step enum

**File:** `src/types_core.rs`

Add to the `Step` enum at line 665 (after the last variant, currently `Step::Delete`):

```rust
/// Process items in batches that each fit within one context window.
/// Source-agnostic: items can be files, shell output segments, search results, etc.
/// Cross-batch awareness through progressive summarization.
Batch {
    id: String,
    purpose: String,
    batches: Vec<BatchGroup>,
    depends_on: Vec<String>,
    success_condition: String,
},
```

Update all match arms on the `Step` enum to handle `Step::Batch { .. }`. Use `rg "Step::Read" src/` to find all exhaustive match blocks. Add `Step::Batch { .. }` with appropriate handling:

- `step_id()` in `execution_steps.rs`: return `id.clone()`
- `step_purpose()` in `execution_steps.rs`: return `purpose.clone()`
- `step_depends_on()` in `execution_steps.rs`: return `depends_on.clone()`
- `step_success_condition()` in `execution_steps.rs`: return `success_condition.clone()`
- `step_kind()` in `execution_steps.rs`: return `"batch".to_string()`
- Any serialization/deserialization match blocks
- `handle_program_step()` in `execution_steps.rs:736` — add dispatch arm (executor is in Task 502)

### Step 3: Create the BatchPlannerIntelUnit

**File:** `src/intel_units/intel_units_batch_planner.rs` (NEW)

This intel unit wraps a deterministic greedy bin-packing algorithm. It is **source-agnostic**: it only operates on `estimated_tokens`, not on data origins. Data-type awareness lives in the `ItemKind` discriminator, which is preserved through the planner into the output plan for the executor to use.

```rust
//! @efficiency-role: reasoning-intel
//!
//! Context-Budget Batch Planner Intel Unit.
//!
//! Decomposes large investigation tasks into context-budget-aware batches.
//! Uses accurate token counts (tiktoken-rs via Task 499) for budget calculation.
//! Employs greedy bin-packing. Source-agnostic: works on any BatchableItem
//! regardless of whether data comes from files, shell output, or search results.

use crate::intel_trait::{IntelContext, IntelOutput, IntelUnit};
use crate::*;

// Re-export the types from types_api for convenience
pub(crate) use crate::BatchableItem;
pub(crate) use crate::BatchPlannerInput;
pub(crate) use crate::BatchPlan;
pub(crate) use crate::BatchGroup;

pub(crate) struct BatchPlannerUnit;

impl IntelUnit for BatchPlannerUnit {
    fn name(&self) -> &str { "batch_planner" }
    fn max_tokens(&self) -> usize { 32 } // deterministic, minimal output
    fn timeout_s(&self) -> u64 { 3 }
    fn requires_network(&self) -> bool { false }

    fn execute(&self, ctx: &IntelContext) -> Result<IntelOutput, crate::error::IntelError> {
        // Parse input from context
        let input: BatchPlannerInput = serde_json::from_value(ctx.extra_data.clone())
            .map_err(|e| crate::error::IntelError::Parse(format!("batch planner input: {}", e)))?;

        let plan = Self::plan_batches(&input);
        let data = serde_json::to_value(&plan)
            .map_err(|e| crate::error::IntelError::Parse(format!("batch plan serialization: {}", e)))?;

        Ok(IntelOutput {
            data,
            narrative: format!(
                "Planned {} batches for {} items ({} total tokens)",
                plan.batch_count, plan.total_items, plan.total_estimated_tokens
            ),
            confidence: 1.0,
            classifications: vec![],
        })
    }
}

impl BatchPlannerUnit {
    /// Greedy bin-packing: sort items by token count descending,
    /// fill batches until approaching budget, start new batch.
    /// Source-agnostic — only uses estimated_tokens.
    pub fn plan_batches(input: &BatchPlannerInput) -> BatchPlan {
        let mut items: Vec<_> = input.items.clone();
        // Sort descending by token count (largest items first for best packing)
        items.sort_by(|a, b| b.estimated_tokens.cmp(&a.estimated_tokens));

        let effective_budget = input.available_budget_per_batch
            .saturating_sub(input.response_buffer_tokens);

        let mut batches: Vec<BatchGroup> = Vec::new();
        let mut current_uris: Vec<String> = Vec::new();
        let mut current_kinds: Vec<ItemKind> = Vec::new();
        let mut current_tokens: usize = 0;

        for item in &items {
            // If adding this item would exceed budget OR we've hit max items per batch:
            // finalize current batch and start new one
            let would_overflow = current_tokens + item.estimated_tokens > effective_budget;
            let at_max_items = current_uris.len() >= input.max_items_per_batch;

            if (would_overflow || at_max_items) && !current_uris.is_empty() {
                let batch_num = batches.len() + 1;
                batches.push(BatchGroup {
                    batch_number: batch_num,
                    item_uris: std::mem::take(&mut current_uris),
                    item_kinds: std::mem::take(&mut current_kinds),
                    estimated_tokens: current_tokens,
                    summary_prompt: Self::build_summary_prompt(
                        batch_num, &input.objective, batch_num > 1
                    ),
                    depends_on_previous: batch_num > 1,
                });
                current_tokens = 0;
            }

            // Add item to current batch
            current_uris.push(item.source_kind.to_uri());
            current_kinds.push(item.source_kind.clone());
            current_tokens += item.estimated_tokens;
        }

        // Finalize last partial batch
        if !current_uris.is_empty() {
            let batch_num = batches.len() + 1;
            batches.push(BatchGroup {
                batch_number: batch_num,
                item_uris: current_uris,
                item_kinds: current_kinds,
                estimated_tokens: current_tokens,
                summary_prompt: Self::build_summary_prompt(
                    batch_num, &input.objective, batch_num > 1
                ),
                depends_on_previous: batch_num > 1,
            });
        }

        let total_items: usize = batches.iter().map(|b| b.item_uris.len()).sum();
        let total_tokens: usize = batches.iter().map(|b| b.estimated_tokens).sum();
        let batch_count = batches.len();
        let estimated_cost = total_tokens + batch_count * input.response_buffer_tokens;

        BatchPlan {
            batches,
            total_items,
            total_estimated_tokens: total_tokens,
            batch_count,
            estimated_total_cost_tokens: estimated_cost,
        }
    }

    /// Build a summary prompt for a batch.
    /// First batch: broad discovery. Later batches: relate to objective + prior context.
    fn build_summary_prompt(batch_num: usize, objective: &str, include_prior: bool) -> String {
        let base = format!(
            "Batch {}: Analyze the content from these items focusing on their relevance \
             to the objective: \"{}\". Identify key structures, functions, types, and \
             patterns. Note connections between items in this batch. Be thorough — this \
             summary may be the only representation of these items in later analysis.",
            batch_num, objective
        );
        if include_prior {
            format!(
                "{}\n\nCross-reference findings with previous batch summaries. \
                 Note confirmations, contradictions, and new insights. \
                 Build cumulative understanding toward the objective.",
                base
            )
        } else {
            base
        }
    }
}
```

The `ItemKind::to_uri()` helper produces a human-readable string for each source kind:
- `FilePath("/path/to/file.rs")` → `"/path/to/file.rs"`
- `ShellOutput { command_hash, .. }` → `"shell://{command_hash}/offset={offset_bytes}/len={length_bytes}"`
- `SearchPage { query, file, .. }` → `"search://{query}@{file}:{start_line}"`
- `TextBlock { source_label }` → `"text://{source_label}"`

### Step 4: Register the intel unit

**File:** `src/intel_units/mod.rs`

Add `pub(crate) mod intel_units_batch_planner;` after the existing module declarations.

### Step 5: Add planning function to orchestration_planning.rs

**File:** `src/orchestration_planning.rs`

Add a new public function:

```rust
/// Plan context-budget-aware batches for investigation tasks.
///
/// Called when complexity is INVESTIGATE or higher and the task involves
/// many items that may collectively exceed the context window.
///
/// Returns None if batch planning is not needed (single item, or all items
/// fit within one context window).
pub async fn plan_batches_if_needed(
    client: &reqwest::Client,
    objective: &str,
    candidate_paths: &[ScoutCandidate],
    context_window_tokens: usize,
    conversation_tokens: usize,
) -> Result<Option<BatchPlan>> {
    // If no items or only one item, batch planning is unnecessary
    if candidate_paths.len() <= 1 {
        return Ok(None);
    }

    // Estimate tokens for each candidate item.
    // Currently only FilePath source kind — extend with more ItemKind
    // variants when shell/search batching is added.
    let mut items: Vec<BatchableItem> = Vec::new();
    let mut total_item_tokens: usize = 0;

    for candidate in candidate_paths {
        // Read file content to estimate tokens
        let full_path = std::path::Path::new(&candidate.path);
        let content = match std::fs::read_to_string(full_path) {
            Ok(c) => c,
            Err(_) => continue, // skip unreadable items
        };
        let estimated = crate::token_counter::count_tokens(&content);
        total_item_tokens += estimated;
        items.push(BatchableItem {
            source_kind: ItemKind::FilePath(candidate.path.clone()),
            estimated_tokens: estimated,
            description: candidate.reason.clone(),
        });
    }

    // If all items fit in one context window, no batching needed
    // Reserve 2000 tokens for system prompt + conversation + response
    let overhead = 2000 + conversation_tokens;
    let available = context_window_tokens.saturating_sub(overhead);
    if total_item_tokens <= available {
        trace(
            &crate::Args::default(), // FIXME: pass real args
            &format!(
                "batch_plan_skip reason=all_items_fit total_tokens={} available={}",
                total_item_tokens, available
            ),
        );
        return Ok(None);
    }

    let input = BatchPlannerInput {
        objective: objective.to_string(),
        items,
        available_budget_per_batch: available,
        response_buffer_tokens: 1500, // reserve for LLM summary output
        max_items_per_batch: 20,       // safety cap
    };

    let context = IntelContext::new(
        objective.to_string(),
        RouteDecision::default(),
        String::new(),
        String::new(),
        vec![],
        client.clone(),
    );
    let context = context.with_extra_data(
        serde_json::to_value(&input).unwrap_or_default()
    );

    let unit = BatchPlannerUnit;
    let output = unit.execute_with_fallback(&context).await?;
    let plan: BatchPlan = serde_json::from_value(output.data)
        .map_err(|e| anyhow::anyhow!("batch plan parse: {}", e))?;

    trace(
        &crate::Args::default(), // FIXME: pass real args
        &format!(
            "batch_plan_created batches={} items={} total_tokens={}",
            plan.batch_count, plan.total_items, plan.total_estimated_tokens
        ),
    );

    Ok(Some(plan))
}
```

Note: The `FIXME` on `Args::default()` is a placeholder. The function needs a real `Args` reference for `trace()`. Either pass `args: &Args` as a parameter or make the trace calls conditional.

### Step 6: Convert BatchPlan to Step::Batch

Add a conversion function in the same file or in `src/work_graph_bridge.rs`:

```rust
/// Convert a BatchPlan into a single Step::Batch for execution.
pub fn batch_plan_to_step(plan: &BatchPlan, objective: &str) -> crate::Step {
    let batch_groups: Vec<crate::BatchGroup> = plan.batches.iter().map(|b| {
        crate::BatchGroup {
            batch_number: b.batch_number,
            item_uris: b.item_uris.clone(),
            item_kinds: b.item_kinds.clone(),
            estimated_tokens: b.estimated_tokens,
            summary_prompt: b.summary_prompt.clone(),
            depends_on_previous: b.depends_on_previous,
        }
    }).collect();

    crate::Step::Batch {
        id: format!("batch_{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>()),
        purpose: format!("Process {} items in {} batches to investigate: {}", 
            plan.total_items, plan.batch_count, objective),
        batches: batch_groups,
        depends_on: vec![],
        success_condition: "All batches summarized, aggregated findings available".to_string(),
    }
}
```

## Acceptance Criteria

1. `cargo build` compiles with no errors
2. `BatchPlannerUnit::plan_batches()` correctly groups items:
   - Empty input → empty plan
   - Single item that fits → 1 batch with 1 item
   - 100 items exceeding budget → N batches, each ≤ `available_budget_per_batch - response_buffer_tokens`
   - Each batch has `depends_on_previous: true` except batch 1
   - `item_kinds` are preserved correctly through the planner (item_kinds[i] matches item_uris[i])
3. Unit tests covering:
   - Item sorting (largest token count first)
   - Budget enforcement (no batch exceeds limit)
   - Single large item (item larger than budget → still gets its own batch)
   - Max items per batch cap
   - Mixed ItemKind preservation (FilePath + ShellOutput + SearchPage in same plan)
4. `cargo test` passes all existing and new tests
5. `BatchPlan` serializes/deserializes correctly via serde_json
6. `ItemKind` enum serializes with correct variant names for human-readable debug output

## Risk Assessment

- **Token estimation accuracy**: The planner depends on tiktoken-rs accuracy from Task 499. If Task 499 isn't done yet, use `count_tokens()` as-is (the heuristic is good enough for planning, ±15% error means batches might be slightly over/under budget but never catastrophically so).
- **Unreadable items**: Some items in `ScoutCandidate` may not exist or be unreadable. The planner skips them (see `continue` in Step 5). This is safe — the plan just won't include those items.
- **Single huge item**: If one item alone exceeds the budget per batch, it still gets its own batch. The executor receives a truncated or chunked version (for files, the existing read truncation applies; for shell output, reactive splitting via Task 502's executor handles it).
