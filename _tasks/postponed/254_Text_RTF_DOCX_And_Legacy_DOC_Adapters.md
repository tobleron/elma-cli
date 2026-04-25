# 254 - Text RTF DOCX And Legacy DOC Adapters

Status: Pending
Priority: P1
Depends on: 249

## Goal

Make common text and office-like document formats usable for ebook/document chat without external converters.

## Format Scope

| Format | Strategy |
|---|---|
| `.txt` | Encoding-aware full text |
| `.rtf` | Evaluate `scrivener-rtf`; fallback to conservative internal text extraction |
| `.docx` | `zip` plus `quick-xml` over WordprocessingML |
| `.doc` | `cfb` plus limited legacy Word extraction, or explicit degraded/unsupported |

## TXT Requirements

- Use `encoding_rs` when UTF-8 fails.
- Record encoding repairs and replacement counts.
- Preserve paragraph boundaries.

## RTF Requirements

- Parse text, unicode escapes, paragraph breaks, and basic headings/styles when possible.
- Drop control words that are not user-visible text.
- Preserve metadata when available.
- If parser quality is not adequate, implement a limited plain-text RTF extractor instead of adding a weak dependency blindly.

## DOCX Requirements

- Open the ZIP package safely.
- Parse `word/document.xml`.
- Extract paragraphs, headings, tables, footnotes/endnotes when practical.
- Pull core metadata from `docProps/core.xml`.
- Preserve heading path and paragraph order.
- Protect against zip bombs and path traversal.

## Legacy DOC Requirements

Legacy `.doc` is a binary Word format inside Compound File Binary. Full fidelity is hard. This task should:

- Use `cfb` to recognize valid CFB containers.
- Detect Word streams such as `WordDocument`, `0Table`, and `1Table`.
- Implement limited text extraction only if it can be fixture-backed.
- Otherwise mark `.doc` as degraded/unsupported with a clear explanation.
- Never claim `.doc` support based only on opening the CFB container.

## Acceptance Criteria

- TXT, RTF, and DOCX produce usable text chunks.
- DOCX citations can include heading/table provenance.
- Legacy DOC is either fixture-backed full/degraded extraction or explicit unsupported.
- Binary DOC files are not read as random text.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test office_document_adapter`
- Fixture tests for UTF-8 TXT, non-UTF-8 TXT, RTF unicode, DOCX paragraphs/tables, and DOC unsupported/degraded.

