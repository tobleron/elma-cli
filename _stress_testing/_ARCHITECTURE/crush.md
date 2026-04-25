# Crush Architectural Analysis

## Overview
A Go-based terminal AI coding assistant (by Charm) featuring a `Coordinator` and `SessionAgent` for multi-agent management.

## 11-Point Deep Dive
1. **Context Compaction:** Auto-summarization thresholds; large windows use buffers, small ones use a 20% ratio.
2. **Memory Management:** Session-based message services with persistent storage.
3. **Agent Awareness:** "Skills" discovery and a `CrushInfoTool` for environment awareness.
4. **Agent Loop Handling:** Streaming loop with `StopWhen` conditions for loop detection and token limits.
5. **Guardrails:** Explicit permission service and "YOLO" mode controls.
6. **Memory Efficiency:** Support for Anthropic-style cache control headers.
7. **UI Specific:** Built with Bubble Tea/Lip Gloss for high-quality TUI.
8. **Session Management:** Robust session service with ID tracking and concurrency support.
9. **Error Handling:** Structured error types and automatic OAuth2 token refreshing.
10. **Model Orchestration:** Merged call options (temperature, topP) from model/provider configs.
11. **Context Window Management:** Real-time token tracking in the session.

## SWOT Analysis
- **Strengths:** Deep MCP/LSP integration, high-performance Go backend, superior TUI.
- **Weaknesses:** Complex configuration merging logic.
- **Opportunities:** Multi-agent coordination ready for expansion.
- **Threats:** Heavy reliance on the internal "Fantasy" abstraction layer.
