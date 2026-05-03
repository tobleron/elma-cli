# Task 491: Source-Agent Command And Slash Action Parity

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Dependencies:** Task 386, Task 388
**References:** source-agent parity: Claude Code, Crush, Roo-Code, Hermes command systems

## Objective

Inventory useful source-agent slash commands and convert high-value ones into Elma command actions without bloating prompts or adding keyword routing.

## Scope

Candidate commands include:

- model/session/status commands
- export/copy/summary commands
- tool/debug commands
- doctor/logs commands
- skills/tools commands
- review/commit helper commands

## Implementation Plan

1. Extend the Task 386 parity matrix with a command/action section.
2. Map commands to existing Elma UI commands where possible.
3. Create pending follow-up tasks only for commands that add reliability or practical usefulness.
4. Ensure command mode preserves the status footer rule.
5. Add tests for command parsing and transcript-visible action results.

## Verification

```bash
cargo test command
cargo test ui
cargo build
```

## Done Criteria

- High-value source-agent commands are mapped to Elma equivalents or documented as non-goals.
- Commands do not bypass orchestration, permissions, or transcript visibility.
- No prompt-core changes are made.

