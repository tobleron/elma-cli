# Troubleshoot T001: Task 045 Terminology Broke Router Classification

## Problem
After Task 045 terminology overhaul, ALL requests are classified as CHAT/CONVERSATION with entropy=0.00, preventing proper workflow execution.

## Evidence
```
Test: S000B_Shell_Primitive (should run shell commands)
Output: speech=CONVERSATION route=CHAT (entropy=0.00)
Result: Just replies with text, doesn't execute commands
```

## Root Cause
Task 045 changed terminology:
- `CHAT` → `CONVERSATION` / `DIRECT_ANSWER`
- `SHELL` → `TERMINAL_ACTION`
- `WORKFLOW` → `ORCHESTRATED_TASK`
- `INSPECT` → `DISCOVER`
- etc.

**But the router model was tuned on OLD terminology.** The model's output layer still produces old terms, which don't match new code mappings.

## Solution Options

### Option A: Revert Task 045 (Recommended)
Revert terminology to original terms that the tuned model understands.

**Pros:**
- Immediate fix
- No re-tuning needed
- Preserves working classification

**Cons:**
- Loses "improved articulation" benefits

### Option B: Re-tune with New Terminology
Update all system prompts with new terms and re-tune model.

**Pros:**
- Keeps Task 045 improvements
- Proper long-term solution

**Cons:**
- Requires full re-tuning
- May take hours
- No guarantee of better performance

### Option C: Hybrid (Keep Code Terms, Map Router Terms)
Keep new terms in code but router still uses old terms.

**Pros:**
- Compromise solution
- No re-tuning needed

**Cons:**
- Inconsistent terminology
- Confusing for future development

## Recommended Action
**Option A: Revert Task 045**

The terminology changes don't provide enough benefit to justify breaking the entire classification system. The old terms (CHAT, SHELL, PLAN, etc.) work fine and are well-understood.

## Files to Revert
- src/routing_calc.rs
- src/defaults_router.rs
- src/execution_ladder.rs
- src/orchestration_planning.rs
- src/program_policy.rs
