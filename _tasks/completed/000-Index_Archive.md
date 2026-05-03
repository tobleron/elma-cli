# Pending Tasks Index — Elma CLI Audit & Implementation Plan

## Audit Summary

The elma-cli codebase (193+ source files, ~80k+ lines of Rust) is a sophisticated CLI agent system with extensive subsystems: tool calling, streaming, execution ladders, evidence ledgers, shell preflight, permission gates, auto-compaction, session management, and multiple orchestration pathways.

### Key Strengths Found
- Well-structured tool registry system (`elma-tools` crate) with policy metadata
- Robust shell preflight system with risk classification and dry-run previews
- Comprehensive stop policy with multiple stagnation detection mechanisms
- Evidence ledger with staleness tracking and grounding enforcement
- Good use of Rust's type system for most public APIs
- Active commitment to small-model-friendliness in philosophy

### Critical Architectural Concerns
1. **Dual orchestration paths create divergence risk**: The codebase maintains two parallel orchestration systems — the old Maestro-based pipeline (`build_program_from_maestro`, `orchestrate_instruction_once`) and the new tool-calling pipeline (`run_tool_calling_pipeline`, `run_tool_loop`). Only one is active, but both systems pull in dependencies and multiply the maintenance surface. The old path has hardcoded tool capability strings and fixed JSON schemas that haven't been updated.

2. **Protected path blocking silently disabled**: The `check_protected_paths()` function in `shell_preflight.rs:634-639` returns `None` unconditionally, with a comment that "protected path blocking removed." This means Elma can `rm -rf src/`, `rm Cargo.toml`, or `rm -rf .git/` with only the standard risk classification (which classifies `rm` as Dangerous but the permission gate can be bypassed in safe_mode=off). This is a serious safety regression.

3. **Global mutable state is pervasive**: The codebase uses `static OnceLock<Mutex<T>>` and `static OnceLock<RwLock<Option<T>>>` extensively for session state, evidence ledgers, permission caches, command budgets, execution profiles, and more. This makes testing difficult, creates hidden coupling, and prevents multiple concurrent sessions.

4. **Monolithic `tool_calling.rs` (3345 lines)**: Contains all 30+ tool executors in a single file with a massive match statement, making it hard to audit individual tool safety, add tests per tool, or reason about tool lifecycle.

5. **Module declarations without implementations**: Four modules are declared in `main.rs` but have no corresponding files: `orchestration_loop_helpers.rs`, `orchestration_loop_reviewers.rs`, `orchestration_loop_verdicts.rs`, and `orchestration_retry_tests.rs`. These are either dead declarations or incomplete refactoring.

6. **Execution-level determinism rules violated by hardcoded keyword matchers**: Despite AGENTS.md Rule 1 ("Do Not Turn Elma Into A Keyword Matcher"), `execution_ladder/depth.rs` contains functions like `requests_planning()`, `requests_strategy()`, `requests_bulk()`, `requests_multi_step_verbs()` that use `str.contains()` on hardcoded indicator strings. This directly violates the "no keyword matchers" rule.

7. **Unused or underutilized crate dependencies**: The Cargo.toml includes crates like `djvu-rs`, `mobi`, `epub` whose extraction integration isn't fully wired (per `docs/_proposals/004-crate-reconciliation.md` and `005-extraction-integration.md`).

8. **Path validation inconsistency**: Tools like `exec_read`, `exec_ls`, `exec_edit`, `exec_write` reject absolute paths, but `exec_shell` passes commands directly to the shell after preflight — there's no equivalent path sandboxing for `shell` tool targets. The `resolve_path()` function in `shell_preflight.rs` resolves relative paths but accepts absolute paths verbatim.

9. **TUI and model-calling tightly coupled**: The `tool_loop.rs` streaming functions interleave SSE parsing, UI event emission, thinking block detection, and content accumulation in a single function (`request_tool_loop_model_turn_streaming`, ~200 lines). This makes it hard to test the SSE parsing independently or swap the UI layer.

10. **Limited defensive parsing of model output**: Tool call arguments are parsed with `serde_json::from_str()` and on failure only the raw string preview is included in the error. There's no structured JSON repair attempt, no schema validation of argument shapes before execution, and no recovery for near-miss JSON (unquoted strings, trailing commas).

## Ordered Task List

