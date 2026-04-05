# Task 023: Expert Responder Output - Transient Only (NOT in Chat Context)

## Priority
**P0 - CRITICAL** (Prevents context pollution across iterations)
**Master Plan:** Tracked under Task 095, active sub-task (multi-turn verification remaining)

## Problem Statement

Expert responder advice must stay transient. It should shape Elma's final presentation without being stored in `runtime.messages` or polluting session chat context. The same transient rule also applies to `intent_helper` output.

**Analogy:**
- ✅ CORRECT: The advisor whispers to Elma's mind (transient, like a thought)
- ❌ WRONG: The advisor speaks aloud in the conversation (stored in history)

## Architecture

### What Should Be Stored in Chat Context
```
- User messages
- Elma's responses
- Execution outputs (shell results, file contents)
```

### What Should NOT Be Stored in Chat Context
```
- Expert responder guidance (transient inspiration)
- Rephrased Intention (internal processing)
- Classification results (internal routing)
- Reflection scores (internal validation)
```

## Implementation Requirements

### 1. Audit `runtime.messages` Population
**File:** `src/app_chat_core.rs`

**Check:**
- Expert responder output is NEVER added to `runtime.messages`
- Only user input and Elma output are pushed

**Code Pattern:**
```rust
// ✅ CORRECT
runtime.messages.push(ChatMessage {
    role: "user".to_string(),
    content: line.to_string(),
});

// Internal helpers run (transient, NOT stored)
let intent = annotate_user_intent(...);
let response_advice = request_response_advice_via_unit(...);

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
- Session trace can log internal guidance (for debugging)
- Session chat context CANNOT include internal guidance

### 3. Audit Orchestrator Input
**File:** `src/orchestration*.rs`

**Check:**
- Orchestrator receives `runtime.messages` (clean conversation)
- Response advice is passed separately (not via messages)

## Acceptance Criteria
- [x] Expert responder output is NEVER in `runtime.messages`
- [x] Chat context contains only: User + Elma turns
- [x] Response advice is passed to the presenter via transient narrative extras
- [x] Runtime loads and syncs `expert_responder.toml` as a managed canonical prompt
- [x] `angel_helper` profile identity is normalized to `expert_responder`
- [x] Test: `cargo build`
- [x] Test: `cargo test`
- [x] Test: real CLI `hello`
- [ ] Test: 5+ iteration conversation remains clean
- [ ] Test: Orchestrator is not confused by accumulated internal processing

## Expected Impact
- **-90% context pollution** (internal guidance stays transient)
- **+50% orchestrator accuracy** (clean conversation history)
- **+30% multi-turn reliability** (no accumulated confusion)

## Related Tasks
- Task 010: Elma Helper Intention Clarification
- Task XXX: Session context cleanup (if exists)

## Notes
User's analogy still applies: if 2 people are talking and an advisor inspires person A, person B does not hear the advisor's words, only person A's output.
