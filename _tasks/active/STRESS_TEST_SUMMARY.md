# Stress Testing Session Summary

## Date: 2026-04-02

## Tasks Completed

### ✅ Task 014: Confidence-Based Routing (Option F)
- Pattern matching for obvious chat (REMOVED - violated philosophy)
- Confidence-based fallback (entropy > 0.8 → CHAT) ✅ WORKING
- Formula-level alignment ✅ WORKING
- Step limits (max 12 steps) ✅ WORKING
- Duplicate step detection ✅ WORKING
- Output truncation (2000 chars) ✅ WORKING

### ✅ Task 045: Articulation Accuracy Migration
- **REVERTED** - Broke router classification
- Model was tuned on old terminology (CHAT, SHELL, etc.)
- New terms (CONVERSATION, TERMINAL_ACTION) not recognized
- All requests routed to CHAT with entropy=0.00

### ✅ Task 046: Auto-Tuning on Prompt Changes
- Prompt hash tracking ✅ IMPLEMENTED
- Change detection ✅ IMPLEMENTED
- Auto-tune trigger ⚠️ DISABLED (causes issues)
- **Keep for future use** - needs debugging

## Test Results

### S000A (Chat Baseline): ✅ PASSED
```
[CLASSIFY] speech=SHELL route=CHAT (entropy=1.42)
[PLAN] DIRECT → 1 steps
Elma: Hello! As a CLI agent, my primary goal is...
```

### S000B+ (Shell/Workflow tests): ⏳ TIMEOUT
- Classification working correctly
- Model hangs in retry loops
- Shell syntax issues (process substitution)
- 30-minute timeouts too long for practical testing

## Key Learnings

1. **Don't change router terminology without re-tuning**
   - Model output layer is fixed to trained vocabulary
   - Task 045 broke entire classification system

2. **Confidence-based fallback works**
   - High entropy → safe CHAT default
   - Prevents over-orchestration

3. **Step limits prevent plan collapse**
   - Max 12 steps enforced
   - Duplicate detection working

4. **Local model limitations**
   - 3B model is slow
   - Gets stuck in retry loops
   - Needs shorter timeouts for practical testing

## Recommendations

### Immediate
1. Reduce stress test timeout from 30min to 5min
2. Fix shell command syntax (avoid process substitution)
3. Debug prompt change detection (currently disabled)

### Future
1. Consider Task 045 ONLY with full re-tuning
2. Add model response timeout (not just test timeout)
3. Improve shell command repair logic

## Files Modified

- `src/routing_calc.rs` - Reverted terminology
- `src/defaults_router.rs` - Reverted system prompts
- `src/execution_ladder.rs` - Reverted level names
- `src/orchestration_planning.rs` - Confidence fallback
- `src/program_policy.rs` - Step limits, duplicate detection
- `src/app_chat_helpers.rs` - Output truncation
- `src/tune.rs` - Prompt hash tracking
- `src/types_core.rs` - prompt_hashes field
- `src/optimization_tune.rs` - Store prompt hashes
- `run_stress_tests_cli.sh` - New CLI test runner

## Test Count
- Unit tests: 109 passing ✅
- Stress tests: 1/19 passed, 18 timeout ⚠️