| # | Title | Priority | Category |
|---|-------|----------|----------|
| 550 | Remove or fully deprecate legacy Maestro orchestration pipeline | Critical | architecture |
| 551 | Restore protected path blocking with explicit allowlist | Critical | security |
| 552 | Split monolithic tool_calling.rs into per-tool executor modules | Critical | refactoring |
| 553 | Implement strict tool argument JSON validation | Critical | validation |
| 554 | Replace global mutable state with session-scoped ownership | High | state management |
| 555 | Create or remove missing module declarations | High | architecture |
| 556 | Replace hardcoded keyword matchers in execution_ladder/depth.rs | High | architecture |
| 557 | Add path sandboxing for shell tool targets | High | security |
| 558 | Decouple SSE streaming from TUI event emission | High | refactoring |
| 559 | Implement defensive JSON parsing with repair for all model outputs | Critical | parsing |
| 560 | Centralize tool execution lifecycle with pre/post hooks | High | architecture |
| 561 | Add comprehensive model-output fuzzing and repair tests | High | testing |
| 562 | Create bounded Finite State Machine for agent lifecycle | High | state management |
| 563 | Add integration tests for full tool-calling pipeline | High | testing |
| 564 | Normalize error types across all modules | Medium | error handling |
| 565 | Standardize tool result envelope (exit_code, timed_out, signal_killed) | Medium | tool calling |
| 566 | Wire unused crate integrations (djvu, mobi, epub) | Medium | architecture |
| 567 | Audit and harden evidence ledger consistency | Medium | validation |
| 568 | Add context window budget accounting with user visibility | Medium | LLM I/O |
| 569 | Separate read-only vs mutating tool categories in all UI/schema layers | Medium | tool calling |
| 570 | Implement bounded retry with exponential backoff for API calls | Medium | error handling |
| 571 | Implement session snapshot and rollback system | Medium | state management |
| 572 | Add golden-tests for tool output formats | Medium | testing |
| 573 | Implement structured logging with session-scoped tracing | Medium | logging/observability |
| 574 | Create canonical execution taxonomy document | Medium | documentation |
| 575 | Refactor session state persistence to be transactional | High | state management |
| 576 | Add deterministic dependency injection for testable components | Medium | testing |
| 577 | Add ANSI escape sequence sanitization boundary | Medium | sanitization |
| 578 | Consolidate duplicate normalize_shell_signal implementations | Medium | refactoring |
| 579 | Add offline capability detection and graceful degradation | Medium | architecture |
| 580 | Document validation boundaries in architecture docs | Medium | documentation |
| 581 | Remove dead `_PROTECTED_DIRS` and `_PROTECTED_FILES` constants | Low | refactoring |
| 582 | Add regression tests for stagnation and stop policy | Medium | testing |
| 583 | Implement config validation at startup with clear error messages | Medium | config |
| 584 | Add TUI rendering tests with insta snapshots | Medium | testing |
| 585 | Create architecture decision record for tool calling vs orchestration | Low | documentation |

## Priority Distribution

- **Critical**: 6 tasks
- **High**: 12 tasks
- **Medium**: 14 tasks
- **Low**: 3 tasks

## Recommended Implementation Phases

### Phase 1: Foundation & Safety (Tasks 550–554)
Establish a safe, maintainable baseline. Remove legacy code, restore protections, split monoliths, add validation, and clean up global state.

### Phase 2: Hardening (Tasks 555–563)
Fix architectural violations, add defensive parsing, normalize errors, add missing modules, sandbox paths, decouple layers, and create the execution lifecycle.

### Phase 3: Testability & Observability (Tasks 564–576)
Integration tests, golden tests, model output fuzzing, structured logging, context budget visibility, and FSM-based state management.

### Phase 4: Polish & Documentation (Tasks 577–585)
Documentation, config validation, TUI tests, cleanup, offline capability, and architecture decisions.

## Top 10 Highest-Impact Fixes

1. **Task 552**: Split tool_calling.rs — every tool executor is in one file; splitting enables independent testing per tool
2. **Task 551**: Restore protected paths — prevents accidental workspace destruction
3. **Task 550**: Remove legacy orchestration — eliminates ~3000 lines of dead code and reduces complexity
4. **Task 553**: Tool argument validation — prevents model hallucination from reaching dangerous execution
5. **Task 556**: Replace keyword matchers — fixes AGENTS.md Rule 1 violation
6. **Task 554**: Session-scoped state — enables testing and prevents state leakage
7. **Task 559**: Defensive JSON parsing — prevents silent failures from malformed model output
8. **Task 558**: Decouple SSE/TUI — enables independent testing of both layers
9. **Task 562**: FSM for agent lifecycle — makes state transitions explicit and auditable
10. **Task 557**: Path sandboxing for shell — closes the largest remaining path traversal gap

## Top 10 Highest-Risk Areas

1. `shell_preflight.rs:634-639` — Protected path blocking is commented out
2. `tool_calling.rs` — 3345-line monolith with filesystem operations buried deep
3. Global `OnceLock<Mutex<...>>` statics in 15+ modules — untestable state
4. `prompt_core.rs` — System prompt is minimal; may not guide small models effectively
5. `tool_loop.rs` — SSE + TUI coupling prevents independent verification
6. `execution_ladder/depth.rs` — Keyword matchers violate system philosophy
7. `orchestration_core.rs` — Dead orchestration path with hardcoded capability strings
8. `session_store.rs` — SQLite session storage with unbounded growth potential
9. `permission_gate.rs` — Non-interactive mode auto-denies commands from PTY
10. `json_parser.rs` / `json_repair.rs` — May not handle all small-model JSON errors

## Dependency Notes

- Task 552 (split tool_calling.rs) should be done BEFORE Task 557 (path sandboxing) and Task 560 (execution lifecycle) because those modify tool executors
- Task 554 (session-scoped state) is a prerequisite for Task 562 (FSM)
- Task 550 (remove legacy orchestration) unblocks cleaner architecture for Tasks 558, 560
- Task 559 (defensive JSON parsing) should be done BEFORE Task 553 (tool argument validation) since validation depends on robust parsing
- Task 556 (replace keyword matchers) should precede Task 562 (state machine) since the matchers influence state transitions
