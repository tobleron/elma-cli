# Task 786: Centralized Tool Output Capping

## Objective
Implement centralized auto-persistence for large tool outputs to prevent context window flooding on local models (P0 Priority).

## Background
Currently, Elma lacks a centralized mechanism to cap tool outputs. A single large `rg` or `cat` output can consume 50%+ of a 3B model's 8K context window in one turn, leaving no room for reasoning and causing the session to fail or loop. References like Codex-RS and Claude Code solve this by automatically writing large results to temp files and returning a truncated summary + file path to the model.

## Requirements
1. Define a global `MAX_RESULT_SIZE_CHARS` (e.g., 8,000 characters).
2. Create a utility wrapper for tool outputs (e.g., in `tool_result_storage.rs` or similar) that intercepts outputs before they are added to the conversation history.
3. If an output exceeds the threshold:
   - Write the full output to a secure temporary file (`tempfile` crate).
   - Generate a truncated preview (e.g., first 20 lines + last 20 lines).
   - Return the preview + a system message indicating the output was truncated and saved to the temp file path.
4. Ensure the model can still view the temp file if it needs more detail (using `read` or `search` on the temp file).

## Success Criteria
- [ ] A tool call that returns 50,000 characters is automatically capped.
- [ ] The full 50,000 characters are saved to a temp file.
- [ ] The model receives a concise preview and the path to the temp file without exceeding context limits.
- [ ] The change is tested against a large file read.
