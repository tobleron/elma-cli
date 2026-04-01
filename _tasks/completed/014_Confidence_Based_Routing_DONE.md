# Task 014: Implement Confidence-Based Routing with Pattern Fallbacks

## Priority
**P0 - CRITICAL** (Fixes root cause of misclassification)

## Status
**PENDING** — Ready for implementation

## Problem
Current classification relies on single-pass 3B model classification with no safety nets. Wrong classifications cascade to wrong execution, causing:
- Over-orchestration on simple chat (generates shell steps for "hello")
- Plan collapse on complex tasks (40+ identical steps)
- User frustration from wrong behavior

## Industry Context

**Research findings:**
- **NadirClaw:** Uses confidence threshold (0.06), defaults to complex model when uncertain
- **Open Interpreter:** No confidence thresholds, relies on user confirmation for everything
- **Aider:** No classification, relies on git rollback for mistakes

**Elma's opportunity:** Confidence-based workflow selection (unique in CLI agents)

## Objective

Implement Option F (Combined approach):
1. Pattern matching for obvious chat cases
2. Confidence-based fallback for uncertain classifications
3. Keep Fix 1 (formula-level alignment) — already done

## Implementation Steps

### Step 1: Add Obvious Chat Pattern Detection (10 min)

**File:** `src/orchestration_planning.rs`

**Add function:**
```rust
fn is_obvious_chat(input: &str) -> bool {
    let lower = input.to_lowercase();
    lower.starts_with("hello") || 
    lower.starts_with("hi ") ||
    lower.starts_with("hey ") ||
    lower.contains("how are you") ||
    lower.contains("who are you") ||
    (input.len() < 30 && !lower.contains("file") && !lower.contains("run"))
}
```

**Integration:** Check before classification, force CHAT route if matched.

### Step 2: Add Confidence-Based Fallback (15 min)

**File:** `src/routing_infer.rs` or `src/orchestration_planning.rs`

**Logic:**
```rust
if entropy > 0.8 || margin < 0.15 {
    // Model is uncertain - default to CHAT (safe, no execution)
    return RouteDecision {
        route: "CHAT".to_string(),
        confidence: 0.5,
        ...
    };
}
```

**Thresholds:**
- Entropy > 0.8 = high uncertainty
- Margin < 0.15 = close call between top choices

### Step 3: Keep Fix 1 (Formula-Level Alignment)

**Status:** ✅ Already implemented in Task 007/013

**Location:** `src/orchestration_planning.rs`

**Function:** `derive_planning_prior_with_ladder()` enforces formula-level alignment.

## Acceptance Criteria

- [ ] Obvious chat patterns (hello, who are you) always route to CHAT
- [ ] Low-confidence classifications default to CHAT (safe)
- [ ] High-confidence classifications proceed normally
- [ ] Formula-level alignment validates formula matches level
- [ ] S000A (Chat Baseline) passes — no shell commands for greetings
- [ ] S001 (Connectivity) passes — no infinite token repetition
- [ ] S002/S005/S006 pass — no plan collapse (step limits enforced)

## Files to Modify

| File | Change | Lines |
|------|--------|-------|
| `src/orchestration_planning.rs` | Add `is_obvious_chat()` + integration | ~40 |
| `src/routing_infer.rs` OR `src/orchestration_planning.rs` | Add confidence fallback | ~20 |
| `src/program_policy.rs` | Already has Fix 1 + step limits | ✅ Done |
| `src/app_chat_helpers.rs` | Already has truncation | ✅ Done |

## Expected Impact

| Metric | Before | After |
|--------|--------|-------|
| **Chat classification accuracy** | ~60% | ~90% |
| **Over-orchestration rate** | ~40% | <10% |
| **Plan collapse incidents** | ~30% | <5% |
| **User confirmation prompts** | 0 (fully automated) | 0 (still automated) |

## Verification

### Unit Tests
```bash
cargo test is_obvious_chat
cargo test confidence_fallback
```

### Stress Tests
```bash
./run_stress_tests_cli.sh
```

**Expected results:**
- S000A (Chat): CHAT route, reply_only formula, 1 step ✅
- S000B-S000I (Primitives): Correct route, step limits enforced ✅
- S001 (Connectivity): No infinite repetition (truncation) ✅
- S002/S005/S006: No plan collapse (step limits) ✅

## Dependencies
- ✅ Task 007 (Classification decoupled)
- ✅ Task 010 (Strategy chains)
- ✅ Task 011 (Guardrails)
- ✅ Task 012 (Atomic intel units)
- ✅ Task 013 (JSON pipeline)

## Architecture Alignment
- ✅ **Elma philosophy** — "soft guidance rather than hard constraints"
- ✅ **Industry best practice** — Confidence thresholds (NadirClaw, Rasa)
- ✅ **Automated safety** — No user confirmation needed
- ✅ **Low friction** — Confident classifications execute immediately

## Notes

### Why CHAT as Safe Default?

When uncertain, default to CHAT because:
- CHAT = reply_only formula = no execution
- User can rephrase if they actually wanted execution
- Safer to under-execute than over-execute

### Threshold Tuning

Start conservative:
- Entropy > 0.8 = uncertain
- Margin < 0.15 = uncertain

Adjust based on stress test results:
- Too many false negatives? → Lower thresholds
- Still over-orchestrating? → Raise thresholds

### Industry Validation

**NadirClaw approach:** "Cheaper to over-serve than under-serve"
- They default to complex model when uncertain
- We default to CHAT (safe) when uncertain
- Both use confidence thresholds — same principle, different defaults

**Open Interpreter approach:** User confirms everything
- High safety, high friction
- We offer high safety, low friction (automated)
