# Task 102: Surgical Code Syntax Highlighting

## Objective
Implement syntax highlighting for code blocks within messages to improve readability of technical responses.

## Technical Implementation Plan (Rust)

### Core Requirements
1. **Language Detection**:
    - Parse markdown code blocks (e.g., ```rust) to extract the language name.
    - Implement a `detect_language(code_block: &str)` function in `src/text_utils.rs`.
2. **Highlighter Library**:
    - Use the `syntect` crate to perform the syntax highlighting.
    - Load a default theme (e.g., `base16-ocean.dark`) during bootstrap.
3. **Rendering Component**:
    - Implement a `render_highlighted_code(code: &str, lang: &str)` in `src/ui.rs`.
    - Convert `syntect`'s style output to ANSI escape codes for the terminal.
4. **Performance Optimization**:
    - Cache highlighting results for large code blocks to prevent re-parsing during session review or history scrolling.
5. **Integration**:
    - Update the markdown renderer in `src/ui_chat.rs` to call `render_highlighted_code` when a code block is encountered.

### Proposed Rust Dependencies
- `syntect = "5.2"`: The industry-standard library for syntax highlighting in Rust.

### Verification Strategy
1. **Visuals**:
    - Confirm code blocks for common languages (Rust, Python, JS, Bash) are correctly colorized.
    - Confirm multi-line strings and comments are handled.
2. **Robustness**:
    - Verify it handles unknown languages gracefully (fallback to plain text).
    - Ensure large code blocks do not cause UI stutter.
