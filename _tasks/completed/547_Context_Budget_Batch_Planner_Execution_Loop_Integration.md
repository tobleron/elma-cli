# Task 502: Context-Budget Batch Planner — Execution, Loop Integration, and Semantic Features

**Status:** pending
**Priority:** HIGH
**Estimated effort:** 4-5 days
**Primary surfaces:** `src/execution_steps.rs`, `src/orchestration_loop.rs`, `src/intel_units/intel_units_batch_planner.rs` (extend from Task 501), `src/file_scout.rs`, `src/repo_map.rs`
**Depends on:** Task 501 (batch planner intel unit and types), Task 499 (accurate token counting)
**Related tasks:** Task 362 (parallel read/search tool execution), Task 463 (symbol-aware repo map), Task 456 (file context tracker)

## Objective

Wire the batch planner into the autonomous execution loop, implement the `Step::Batch` executor with source-aware data acquisition (matching on `ItemKind` for file read, shell output splitting, search pagination), add semantic item grouping using repo map symbol data, and implement progressive cross-batch summarization.

## Current State

- `Step::Batch` variant defined in Task 501 (with `BatchGroup` re-used from types_api)
- `BatchPlannerUnit` intel unit created in Task 501 (deterministic bin-packing, source-agnostic)
- `plan_batches_if_needed()` function added to `orchestration_planning.rs` in Task 501
- `batch_plan_to_step()` conversion function in Task 501
- `src/execution.rs:13` — `execute_program()` iterates over program steps sequentially
- `src/orchestration_loop.rs:90` — `run_autonomous_loop()` calls `execute_program()` at line 207
- `src/execution_steps_read.rs:8` — `handle_read_step()` already handles multi-file reads via `paths: Option<&[String]>`
- `src/execution_steps.rs:311` — `handle_summarize_step()` already exists for calling the summarizer LLM
- `src/repo_map.rs` — `RepoMapCache.files[].symbols` has cross-file symbol references

## Implementation Plan

### Step 1: Implement `handle_batch_step()` executor

**File:** `src/execution_steps.rs`

Add the executor function after `handle_summarize_step` (around line 345). Place before `handle_program_step`.

This executor is **source-aware**: it matches on `ItemKind` to decide how to acquire content for each item in a batch. File paths read from disk. Shell output segments slice from a stored output blob. Search pages re-run a constrained search. Text blocks are passed as-is.

