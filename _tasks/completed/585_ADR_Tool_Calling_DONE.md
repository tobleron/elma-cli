# 585 — Create Architecture Decision Record for Tool Calling vs Orchestration

- **Priority**: Low
- **Category**: Documentation
- **Depends on**: 550 (orchestration removal)
- **Blocks**: None

## Problem Statement

The codebase underwent a significant architectural shift from the Maestro-based orchestration pipeline to the direct tool-calling pipeline. However, the rationale for this shift is not documented in a single decision record. Understanding why the change was made is important for:

1. Future contributors who encounter references to the old system
2. Deciding whether to invest in the new pipeline or restore the old one
3. Understanding the trade-offs made

## Recommended Target Behavior

Create `docs/decisions/001-tool-calling-over-orchestration.md` following the Architecture Decision Record (ADR) format:

```markdown
# ADR 001: Direct Tool Calling over Maestro Orchestration

## Status
Accepted

## Context
The original architecture used a two-phase approach:
1. Maestro intel unit generates text instructions
2. Orchestrator intel unit transforms instructions into JSON Step objects

## Decision
Replace with direct tool-calling: model receives tools directly and calls them.

## Rationale
- Eliminates one model round-trip (reduces latency)
- Model plans and executes in one turn (fewer context switches)
- Smaller total prompt size (no orchestration prompt)
- Better for small models (simpler contract)

## Consequences
- Model must plan and execute without intermediate planning step
- Complexity assessment must be done before tool calling, not during
- Work graph becomes advisory rather than prescriptive
- Legacy orchestration code must be maintained or removed
```

## Source Files That Need Modification

- `docs/decisions/` (new directory) — Create if it doesn't exist
- `docs/decisions/001-tool-calling-over-orchestration.md` (new)

## Acceptance Criteria

- ADR documents the architectural decision
- Includes context, decision, rationale, and consequences
- References source files for both old and new systems
- Includes date and status
