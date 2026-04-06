# Task 111: Integrate Syntect Engine for Surgical Syntax Highlighting

## Objective
Integrate the `syntect` library to provide high-fidelity syntax highlighting for all code blocks and configuration files displayed in the terminal.

## Technical Implementation Plan

### 1. Syntect Environment Setup
- Implement a `SyntaxHighlighter` struct in `src/text_utils.rs`.
- Load the default `SyntaxSet` (newlines/binary) and `ThemeSet` during the bootstrap phase.
- Use the `base16-ocean.dark` or `Monokai` theme as the default.

### 2. ANSI Conversion Logic
- Implement a converter that takes `syntect::highlighting::Style` and maps it to ANSI escape codes.
- Handle "TrueColor" (24-bit) vs "256-color" fallbacks based on terminal capability detection.

### 3. Markdown Integration
- Update the message rendering logic in `src/ui_chat.rs` to:
    - Identify code blocks using regex or a markdown parser.
    - Extract the language (e.g., `rust`, `json`).
    - Pass the block through `syntect`.
    - Print the highlighted result to the terminal.

### 4. Performance & Caching
- Implement a simple `LruCache` for highlighted snippets to avoid re-computing colors for recurring UI elements or session history.

## Verification Strategy
1. **Correctness**: Verify that Rust, Python, and JSON code blocks are highlighted correctly according to their respective grammars.
2. **Fallback**: Confirm that code with an unknown language tag (e.g., ```foobar) is rendered as plain text without crashing.
3. **Performance**: Measure the rendering time for a 500-line code file; it should be sub-10ms.