```rust
/// Execute a batch step: for each batch, acquire content based on ItemKind,
/// summarize with LLM, and accumulate batch summaries for the final aggregation.
/// Source-agnostic planner output is realized into concrete data here.
#[allow(clippy::too_many_arguments)]
async fn handle_batch_step(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    summarizer_cfg: &Profile,
    batches: &[BatchGroup],
    objective: &str,
    state: &mut ExecutionState,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<()> {
    let mut batch_summaries: Vec<String> = Vec::new();
    let mut total_items_processed: usize = 0;
    let mut failures: Vec<String> = Vec::new();

    for batch in batches {
        if let Some(ref mut t) = tui {
            t.set_coordinator_status(
                format!("Batch {}/{}: processing {} items...", 
                    batch.batch_number, batches.len(), batch.item_uris.len()),
                true,
            );
            let _ = t.pump_ui();
        }

        // Acquire content for each item in this batch based on source_kind
        let mut batch_content = String::new();
        for (i, kind) in batch.item_kinds.iter().enumerate() {
            let item_uri = &batch.item_uris[i];

            let content_result = match kind {
                ItemKind::FilePath(path) => {
                    let full_path = if path.starts_with('/') {
                        std::path::PathBuf::from(path)
                    } else {
                        workdir.join(path)
                    };
                    match std::fs::read_to_string(&full_path) {
                        Ok(c) => {
                            let tokens = crate::token_counter::count_tokens(&c);
                            Ok(format!("\n=== ITEM: {} ({} tokens, source=file) ===\n{}\n",
                                path, tokens, c))
                        }
                        Err(e) => Err(format!("File read error: {} — {}", path, e)),
                    }
                }
                ItemKind::ShellOutput { command_hash, offset_bytes, length_bytes } => {
                    // Retrieve from stored shell output (see artifact system)
                    let artifact_key = format!("shell_output_{}", command_hash);
                    if let Some(output) = state.artifacts.get(&artifact_key) {
                        let start = *offset_bytes as usize;
                        let end = (*offset_bytes + *length_bytes) as usize;
                        let segment = output.get(start..end.min(output.len())).unwrap_or("");
                        Ok(format!("\n=== ITEM: shell://{} (bytes {}-{}) ===\n{}\n",
                            command_hash, offset_bytes, offset_bytes + length_bytes, segment))
                    } else {
                        Err(format!("Shell output artifact not found: {}", command_hash))
                    }
                }
                ItemKind::SearchPage { query, file, start_line, match_count } => {
                    // Execute constrained search
                    let search_cmd = std::process::Command::new("rg")
                        .args(["-n", "-C", "2", query, file])
                        .output();
                    match search_cmd {
                        Ok(out) => {
                            let text = String::from_utf8_lossy(&out.stdout);
                            Ok(format!("\n=== ITEM: search://{}@{} ({} matches) ===\n{}\n",
                                query, file, match_count, text))
                        }
                        Err(e) => Err(format!("Search error: {}@{} — {}", query, file, e)),
                    }
                }
                ItemKind::TextBlock { source_label } => {
                    // TextBlock content is expected to be pre-loaded in artifacts
                    let artifact_key = format!("text_block_{}", source_label);
                    if let Some(text) = state.artifacts.get(&artifact_key) {
                        Ok(format!("\n=== ITEM: text://{} ===\n{}\n", source_label, text))
                    } else {
                        Err(format!("Text block artifact not found: {}", source_label))
                    }
                }
            };

            match content_result {
                Ok(content) => {
                    batch_content.push_str(&content);
                    total_items_processed += 1;
                }
                Err(err_msg) => {
                    failures.push(format!("{} [{}]", err_msg, item_uri));
                    batch_content.push_str(&format!("\n=== ITEM: {} (ERROR: {}) ===\n", item_uri, err_msg));
                }
            }
        }

        // Build summarization prompt with progressive context
        let mut summary_prompt = batch.summary_prompt.clone();

        if batch.depends_on_previous && !batch_summaries.is_empty() {
            summary_prompt.push_str(&format!(
                "\n\nThis is batch {}/{}.\n", batch.batch_number, batches.len()
            ));
            summary_prompt.push_str("\n## Previous batch findings (for context, do not repeat)\n");
            for (i, prior) in batch_summaries.iter().enumerate() {
                // Truncate each prior summary to ~500 tokens to keep context lean
                let token_count = crate::token_counter::count_tokens(prior);
                let display = if token_count > 500 {
                    let cutoff = prior.char_indices()
                        .nth(prior.len() / 4 * 3)
                        .map(|(i, _)| i)
                        .unwrap_or(prior.len());
                    format!("{}... (truncated, {} total tokens)", &prior[..cutoff], token_count)
                } else {
                    prior.clone()
                };
                summary_prompt.push_str(&format!(
                    "### Batch {} summary ({})\n{}\n\n", i + 1, token_count, display
                ));
            }
            summary_prompt.push_str(
                "Use the above context to avoid repeating findings. \
                 Focus on new information and connections across batches. \
                 Build cumulative understanding toward the objective."
            );
        }

        // Call summarizer LLM
        let summary = summarize_batch_content(
            client, chat_url, summarizer_cfg,
            &batch_content, &summary_prompt, objective,
        ).await?;

        batch_summaries.push(summary.clone());

        // Store per-batch summary as artifact
        let artifact_key = format!("batch_summary_{}", batch.batch_number);
        state.artifacts.insert(artifact_key, summary);

        if let Some(ref mut t) = tui {
            t.push_meta_event(
                "BATCH",
                &format!(
                    "batch {}/{} complete: {} items processed, {} failures",
                    batch.batch_number, batches.len(),
                    batch.item_uris.len(), failures.len()
                ),
            );
            let _ = t.pump_ui();
        }
    }

    // Aggregate all batch summaries into final output
    let mut aggregated = String::new();
    aggregated.push_str(&format!(
        "## Batch Processing Results: {} items across {} batches\n\n",
        total_items_processed, batches.len()
    ));

    if !failures.is_empty() {
        aggregated.push_str("### Warnings\n");
        for f in &failures {
            aggregated.push_str(&format!("- {}\n", f));
        }
        aggregated.push('\n');
    }

    for (i, summary) in batch_summaries.iter().enumerate() {
        aggregated.push_str(&format!("### Batch {}\n{}\n\n", i + 1, summary));
    }

    // Store aggregated summary as artifact
    state.artifacts.insert("aggregated_summary".to_string(), aggregated.clone());

    let success = failures.is_empty() || failures.len() < batches.iter().map(|b| b.item_uris.len()).sum::<usize>() / 2;

    state.step_results.push(StepResult {
        id: format!("batch_{}", uuid::Uuid::new_v4().to_string().chars().take(8).collect::<String>()),
        kind: "batch".to_string(),
        purpose: format!("Process {} items in {} batches", total_items_processed, batches.len()),
        depends_on: vec![],
        success_condition: "All batches completed with usable summaries".to_string(),
        ok: success,
        summary: aggregated,
        command: None,
        raw_output: None,
        exit_code: None,
        output_bytes: None,
        truncated: false,
        timed_out: false,
        artifact_path: None,
        artifact_kind: None,
        outcome_status: Some(if success { "completed" } else { "partial" }.to_string()),
        outcome_reason: if !failures.is_empty() {
            Some(format!("{} item acquisition failures", failures.len()))
        } else {
            None
        },
    });

    Ok(())
}

/// Call the summarizer LLM to produce a focused summary of batch content.
async fn summarize_batch_content(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    batch_content: &str,
    summary_prompt: &str,
    objective: &str,
) -> Result<String> {
    let system_prompt = format!(
        "You are a structured analysis summarizer. Your task: read the following content \
         from multiple sources and produce a detailed summary focused on the objective: \"{}\". \n\
         Include: key information found, relationships between items, and relevance to \
         the objective. Be thorough — this summary may be the only representation of \
         these items for later analysis. \n\
         Output format: plain text paragraphs, no markdown headings.",
        objective
    );

    let user_message = format!(
        "{}\n\n## Item contents\n{}",
        summary_prompt, batch_content
    );

    // Use existing summarizer LLM call (same pattern as handle_summarize_step)
    let response = crate::llm_provider::chat_completion(
        client,
        chat_url,
        cfg,
        &system_prompt,
        &user_message,
        vec![],
        None,
    ).await?;

    Ok(response)
}
```

