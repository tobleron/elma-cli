# Task 392: Plaintext Default And Markdown Output Tool

**Status:** Pending
**Priority:** HIGH
**Estimated effort:** 2-3 days
**Dependencies:** Task 384
**References:** user objective for plain ratatui output and optional markdown files

## Objective

Make terminal final answers plain text by default while preserving a markdown-generation tool for explicit user requests such as reports, documentation files, and exported summaries.

## Problem

Ratatui output should not depend on markdown rendering for normal assistant replies. Markdown remains useful as an artifact format, but terminal answers should be clean, readable plain text.

## Implementation Plan

1. Add a final-answer display mode distinction:
   - `terminal_plaintext`
   - `markdown_artifact`
2. Update finalization so normal chat responses are converted to high-quality plain text before rendering.
3. Keep markdown rendering available for existing transcript/history paths if needed, but do not require markdown in the final answer path.
4. Add a `markdown_report` or equivalent artifact tool/path that writes `.md` only when the user asks for markdown output.
5. Ensure markdown artifacts never replace the plain terminal answer unless the user explicitly asks to view markdown text.
6. Add tests for markdown stripping, code block preservation as plain text, list formatting, and markdown artifact creation.

## Non-Scope

- Do not remove existing markdown parsing modules.
- Do not change `src/prompt_core.rs`.

## Verification

```bash
cargo test final_answer
cargo test markdown
cargo test ui
cargo build
```

Manual probes:

- Ask a normal question and verify the terminal response is plain text.
- Ask for a markdown report file and verify a `.md` artifact is created.

## Done Criteria

- Plain text is the default terminal final-answer format.
- Markdown output exists as an explicit artifact path.
- Final answers do not leak internal analysis or markdown-only formatting artifacts.

