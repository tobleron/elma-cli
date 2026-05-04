# Task 621: Agent response quality — terse output lacks acknowledgment

## Type

Bug (Finalization / Model Robustness)

## Severity

Medium

## Scope

System-wide (evidence finalizer + system prompt)

## Session Evidence

**Session:** `s_1777889677_334314000`

Turn 1 ("what is the oldest file in project_tmp?"): answer_len=12 — answer was just `TEST_PLAN.md` (filename, no sentence)
Turn 2 ("delete that file"): answer_len=43 — brief, no confirmation tone
Turn 3 ("delete GEMINI_test.md"): answer_len=77 — improvement but still impersonal
Turn 4 ("delete findings.txt"): answer_len=85

The agent responses are bare facts — no natural acknowledgment like "I've found that TEST_PLAN.md is the oldest file in project_tmp" or "I've moved findings.txt to the trash — done."

## Problem

The evidence finalizer produces ultra-concise, factual output. Combined with the `Mode: Concise` prompt instruction, answers become robotic one-liners that don't feel like an agent conversation. The user doesn't know what the agent did unless they read tool output themselves.

This violates Elma's design philosophy — the agent should "feel premium, careful, and capable." A 4B model can produce natural acknowledgments; the prompt is constraining it too much.

## Root Cause Hypothesis

**Confirmed:** The `Mode: Concise` instruction in `prompt_core.rs` says "Respond concisely in natural prose, less than 300 words." But the system prompt's evidence finalizer path forces extreme conciseness. Combined, the model outputs only the bare fact (filename, or "Moved to trash") without contextual framing.

Additionally, the evidence finalizer prompt (`request_final_answer_from_evidence`) feeds BOTH the original user request and all evidence blocks. The model chooses the shortest possible answer from the evidence.

## Proposed Solution

### Part A: Update evidence finalizer prompt
In `tool_loop.rs::request_final_answer_from_evidence`, change the prompt to:
- Require a complete sentence response (not just a noun)
- Include an acknowledgment of the action taken
- Use a natural conversational tone

Change from:
"Answer concisely using only the evidence above. Use plain text only..."
To:
"Answer in a natural conversational tone. Acknowledge what you did. Use the evidence to form a complete sentence. Be concise but human."

### Part B: Add response framing in system prompt
In `prompt_core.rs` Concise mode, add: "Always respond with complete sentences. Acknowledge what action was taken or what was found."

Files to change:
- `src/tool_loop.rs` — `request_final_answer_from_evidence` prompt
- `src/prompt_core.rs` — Concise mode instruction

## Acceptance Criteria

- [ ] Agent responses are complete sentences, not bare nouns
- [ ] Agent acknowledges actions taken (not just raw fact)
- [ ] Users can understand what happened without reading tool output
- [ ] Responses stay under 300 words for Concise mode

## Verification Plan

- Replay session: verify turn "delete findings.txt" produces "I've moved findings.txt to the trash" not just "Moved to trash"
- Manual test: ask "what is the oldest file in project_tmp?" and verify answer is a complete sentence

## Dependencies

None.
