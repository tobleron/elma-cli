# Task 382: Keyword Heuristic Decomposition Audit

## Backlog Reconciliation (2026-05-02)

Superseded for current pending work by Task 453. Use this file only as historical audit context; implementation should follow Task 453 and the current no-keyword-matcher rule.


**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 1-2 days
**Dependencies:** Task 376 (must complete first — handles the `line.len()` heuristic)
**References:** AGENTS.md Rule 1, _masterplan.md Task 355

## Problem

AGENTS.md Rule 1 mandates: "Routing, classification, and behavior selection must never use hardcoded word triggers." The `line.len() < 30` heuristic (fixed in Task 376) was the most impactful violation, but there may be others lurking in routing, finalization, compaction, and tool choice.

## Objective

Audit the entire codebase for semantic keyword/pattern heuristics and either:
1. Replace them with metadata, confidence scores, or focused intel units (following Rule 1)
2. Explicitly document and test them if they are safety/parser heuristics that cannot be removed

## Audit Checklist

### Routing
- [x] `app_chat_loop.rs:846` → `line.len() < 30` → **FIXED by Task 376**
- [ ] `routing_infer.rs:374` → `extract_first_path_from_user_text` → path-scoped routing heuristic
- [ ] `routing_infer.rs:380-386` → `fallback_to_chat` based on entropy/margin thresholds (acceptable — confidence-based)
- [ ] `routing_infer.rs:248` → `should_short_circuit_chat_route` — LLM-consensus-based (acceptable)

### Finalization
- [ ] `app_chat_loop.rs:963` → `is_trivial` based on "CHAT" + "reply_only" string match — **FIXED by Task 377**
- [ ] Any remaining string-matching in final answer extraction

### Compaction
- [ ] `effective_history.rs` → compaction triggers
- [ ] Any length-based compaction thresholds

### Tool Choice
- [ ] Tool selection logic for automatic tool discovery
- [ ] Any hardcoded tool preferences based on query text

### Evidence
- [ ] `evidence_ledger.rs` → claim-to-evidence mapping
- [ ] Any keyword-based evidence sufficiency checks

## Implementation Plan

1. Run grep audit for pattern-matching on user input: `line.contains`, `input.contains`, `line.find`, `line.starts_with`, string matching on user message content
2. Classify each hit as: SEMANTIC (must replace), SAFETY (must keep + test), PARSER (must keep + test)
3. For each SEMANTIC hit: design replacement (intel unit, metadata check, confidence gate)
4. For each SAFETY/PARSER hit: add explicit test
5. Generate audit report in task file

## Files to Search

```bash
rg "\.contains\(|\.starts_with\(|\.ends_with\(|\.eq_ignore_ascii_case" src/ --include '*.rs' | grep -v test | grep -v "_knowledge_base"
```

## Output

Task file must be updated with the full inventory of findings and their dispositions (REPLACED, KEPT+TESTED, or BENIGN).

## Verification

```bash
cargo build
cargo test
```

**Audit output**: Document every keyword heuristic found, its disposition, and the replacement (if any).
