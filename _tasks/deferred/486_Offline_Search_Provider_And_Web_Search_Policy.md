# Task 486: Offline Search Provider And Web Search Policy

**Status:** Pending
**Priority:** LOW
**Estimated effort:** 2-4 days
**Dependencies:** Task 485, Task 465
**References:** source-agent parity: web search tools with Elma offline-first policy

## Objective

Provide a search abstraction that prioritizes offline sources first and only uses web search when explicitly enabled and necessary.

## Implementation Plan

1. Define `search_provider` metadata:
   - local workspace
   - local memory/index
   - local docs cache
   - optional web
2. Implement provider selection without keyword routing.
3. Keep web search disabled by default and governed by the same network policy as fetch/browser.
4. Return source provenance and freshness metadata.
5. Add transcript rows when the system chooses offline search, optional web search, or no search.

## Verification

```bash
cargo test search_provider
cargo test fetch_policy
cargo test evidence
cargo build
```

## Done Criteria

- Offline search is always tried before web search when suitable.
- Web search cannot run without explicit enablement.
- Search results include provenance and evidence refs.

