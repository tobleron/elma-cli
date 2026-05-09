# Task 770: Reopen Architecture Documentation Drift After Round 9 Audit

## Type
Documentation / Reliability / Hardening

## Severity
High

## Scope
Documentation, architecture map, developer guidance

## Problem
The revision audit found that architecture documentation is still materially stale after Task 756 was archived as complete.

Evidence from 2026-05-07:

```text
$ wc -l src/app_chat_loop.rs src/tool_loop.rs src/work_graph.rs docs/DEVELOPMENT.md docs/ARCHITECTURE.md
    1779 src/app_chat_loop.rs
    3868 src/tool_loop.rs
     518 src/work_graph.rs
     457 docs/DEVELOPMENT.md
    1025 docs/ARCHITECTURE.md
```

But `docs/DEVELOPMENT.md` still claims:

```text
cargo test                          # All tests (600+)
| `cargo test` | Unit tests across all modules (600+ tests) |
├── app_chat_loop.rs                 # Main interactive chat loop (42K+ lines)
├── tool_loop.rs                     # Tool-calling execution loop (1.5K lines)
```

The real test suite currently runs 1683 tests, not "600+". `src/app_chat_loop.rs` is 1779 lines, not "42K+ lines", and `src/tool_loop.rs` is 3868 lines, not "1.5K lines".

`docs/ARCHITECTURE.md` also still describes route selection as heuristic:

```text
│ 3. Route decision (heuristic) │
```

and includes a `Route Decision` section that says active routing uses a conservative line-length heuristic in `app_chat_loop.rs`. Current code contradicts that claim:

```rust
// app_chat_loop.rs
// LLM-driven route inference replaces the old line.len() < 30 heuristic.
```

This violates AGENTS.md and Architectural Rule 5: repo-specific claims must be grounded in current workspace evidence. Stale architecture docs make future agents create poor tasks, misunderstand active routing, and mis-estimate module risk.

## Root Cause
Task 756 correctly identified documentation drift but did not leave a durable verification gate that prevents drift from returning. Subsequent Round 9 work changed runtime architecture, tests, and module line counts without refreshing `docs/ARCHITECTURE.md` and `docs/DEVELOPMENT.md`.

## Proposed Solution
Phase 1: Add a documentation facts script.
- Create or update `_scripts/update_docs_line_counts.sh` so it reports actual `wc -l` counts for documented Rust modules.
- Add a check mode that fails when documented line-count claims diverge from current source.

Phase 2: Correct stale docs.
- Update `docs/DEVELOPMENT.md` test-count references from "600+" to a non-stale phrasing or generated value.
- Correct `app_chat_loop.rs`, `tool_loop.rs`, and other module line descriptions.
- Rewrite the `docs/ARCHITECTURE.md` route decision section to describe the current model-inferred route path and any deterministic confidence fallback accurately.

Phase 3: Add a regression gate.
- Add a focused test or script invocation that can be run in the verification ladder to catch stale line-count and route-architecture claims.
- Document the command in `docs/DEVELOPMENT_GUIDELINES.md`.

## Acceptance Criteria
- [ ] `docs/DEVELOPMENT.md` no longer contains stale hardcoded "600+" test-count claims.
- [ ] Documented module line counts are either removed, generated, or verified against `wc -l`.
- [ ] `docs/ARCHITECTURE.md` no longer says the active route decision is the old line-length heuristic.
- [ ] A repeatable docs drift check exists and fails on stale line-count claims.
- [ ] The verification plan passes.

## Verification Plan
- Unit/script check: run the docs drift check and confirm it passes.
- Manual spot check: compare at least 10 documented module references with `wc -l` and actual file existence.
- Search check: `rg -n "600\\+|42K\\+|Route Decision|heuristic" docs/DEVELOPMENT.md docs/ARCHITECTURE.md`.
- Build/test safety: `cargo check -q` and `cargo test -q`.

## Dependencies
- `_scripts/`
- `docs/ARCHITECTURE.md`
- `docs/DEVELOPMENT.md`
- `docs/DEVELOPMENT_GUIDELINES.md`

## Notes
Do not turn this into a broad rewrite. Remove brittle numeric claims where possible, and generate or verify any remaining numbers. This task improves truthfulness and reliability without expanding Elma's feature surface.
