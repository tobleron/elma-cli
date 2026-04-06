# Task 121: Command Budget & Rate Limiting

## Priority
**P2 — Safety**
**Created:** 2026-04-05
**Status:** Pending
**Dependencies:** Task 116 (Destructive Command Detection)

## Problem

A model in a confused loop could execute unlimited destructive commands. No session-level budget exists.

## Scope

### 1. Per-Session Command Budget
- Track counts: safe commands (unlimited), caution commands (max 20/session), dangerous commands (max 5/session)
- When budget exhausted: block further commands of that type, require `/reset-budget` or session restart
- Budget resets on `/reset` or new session

### 2. Rate Limiting
- Max 10 shell commands per turn (model can only call tools so fast)
- Max 3 destructive commands per turn
- Throttle: 100ms between commands (prevents runaway loops)

### 3. Integration Points
- `src/command_budget.rs` (new) — budget tracking
- `src/tool_calling.rs` — `exec_shell()` checks budget before execution
- `src/app.rs` — `/reset-budget` command

## Design Principles
- **Small-model-friendly:** Clear budget messages ("3/5 dangerous commands used this session")
- **Non-blocking for safe ops:** Read commands never consume budget
- **Transparent:** Budget status shown in trace, not hidden

## Verification
1. `cargo build` clean
2. `cargo test` — budget tracking, rate limiting, reset behavior
3. Real CLI: 6+ destructive commands → 6th blocked with budget message

## Acceptance Criteria
- [ ] Session tracks dangerous command count
- [ ] Budget exhaustion blocks further destructive commands
- [ ] Rate limiting prevents runaway loops
- [ ] `/reset-budget` restores budget
- [ ] Safe commands never blocked by budget
