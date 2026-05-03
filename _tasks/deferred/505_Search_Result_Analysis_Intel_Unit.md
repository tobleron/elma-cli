# Task 505: Search Result Analysis Intel Unit

**Status:** pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Primary surfaces:** `src/intel_units/intel_units_search_ranker.rs` (new), `src/tool_loop.rs` (tool execution path)
**Depends on:** Task 501 (batch planner — same pattern)

## Objective

Replace dumb truncation of large search results with intelligent file ranking. When `search` returns >100 results, instead of truncating alphabetically (which favors trivial files like `Cargo.lock` over important ones like `tool_loop.rs`), pass the raw result list to an intel unit that selects the top N most relevant files based on the user's question.

## How It Works

The current `search` tool executor at `tool_calling.rs` returns results as-is, truncated at a char count limit. This means long file lists are trimmed by position (alphabetical order), not relevance.

The fix: after search execution but before returning results to the model:

1. If search result count > 100 or result chars > 50KB, intercept the output
2. Call the SearchRanker intel unit with: (user_query, search_results_raw, max_count=180)
3. The intel unit returns a ranked list of the top 180 most relevant file paths
4. The search tool output is replaced with the ranked list
5. The model only sees relevant files in context

## Intel Unit Design

```rust
pub(crate) struct SearchRankerUnit { profile: Profile }

impl IntelUnit for SearchRankerUnit {
    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let query = context.extra("query")...;
        let results = context.extra("results")...;
        let max_count: usize = 180;

        let prompt = format!(
            "Given this user request: {query}\n\n\
             Rank these search results by relevance. Return up to {max_count} \
             file paths from most to least useful for answering the user.\n\n\
             {results}\n\n\
             Output format: one file path per line, no numbering or explanation."
        );

        let ranked = execute_intel_text_from_user_content(&context.client, &self.profile, prompt).await?;
        // Parse into lines, take max_count
        let paths: Vec<&str> = ranked.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).take(max_count).collect();
        Ok(IntelOutput::success(self.name(), json!({"paths": paths, "count": paths.len()}), 0.9))
    }
}
```

## Integration Point

The interception happens in `src/tool_calling.rs` search tool executor, after `ripgrep` or `glob` returns results but before the result is formatted for the model. If the result exceeds a threshold (configurable: `search_rank_threshold_count = 100`, `search_rank_threshold_chars = 50000`), route to the ranker intel unit.

## Files to Create/Modify

- `src/intel_units/intel_units_search_ranker.rs` (new — the intel unit)
- `src/intel_units/mod.rs` (register the new module)
- `src/tool_calling.rs` (intercept large search results, call ranker)

## Verification

- Search matching 500+ files should return max 180 ranked paths
- The ranked paths should start with the most semantically relevant
- `cargo build && cargo test && cargo clippy` passes
