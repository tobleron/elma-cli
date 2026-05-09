# Task 790: Compaction User Message Preservation & Fallback

## Objective
Preserve recent user instructions during context compaction and implement progressive trimming as a fallback (P1 Priority).

## Background
Currently, Elma's `auto_compact.rs` replaces the entire conversation history with a single summary. This discards the user's original request and recent feedback, causing the model to lose track of the core objective. Codex-RS preserves a configurable budget of recent user messages (`COMPACT_USER_MESSAGE_MAX_TOKENS`) during compaction. Additionally, if the compaction prompt itself exceeds the context window, Elma hard-fails after 3 retries, whereas reference architectures gracefully trim the oldest items and retry.

## Requirements
1. **User Message Preservation:** Update the compaction logic to identify and retain the most recent user messages (up to a defined token limit, e.g., 2000 tokens for an 8K window) so the model remembers the current objective. These preserved user messages should be placed after the generated summary in the compacted history.
2. **Progressive Trimming Fallback:** If the `auto_compact` summarization request fails due to context limits (e.g., the history is too large even to summarize), implement a fallback that drops the oldest turn(s) (excluding the system prompt) and retries the summarization.
3. Add tracing events for when user messages are preserved or when progressive trimming is triggered, ensuring visibility in the transcript (per AGENTS.md Rule 6).

## Success Criteria
- [ ] After a compaction event, the most recent user request is still present in the `messages` array.
- [ ] If a conversation is artificially bloated beyond the context window, compaction successfully recovers by progressively trimming the oldest items rather than panicking or failing the turn.
- [ ] Compaction decisions are visible in the transcript.