### Step 2: Wire `Step::Batch` into `handle_program_step()`

**File:** `src/execution_steps.rs`, in `handle_program_step()` around line 736

Add to the `match step` block:

```rust
Step::Batch { id, purpose, batches, depends_on, success_condition } => {
    handle_batch_step(
        args, client, chat_url, session, workdir,
        summarizer_cfg,
        &batches, objective, state, tui.as_deref_mut(),
    ).await?
}
```

### Step 3: Integrate batch planner into the autonomous loop

**File:** `src/orchestration_loop.rs`

In `run_autonomous_loop()` at line 90, after the `AgentPlan` is created (line 132) and before the main `loop` (line 154), add:

```rust
// Check if batch planning is needed and the program has Read steps that
// should be batched for context budget management.
// This fires for INVESTIGATE+ complexity with needs_evidence.
let program = if plan.current_program.steps.iter().any(|s| {
    matches!(s, Step::Read { .. })
}) && complexity.complexity != "DIRECT" && complexity.needs_evidence {
    // Collect file paths from existing Read steps
    let read_paths: Vec<String> = plan.current_program.steps.iter()
        .filter_map(|s| {
            if let Step::Read { path, paths, .. } = s {
                if let Some(p) = path { return Some(vec![p.clone()]); }
                if let Some(ps) = paths { return Some(ps.clone()); }
            }
            None
        })
        .flatten()
        .collect();

    if !read_paths.is_empty() {
        // Convert paths to ScoutCandidates
        let candidates: Vec<ScoutCandidate> = read_paths.iter().map(|p| {
            ScoutCandidate {
                path: p.clone(),
                reason: "explicitly requested".to_string(),
            }
        }).collect();

        // Get context budget info
        let context_window = crate::model_capabilities::context_window_tokens(
            &runtime_model_capabilities() // from app bootstrap
        );
        let conversation_tokens = messages.iter()
            .map(|m| crate::token_counter::count_tokens(&m.content))
            .sum();

        // Plan batches
        if let Ok(Some(batch_plan)) = crate::orchestration_planning::plan_batches_if_needed(
            client, &user_message, &candidates, context_window, conversation_tokens,
        ).await {
            // Replace program with batch step
            let batch_step = crate::orchestration_planning::batch_plan_to_step(
                &batch_plan, &user_message
            );
            let mut new_program = plan.current_program.clone();
            new_program.steps = vec![batch_step];
            plan.current_program = new_program;

            if let Some(ref mut t) = tui {
                t.push_meta_event(
                    "BATCH_PLAN",
                    &format!(
                        "{} items split into {} batches ({} estimated tokens total)",
                        batch_plan.total_items,
                        batch_plan.batch_count,
                        batch_plan.estimated_total_cost_tokens,
                    ),
                );
            }
        }
    }
    plan.current_program.clone()
} else {
    plan.current_program.clone()
};

plan.current_program = program;
```

