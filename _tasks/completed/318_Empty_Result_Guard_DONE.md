# Task 318: Empty Result Guard (Proposal 009)

**Status:** pending  
**Proposal:** [docs/_proposals/009-empty-result-guard.md](../../docs/_proposals/009-empty-result-guard.md)  
**Depends on:** None  

## Summary

Inject `"(empty result)"` placeholder in `tool_loop.rs` when a non-respond tool returns `ok: true` with empty content. Prevents small models from misinterpreting empty success as "need to retry."

## Why

Empty success results are ambiguous — the model can't distinguish "tool worked but produced no output" from "tool call was lost." Small models (especially 4B-class) retry the same command when they see empty output. Goose uses this exact `"(empty result)"` injection pattern and it's proven effective.

## Implementation Steps

1. In `tool_loop.rs`, after tool result is pushed to messages (~line 1023), add guard: if `result.ok && result.content.trim().is_empty() && tc.function.name != "respond"`, inject `"(empty result)"`
2. Build and test with `prompt_01`

## Success Criteria

- [x] Empty `ok: true` results from non-respond tools produce `"(empty result)"`
- [x] `respond` tool results never altered
- [x] `cargo build` succeeds
