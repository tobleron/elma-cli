# agenticSeek Architectural Analysis

## Overview
A Python-based multi-agent framework using a "Router-Planner-Worker" pattern with FastAPI.

## 11-Point Deep Dive
1. **Context Compaction:** Uses a specialized `Memory` class with a dedicated summarization model (`pszemraj/led-base-book-summary`) for context compression.
2. **Memory Management:** Session recovery via JSON memory files in `conversations/`.
3. **Agent Awareness:** Defined by roles in `sources/agents/`. The PlannerAgent delegates tasks based on agent capabilities.
4. **Agent Loop Handling:** Managed via `Interaction.think()` and `PlannerAgent.process()`, including plan-generation retry logic.
5. **Guardrails:** Basic logging, cloud provider warnings, and headless browser enforcement in Docker.
6. **Memory Efficiency:** Conditional summarization based on `ideal_ctx` calculations.
7. **UI Specific:** FastAPI backend for web frontend; CLI uses `readline` and ANSI colors.
8. **Session Management:** UUID-based session IDs persisted to disk.
9. **Error Handling:** `think_wrapper` catches agent process exceptions; includes JSON parsing retries.
10. **Model Orchestration:** Temperature configurable via `config.ini`.
11. **Context Window Management:** Estimates context window size based on model name substrings (e.g., "14b").

## SWOT Analysis
- **Strengths:** Robust task decomposition, dedicated summarization model for memory.
- **Weaknesses:** Brittle heuristic-based context window estimation.
- **Opportunities:** Potential for MCP (Model Context Protocol) integration.
- **Threats:** High dependency on specific external models for memory management.