**Important**: This code needs access to `runtime` or model capabilities. If `run_autonomous_loop()` doesn't currently receive `AppRuntime`, we need to either:

**Option A**: Pass `context_window_tokens: usize` as a new parameter
**Option B**: Call `model_capabilities::context_window_tokens()` with a default/from the profiles
**Option C**: Read from a global

**Recommend Option B**: The model capabilities are available from the loaded profiles or from `ModelCapabilities::default()`. Since `profiles` is already a parameter, add a `model_capabilities: &ModelCapabilities` parameter.

### Step 4: Wire batch planner integration point

**File:** `src/orchestration_loop.rs`, update function signature at line 90

Add parameter:
```rust
pub(crate) async fn run_autonomous_loop(
    // ... existing params ...
    model_capabilities: &ModelCapabilities,  // NEW
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> Result<AutonomousLoopOutcome> {
```

Use `model_capabilities::context_window_tokens(model_capabilities)` instead of the placeholder in Step 3 above.

**File:** `src/orchestration_retry.rs:477` — the call to `run_autonomous_loop()` needs the new parameter. Pass `&profiles.model_capabilities` or equivalent.

### Step 5: Add semantic item grouping via import/call graph

**File:** `src/intel_units/intel_units_batch_planner.rs` — extend `BatchPlannerUnit`

Add a new method that optionally reorders items within batches based on repo map adjacency:

```rust
/// Reorder items within batches based on semantic adjacency from repo map.
/// Items that are directly connected (imports, calls) should be in the same batch
/// so the model sees coherent content, not random fragments.
/// This applies only to FilePath items — non-file items keep their positions.
pub fn apply_semantic_grouping(
    plan: &mut BatchPlan,
    repo_map: &crate::repo_map::RepoMapCache,
) {
    // Skip if repo map isn't available or plan only has one batch
    if plan.batches.len() <= 1 {
        return;
    }

    // For each batch, sort FilePath items by directory grouping (same dir together).
    // Non-FilePath items stay in their original positions.
    // This is a best-effort optimization — budget constraints take priority.
    for batch in &mut plan.batches {
        // Pair item_uris with item_kinds for sorting
        let mut paired: Vec<(usize, String, ItemKind)> = batch.item_uris.iter()
            .zip(batch.item_kinds.iter())
            .enumerate()
            .map(|(pos, (uri, kind))| (pos, uri.clone(), kind.clone()))
            .collect();

        // Sort FilePath items by directory, keep non-FilePath in position
        paired.sort_by(|(pos_a, uri_a, kind_a), (pos_b, uri_b, kind_b)| {
            let a_is_file = matches!(kind_a, ItemKind::FilePath(_));
            let b_is_file = matches!(kind_b, ItemKind::FilePath(_));
            if a_is_file && b_is_file {
                let dir_a = std::path::Path::new(uri_a).parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                let dir_b = std::path::Path::new(uri_b).parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                dir_a.cmp(&dir_b).then_with(|| uri_a.cmp(uri_b))
            } else {
                pos_a.cmp(pos_b) // preserve original order for non-file items
            }
        });

        batch.item_uris = paired.iter().map(|(_, uri, _)| uri.clone()).collect();
        batch.item_kinds = paired.iter().map(|(_, _, kind)| kind.clone()).collect();
    }
}
```

**Future extension**: When repo_map symbol data includes explicit import/call relationships (e.g., `symbol.file`, `symbol.references` pointing to other files), replace the directory-based heuristic with actual dependency graph clustering. This requires extending `repo_map.rs` to track cross-file symbol references.

### Step 6: Implement progressive summarization in executor

