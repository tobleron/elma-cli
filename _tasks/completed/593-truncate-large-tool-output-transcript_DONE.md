# Task 593: Truncate Large Tool Output in Terminal Transcript

## Session Evidence
Session `s_1777823506_810966000`: Shell `cat` commands produced outputs up to 48,681 characters (ARCHITECTURE.md, 1030 lines). The terminal transcript captured the full raw content, including markdown formatting, code blocks, and inline diagrams. The user saw walls of raw documentation text in their terminal.

Example: `cat docs/ARCHITECTURE.md` → "…+47657 characters truncated" — but the transcript still shows hundreds of lines before the truncation point.

## Problem
The terminal transcript (`terminal_transcript.txt`) captures FULL tool output for `shell` commands, flooding the user's view with raw file contents. The `TRANSCRIPT_OUTPUT_LIMIT` exists (claude_render.rs) but it applies late — much of the content already streams to the transcript. The model needs full output for context, but the user only needs a preview.

## Solution
1. For `shell` tool output in the terminal transcript, apply a hard character limit per tool result: **1,000 chars** for `cat`/`head`/`tail` read-like commands, **10,000 chars** for other commands
2. When truncated, append: `(output truncated — full content available to model and in session evidence)`
3. The model's context still receives the full output (no change to `TOOL_CALLING_SYSTEM_PROMPT` or evidence pipeline)
4. Add a `TranscriptPreview` trait on `ToolExecutionResult` that generates a transcript-safe version of the content

Implementation location: `src/tool_calling.rs` in `emit_tool_result()`, or in the transcript rendering pipeline in `claude_render.rs`.
