# ⏸️ POSTPONED

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 011: Angel Helper Output - Transient Only (NOT in Chat Context)

## Priority
**P0 - CRITICAL** (Prevents context pollution across iterations)

## Problem Statement

Angel Helper's output is currently at risk of being stored in `runtime.messages` or session chat context. This pollutes the conversation history and confuses the orchestrator in later iterations.

**Analogy:**
- ✅ CORRECT: Angel whispers to Elma's mind (transient, like a thought)
- ❌ WRONG: Angel speaks aloud in the conversation (stored in history)

## Architecture

### What Should Be Stored in Chat Context
```
- User messages
- Elma's responses
- Execution outputs (shell results, file contents)
```

### What Should NOT Be Stored in Chat Context
```
- Angel Helper guidance (transient inspiration)
- Rephrased Intention (internal processing)
- Classification results (internal routing)
- Reflection scores (internal validation)
```

## Implementation Requirements

### 1. Audit `runtime.messages` Population
**File:** `src/app_chat_core.rs`

**Check:**
- Angel output is NEVER added to `runtime.messages`
- Only user input and Elma output are pushed

**Code Pattern:**
```rust
// ✅ CORRECT
runtime.messages.push(ChatMessage {
    role: "user".to_string(),
    content: line.to_string(),
});

// Angel runs (transient, NOT stored)
let angel_guidance = angel_helper_intention(...);

// Elma responds
let response = execute_and_respond(...);

// ✅ CORRECT
runtime.messages.push(ChatMessage {
    role: "assistant".to_string(),
    content: response,  // Only Elma's output
});
```

### 2. Audit Session Context Saving
**Files:** `src/session*.rs`, `src/app_chat_core.rs`

**Check:**
- Session trace can log Angel (for debugging)
- Session chat context CANNOT include Angel

### 3. Audit Orchestrator Input
**File:** `src/orchestration*.rs`

**Check:**
- Orchestrator receives `runtime.messages` (clean conversation)
- Angel guidance is passed separately (not via messages)

## Acceptance Criteria
- [ ] Angel output is NEVER in `runtime.messages`
- [ ] Chat context contains only: User + Elma turns
- [ ] Session trace can log Angel (separate from chat context)
- [ ] Test: 5+ iteration conversation remains clean (no Angel pollution)
- [ ] Test: Orchestrator is not confused by accumulated internal processing

## Expected Impact
- **-90% context pollution** (Angel whispers stay transient)
- **+50% orchestrator accuracy** (clean conversation history)
- **+30% multi-turn reliability** (no accumulated confusion)

## Related Tasks
- Task 010: Elma Helper Intention Clarification (Angel Helper implementation)
- Task XXX: Session context cleanup (if exists)

## Notes
User's analogy: "If 2 people are talking and an angel inspires person A, person B doesn't hear the angel's words - they only hear person A's output."
