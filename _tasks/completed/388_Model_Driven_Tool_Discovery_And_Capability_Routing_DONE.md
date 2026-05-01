# Task 388: Model-Driven Tool Discovery And Capability Routing

**Status:** Pending
**Priority:** HIGHEST
**Estimated effort:** 2-3 days
**Dependencies:** Task 378, Task 387
**References:** objectives.md tool awareness, AGENTS.md Rules 1 and 4

## Objective

Make Elma discover and use tools like a normal agent: if the default tool set is insufficient, Elma should ask the tool registry for capabilities, load the best matching tools, and retry with a smaller, more relevant tool set.

## Problem

`tool_search` exists, but discovery must become part of orchestration rather than a tool the model may or may not remember to call. The model should not receive every tool at once, and routing must not rely on hardcoded query words.

## Implementation Plan

1. Add a capability request intel unit with a strict simple JSON output:
   - `capability`: one sentence
   - `confidence`: enum or score
   - `reason`: one sentence
2. Use `DynamicToolRegistry::search` to find matching tools from capability text.
3. Load at most five matching tools for a retry turn.
4. Prefer tools with `rust_native` and `offline_capable` metadata from Task 387.
5. Emit transcript rows for discovery query, selected tools, unavailable prerequisites, and fallback.
6. Add a no-result path that decomposes the request instead of hallucinating unsupported capability.

## JSON Constraint

The discovery intel unit must comply with Task 378:

```json
{"capability":"...","confidence":"high","reason":"..."}
```

No nested objects and no more than three required fields.

## Verification

```bash
cargo test tool_search
cargo test tool_registry
cargo test routing
cargo test orchestration
cargo build
```

Manual probes:

- Ask for a file search with only default tools.
- Ask for an operation that requires `patch`.
- Ask for a network fetch while network tools are disabled.

## Done Criteria

- Tool discovery is invoked by orchestration when capability is missing.
- Discovery results are capped, ranked, and rust-first.
- Missing or disabled tools produce visible, recoverable outcomes.
- No keyword routing is introduced.

