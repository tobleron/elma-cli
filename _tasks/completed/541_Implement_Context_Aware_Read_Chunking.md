# Implement Context-Aware Read Chunking

## Problem
In session `s_1777807006_86051000`, the `read` tool dumped massive amounts of text directly into the tool result context:
- `read (161583 chars → ...)`
- `read (139158 chars → ...)`

This overwhelmed the local model (`Huihui-Qwen3.5-4B`), eventually leading to a hard crash during the finalization stage: `trace: finalization_failed_nonfatal stage=evidence error=error decoding response body`. This violates the system's core philosophy of being "small-model-friendly" and protecting the context window.

## Required Actions
1. **Enforce Read Limits:** The `read` tool must enforce a strict character limit per read request (e.g., max 8000 chars) natively.
2. **Implement Chunking/Pagination via Batch Planner:** Implement the architectural solution described in **Task 546 (Batch Planner Intel Unit)**, **Task 547 (Execution Loop Integration)**, and **Task 548 (tiktoken-rs)** to correctly batch and summarize items instead of trying to stuff them all into one turn.
3. **Guard Against Context Bloat:** Add safety checks in the tool response envelope that prevent massive payloads from being injected blindly into the LLM context.
