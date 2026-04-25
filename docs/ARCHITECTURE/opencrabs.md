# OpenCrabs Architectural Analysis

## Overview
A Rust-based autonomous agent framework using a "Brain" and "AgentContext" design. It emphasizes hybrid search memory (FTS + Vector).

## 11-Point Deep Dive
1. **Context Compaction:** Uses `compact_with_summary` in `context.rs`, summarizing the conversation tail to fit token budgets.
2. **Memory Management:** Shared `qmd` Store (`memory.db`) for long-term memory with hybrid search.
3. **Agent Awareness:** Identity/capabilities defined in workspace Markdown files (`SOUL.md`, `IDENTITY.md`) and injected into system prompts.
4. **Agent Loop Handling:** CancellationToken-based loops for asynchronous channel communications (WhatsApp/Slack).
5. **Guardrails:** Injected policy files (`SECURITY.md`) to enforce behavioral constraints.
6. **Memory Efficiency:** Accurate token counting via `tiktoken` instead of heuristics.
7. **UI Specific:** Minimal; focuses on backend channel integration.
8. **Session Management:** UUID-based sessions tied to `AgentContext`.
9. **Error Handling:** Specialized modules for Brain and Agent-level errors.
10. **Model Orchestration:** Managed through provider traits; relies on external API temperature settings.
11. **Context Window Management:** Explicit `token_count` and `max_tokens` tracking in `AgentContext`.

## SWOT Analysis
- **Strengths:** Precise token management, hybrid search, modular prompts.
- **Weaknesses:** Rust-based complexity for rapid UI prototyping.
- **Opportunities:** Deeper channel integrations.
- **Threats:** High dependency on specific LLM provider capabilities.
