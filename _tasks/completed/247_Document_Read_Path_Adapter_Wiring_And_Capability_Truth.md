# 247 - Document Read Path Adapter Wiring And Capability Truth

Status: completed
Priority: P1, first task in the ebook track
Depends on: Task 197, Task 203

## Problem

Elma currently has `src/document_adapter.rs`, but the live read execution paths still use plain UTF-8 reads. This means binary document support is partly advertised but not reliably deployed.

Known evidence:

| File | Current issue |
|---|---|
| `src/tool_calling.rs` | `exec_read()` reads with `std::fs::read_to_string` |
| `src/execution_steps_read.rs` | `handle_read_step()` reads with `std::fs::read_to_string` |
| `src/orchestration_core.rs` | The system prompt advertises automatic document extraction |
| `src/document_adapter.rs` | Extraction exists but is not authoritative in live read paths |

## Goal

Make `document_adapter::extract_document` the authoritative read path for supported document and ebook formats while preserving normal source-code and plaintext reads.

## Scope

- Introduce a shared read service that decides whether a path should be read as plain text or extracted as a document.
- Wire the shared service into `exec_read()` and `handle_read_step()`.
- Ensure the tool output reports backend, format, chunk count, estimated size, and extraction warnings.
- Make prompt/tool descriptions match actual deployed behavior.
- Keep noninteractive/script output explicit and separate from TUI-owned output.

## Implementation Requirements

- Do not call external conversion tools.
- Do not read binary files through `read_to_string`.
- Do not dump an entire extracted ebook into the context window by default.
- Return a concise extraction summary plus selected content according to the caller budget.
- Preserve existing behavior for code, config, markdown, and plaintext files.
- Surface extraction failures as tool results, not panics.

## Acceptance Criteria

- Reading a PDF through the real CLI no longer fails due to invalid UTF-8.
- Reading an EPUB through the real CLI uses the document adapter.
- Reading a Rust source file still returns source text directly.
- Unsupported binary formats return a clear capability error with no lossy garbage output.
- Prompt text no longer advertises formats that the live read path cannot handle.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_adapter`
- Targeted tests for `exec_read()` and `handle_read_step()` with plaintext, PDF, EPUB, and unsupported binary fixtures.
- Real CLI validation: ask Elma to read a small PDF and a small EPUB, then confirm the transcript shows extraction metadata and useful text.

