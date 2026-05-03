# Task 290: Eliminate Duplicate User Message From Conversation History

## Backlog Reconciliation (2026-05-02)

Resume through Task 469 session-state ownership and Task 470 event logging if duplicate transcript/state entries remain.


**Status:** pending
**Priority:** medium
**Depends on:** None

## Summary

The current user message appears twice in the LLM context: once embedded in the "Recent conversation" history block (within the system prompt), and once as the standalone active user message. This wastes context budget and may confuse small models.

## Scope

- Filter the current turn's user message from the conversation history snippet before embedding it in the system prompt.
- Locate the embedding point (likely `orchestration_core.rs:build_tool_calling_system_prompt` or `app_chat_loop.rs`).
- Ensure no other history-based logic depends on the duplicate.
- Add regression test or assert against duplicate content.

## Non-Goals

- No changes to the tool loop's message construction (system prompt + user message is correct).
- Do not modify `src/prompt_core.rs`.

## Success Criteria

- [ ] The current user message appears exactly once in every LLM call context.
