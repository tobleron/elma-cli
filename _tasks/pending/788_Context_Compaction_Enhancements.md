# Task 788: Context Compaction Enhancements (Context & Integrity)

## Objective
Enhance the `auto_compact` system to re-inject initial system context and ensure tool-call integrity after compaction (P0 Priority).

## Background
When Elma compacts conversation history, it currently replaces the history with a single summary. This causes two major issues:
1. **Context Loss:** The model loses its initial system prompt, tool definitions, and workspace brief, severely degrading subsequent performance.
2. **Integrity Loss:** The summary may reference tool calls without results, or result blocks may be orphaned, causing the model to hallucinate or loop. Reference architectures (Codex-RS, Dirac) explicitly re-inject initial context and validate tool-use/tool-result pairs post-compaction.

## Requirements
1. **Context Re-Injection:** Update `auto_compact.rs` to ensure that after generating the summary, the canonical initial context (system prompt, workspace brief) is re-injected at the top of the compacted history.
2. **Integrity Validation:** Implement a validation step post-compaction that ensures every `tool_call` in the compacted history has a corresponding `tool_result`. If a result is missing, synthesize an empty or "result missing" block to maintain the strict user-assistant-user structural requirements of the API.
3. Ensure these changes comply with AGENTS.md Rule 4a (Complexity gating) and Rule 7 (Decompose instead of blaming the model).

## Success Criteria
- [ ] After compaction, the `messages` array retains the system prompt and tool definitions as the first elements.
- [ ] A test verifies that orphaned tool calls or results in a mocked compacted history are correctly paired or pruned.
- [ ] A long-running session that triggers compaction can successfully execute a tool in the immediately following turn without hallucinating.