This is already partially implemented in Step 1's `handle_batch_step()` — the `batch.depends_on_previous` flag controls whether prior batch summaries are injected into the current batch's summarization prompt.

Enhancement: Add a `total_batches` awareness so batch N knows it's "batch 3 of 5" and can calibrate its depth accordingly:

```rust
// In the summarization prompt builder (Step 1, around line 145):
if batch.depends_on_previous && !batch_summaries.is_empty() {
    summary_prompt.push_str(&format!(
        "\n\nThis is batch {}/{}.\n",
        batch.batch_number, batches.len()
    ));
    summary_prompt.push_str("\n## Previous batch findings (for context, do not repeat)\n");
    for (i, prior) in batch_summaries.iter().enumerate() {
        // Truncate each prior summary to ~500 tokens to keep context lean
        let truncated = crate::token_counter::count_tokens(prior);
        let display = if truncated > 500 {
            format!("{}...", &prior[..prior.char_indices()
                .nth(500.min(prior.len()) / 4 * 3) // rough char cutoff
                .map(|(i, _)| i).unwrap_or(prior.len())])
        } else {
            prior.clone()
        };
        summary_prompt.push_str(&format!(
            "### Batch {} summary ({} tokens)\n{}\n\n",
            i + 1, truncated, display
        ));
    }
    summary_prompt.push_str(
        "Use the above context to avoid repeating findings. \
         Focus on new information and connections across batches. \
         Build cumulative understanding toward the objective."
    );
}
```

### Step 7: Update callers of `run_autonomous_loop()`

**File:** `src/orchestration_retry.rs`

The function `run_autonomous_loop()` is called at:
- Line 477 (main retry loop)
- Line 651 (meta-review fallback)

Both call sites need the new `model_capabilities` parameter.

Similarly, `execute_program()` at `src/execution.rs:13` is called at `orchestration_loop.rs:207`. No signature change needed here — the batch read step is just another Step variant dispatched in `handle_program_step()`.

## Acceptance Criteria

1. `cargo build` compiles with no errors
2. `handle_batch_step()` correctly:
   - Acquires content for each item based on `ItemKind` discriminator
   - Reads files from disk for `FilePath` items
   - Retrieves shell output segments from artifacts for `ShellOutput` items
   - Re-runs constrained search for `SearchPage` items
   - Retrieves text blocks from artifacts for `TextBlock` items
   - Calls summarizer LLM with batch content + prompt
   - Stores per-batch summaries as artifacts
   - Produces aggregated final summary
   - Handles acquisition failures gracefully (continues to next item)
3. Batch planner fires automatically when:
   - Complexity is INVESTIGATE or higher
   - `needs_evidence` is true
   - Multiple Read steps exist in the program
   - Items exceed available context budget
4. Semantic grouping: same-directory FilePath items appear consecutively within batches. Non-file items preserve their original positions.
5. Progressive summarization: batch N summaries reference findings from batches 1..N-1
6. Transcript visibility: batch events appear as `BATCH` and `BATCH_PLAN` meta events
7. `cargo test` passes all existing and new tests
8. Manual test: ask Elma to analyze a project with 20+ files. Verify it creates batches, processes them sequentially, and produces a coherent final answer.

## Risk Assessment

- **Summarizer quality**: The summarizer LLM is called once per batch. If it produces poor summaries, later batches lose context. Mitigation: the progressive summarization prompt includes prior batch summaries so the LLM can reference them.
- **Summarizer timeout**: If a batch has many large items, the summarizer may timeout. Mitigation: respect the summarizer's `timeout_s` config. If a batch is too large, split it further (the planner handles this via `max_items_per_batch`).
- **Token budget accuracy**: If tiktoken-rs underestimates tokens (unlikely with BPE), a batch might exceed the context window. Mitigation: the `response_buffer_tokens` parameter provides a safety margin.
- **Step budget exhaustion**: The autonomous loop has `max_steps: 8`. A single `Step::Batch` counts as one step regardless of batch count.
- **Shell output artifacts**: `ShellOutput` items rely on prior shell execution having stored output in `state.artifacts`. If the artifact isn't found, the item fails gracefully with an error message. The batch planner should only create `ShellOutput` items after the shell command has executed and its output is known to exist.
- **Search re-execution**: `SearchPage` items re-run the search command. This is idempotent but may produce slightly different results if files change between batches. Acceptable — the executor is stateless and the search is run on demand.
