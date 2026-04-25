# 258 - Context Budget Document Work Planner

Status: completed
Priority: P1

## Goal

Implement budget-aware whole-document planning so Elma can chat with large ebooks without silently truncating or pretending it read everything.

## Required Behavior

Document and ebook skills must default to full-document processing unless the user explicitly asks for a skim, scoped chapter/topic, or keyword search.

The planner must choose one of these strategies:

| Strategy | Use when |
|---|---|
| Direct full context | The extracted text fits safely in the available context |
| Staged synthesis | The document is too large but the user asks for broad understanding |
| Retrieval-first | The user asks a specific question |
| Chapter/page scoped | The user names a section, chapter, page, or range |
| Metadata/degraded report | Text extraction is incomplete or unavailable |

## Requirements

- Estimate token counts per document, unit, and chunk.
- Reserve context for system prompt, tool results, chat history, and final answer.
- Create a work plan before loading content into model context.
- Preserve semantic continuity from user request to document strategy.
- For broad summaries, process all units through staged summaries before final answer.
- For Q&A, retrieve evidence but keep an option to broaden if evidence is weak.
- Surface the chosen strategy in transcript-native telemetry.

## Acceptance Criteria

- Large EPUB/PDF files produce a plan instead of first-N-chunk truncation.
- Full-document summary uses every page/chapter through staged synthesis.
- Specific questions retrieve focused chunks and cite them.
- If a model/context window is too small, Elma says what coverage it used.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_work_planner`
- Stress fixture with a large synthetic book.
- Real CLI validation with a large PDF/EPUB and a small context profile.

