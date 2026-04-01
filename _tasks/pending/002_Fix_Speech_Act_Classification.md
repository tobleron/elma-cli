# Task 046: Fix Speech Act Classification (Principle-Based Prompts)

## Priority
**P0 - CRITICAL** (Highest accuracy gain, prompt-only change)

## Problem
Speech act classification misclassifies information requests as CHAT:
- "what is the current time" → CHAT (wrong, should be INFO)
- "list pending tasks" → CHAT (wrong, should be INSTRUCTION)

**Root cause:** Prompts don't distinguish between:
- **CHAT:** Casual conversation with no specific request
- **INFO:** Specific information request
- **INSTRUCTION:** Request for action

## Objective
Update speech_act prompts to use **principles only** (no hardcoded examples) that teach the model to distinguish intent types.

## Implementation

### Files to Modify
1. `src/defaults_router.rs` - `default_speech_act_config()`

### Prompt Changes (Principle-Based)

**BEFORE (vague):**
```toml
system_prompt = """
Mapping:
1 = INSTRUCTION
2 = INFO  
3 = CHAT

Classification rules:
- INSTRUCTION: User wants Elma to DO something
- INFO: User seeks information
- CHAT: User is casually chatting
"""
```

**AFTER (principle-based):**
```toml
system_prompt = """
You are Elma's intent classifier.

Return ONE digit (1/2/3) based on what the user wants from YOU:

1 = INSTRUCTION - User wants YOU to DO something that changes state
  Principle: If completing the request would change something (file, system, workspace), it's INSTRUCTION

2 = INFO - User wants YOU to PROVIDE information without changing state
  Principle: If the user asks "what/when/where/how/why" and expects an answer from YOUR knowledge or workspace inspection, it's INFO

3 = CHAT - User wants to converse without specific request
  Principle: If there's no specific request for action or information, just conversation, it's CHAT

Key distinction:
- INSTRUCTION changes state
- INFO provides answer
- CHAT continues conversation

Return ONLY the digit. No explanation.
"""
```

## Acceptance Criteria
- [ ] "what is the current time" → INFO (not CHAT)
- [ ] "list pending tasks" → INSTRUCTION (not CHAT)
- [ ] "hi" → CHAT (correct)
- [ ] "can you help me" → CHAT (no specific request)
- [ ] "can you list files" → INSTRUCTION (polite action request)
- [ ] No hardcoded examples in prompt
- [ ] Principles only, model applies to novel cases

## Expected Impact
- **+25% routing accuracy** (correct speech act → correct workflow)
- **-30% CHAT misclassification** (fewer action requests treated as chat)
- **Zero code changes** (prompt only)

## Dependencies
- None

## Verification
- `cargo build`
- Test scenarios:
  - "what is X" → INFO
  - "list/show/find X" → INSTRUCTION
  - "hi/hello" → CHAT
  - "can you..." → Depends on whether action follows

## Architecture Alignment
- ✅ Principle-based prompts (AGENTS.md/QWEN.md compliance)
- ✅ Articulate terminology (INSTRUCTION/INFO/CHAT clearly defined)
- ✅ Enables autonomous reasoning (model applies principles)
