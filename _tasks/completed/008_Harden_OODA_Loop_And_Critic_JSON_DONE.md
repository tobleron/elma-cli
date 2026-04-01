# Task 008: Harden OODA Loop And Critic JSON

## Priority
**P1 - HIGH** (-50% critic parse errors, better retry diversity)

## Status
**PENDING** — Requires updates based on Task 044/045 architecture

## Context
Task 044/045 already implemented:
- ✅ `IntelUnit` trait with pre-flight/post-flight validation
- ✅ `JsonErrorHandler` with circuit breaker
- ✅ Critics use `IntelUnit` pattern with automatic fallbacks
- ✅ `chat_json_with_repair_timeout()` handles JSON parsing with repair

**Remaining issues to address:**
- Strategy fixation during retry loops (repeats same failed command)
- Occasional critic JSON corruption (thinking tokens leak into output)
- Reasoning audit trail needs clean token tracking

## Objective
Improve retry loop diversity and critic JSON reliability.

## Technical Tasks

### 1. Adaptive Repair (Strategy Diversification)

**Problem:** During retry (triggered by `outcome_verification`), Elma repeats the exact same command instead of rethinking the approach.

**Solution:** Update `request_recovery_program()` in `src/orchestration_helpers.rs` to:
- Track previously failed commands/steps
- Explicitly forbid failed steps in recovery prompt
- Encourage alternative approaches (e.g., `ls` or `find` instead of failed `cat`)

**Implementation:**
```rust
// In request_recovery_program():
// Add failed_steps parameter to prompt:
let prompt = format!(
    "{}\n\nRECOVERY MODE:\n- Previous failed steps: {:?}\n- DO NOT repeat these steps\n- Try alternative approaches\n...",
    orchestrator_cfg.system_prompt,
    failed_steps.iter().map(|s| s.cmd).collect::<Vec<_>>()
);
```

### 2. Critic JSON Extraction Hardening

**Problem:** Critics (`logical_reviewer`, `efficiency_reviewer`, `risk_reviewer`) occasionally produce invalid JSON or mix thinking tokens with JSON output.

**Current State:**
- Critics use `chat_json_with_repair()` for parsing
- `chat_json_with_repair_timeout()` already extracts JSON from mixed output

**Solution:**
- Update critic prompts to enforce stricter JSON-only output
- Add post-flight validation in `IntelUnit` implementations for critics
- Consider adding `#[serde(deny_unknown_fields)]` for stricter parsing

**Prompt Update Example:**
```toml
# critic.toml
system_prompt = """
Return ONLY one valid JSON object. No prose. No thinking tokens. No code fences.

Schema:
{
  "status": "ok" | "retry",
  "reason": "one short sentence"
}

Rules:
- Output MUST be valid JSON
- Do not include thinking tokens or reasoning
- Do not use markdown code fences
- If uncertain, return status="ok" with conservative reason
"""
```

### 3. Reasoning Path Sanitization

**Problem:** `memory_gate_status=skip reason=unclean_reasoning_fallback` shows reasoning extraction needs hardening.

**Solution:**
- Ensure `reasoning_audit.jsonl` tracks clean tokens only
- Separate thinking content from parseable JSON content
- Add `thinking_content` field to track separated reasoning

**Implementation:**
```rust
// In memory_gate or formula_memory saving:
let clean_reasoning = extract_clean_reasoning(raw_output);
let reasoning_audit = ReasoningAudit {
    clean_tokens: clean_reasoning,
    thinking_content: separated_thinking,
    json_content: parsed_json,
};
```

## Acceptance Criteria
- [ ] Retry loops choose different commands (not same failed command)
- [ ] Zero `logical_review_parse_error` in traces
- [ ] Zero `efficiency_review_parse_error` in traces
- [ ] `memory_gate_status` no longer skips due to "unclean_reasoning"
- [ ] Critic prompts enforce JSON-only output
- [ ] Failed steps are tracked and forbidden in recovery

## Verification
1. Run task that will fail (e.g., read non-existent file)
2. Verify retry chooses different approach (not same failed command)
3. Check `trace_debug.log` for zero critic parse errors
4. Check `memory_gate_status` in session trace

## Dependencies
- ✅ Task 044 (Execution Ladder) — Provides level-aware validation
- ✅ Task 045 (Intel Units) — Provides trait pattern for critics

## Files to Modify
- `src/orchestration_helpers.rs` — `request_recovery_program()` with failed step tracking
- `src/defaults_router.rs` — Critic prompts (stricter JSON enforcement)
- `src/reflection.rs` — Reasoning sanitization (if needed)
- `src/memory_gate.rs` — Clean token tracking

## Estimated Effort
4-6 hours

## Architecture Alignment
- ✅ IntelUnit trait pattern (Task 045)
- ✅ JSON error handler with circuit breaker (Task 003)
- ✅ Principle-based prompts (no hardcoded rules)
