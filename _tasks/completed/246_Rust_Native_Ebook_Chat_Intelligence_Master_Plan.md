# 246 - Rust-Native Ebook Chat Intelligence Master Plan

Status: Pending
Priority: P1 after current P0 security and terminal-safety work, unless explicitly promoted
Depends on: Task 197, Task 203, dependency tasks 223, 224, 226, 233, 234

## Objective

Build a Rust-native document and ebook intelligence track that lets Elma understand, index, cite, and chat with ebooks and long documents without silently truncating the source. The target formats are:

`.epub`, `.pdf`, `.mobi`, `.azw`, `.azw3`, `.kfx`, `.fb2`, `.djvu`, `.txt`, `.html`, `.xhtml`, `.rtf`, `.doc`, `.docx`, `.cbr`, `.cbz`, `.iba`, `.lit`, `.pdb`, `.lrf`, `.lrx`, `.chm`.

Support must mean one of three honest states:

| State | Meaning |
|---|---|
| Full text | Elma extracts searchable text, structure, metadata, provenance, and chunks suitable for chat. |
| Metadata/degraded | Elma can identify the file and may extract metadata or embedded text, but must disclose missing text. |
| Explicit unsupported | Elma recognizes the format and explains why extraction is not available, rather than pretending to read it. |

## Research Synthesis From `_stress_testing/_chat_text_skill`

FlockParser showed that practical ebook chat needs a pipeline, not a single reader call: multi-backend PDF extraction, page markers, text cleanup, token-aware chunking, embedding cache, adaptive top-k, context grouping by source, and multi-pass context budgeting.

LocalRAG showed a clean separation between loaders, validators, chunkers, vector stores, retrievers, caches, rerankers, and MMR. Its useful lesson for Elma is not the Python dependencies; it is the architecture boundary and fallback order.

chatdocs showed broad loader registry behavior. Elma should copy the registry idea, not LangChain or Python `unstructured`.

cliven and doccura showed the minimum viable local RAG loop: validate the file, extract page-aware text, attach source metadata, chunk with overlap, retrieve, then answer with source-only context and explicit insufficiency when the context does not support the answer.

pdfrag is the closest architectural match. It converts PDFs into structured markdown-like text, preserves page and section provenance, stores file signatures, avoids re-indexing unchanged files, adapts chunk size when embeddings reject long input, and enforces citation-oriented answers.

## Current Elma State

`src/document_adapter.rs` already defines a normalized extraction result and supports extension-based extraction for plaintext/code, HTML, PDF, EPUB, DjVu, and MOBI. It also advertises AZW3 as unsupported.

The current implementation is incomplete for real ebook chat:

| Area | Current behavior | Required change |
|---|---|---|
| Live read path | `src/tool_calling.rs` and `src/execution_steps_read.rs` still use `std::fs::read_to_string` | Route supported binary formats through `document_adapter::extract_document` |
| Format detection | Extension-only | Add signature and container sniffing |
| PDF | `pdf_extract::extract_text` all-at-once | Use page-aware extraction and quality flags |
| EPUB | Concatenates spine resources, then `html2text` | Preserve OPF metadata, spine order, TOC, chapter labels |
| MOBI | Uses `mobi::Mobi::content_as_string` | Preserve metadata and classify AZW-like variants |
| Chunking | Simple byte/line size splitting | Token and structure-aware chunking with overlap |
| Retrieval | Lowercase substring search only | Hybrid lexical/semantic retrieval with source diversity |
| Context | Preview first chunks | Budget-aware full-document or staged processing |
| Long-tail formats | Mostly unknown | Add capability matrix and staged adapters |

## Rust-Native Backend Direction

Already present or compatible with current dependency direction:

| Need | Preferred Rust approach |
|---|---|
| PDF text | `pdf-extract::extract_text_by_pages`; evaluate `lopdf` for metadata/page fallback |
| EPUB | Prefer MIT `epub-parser` or an internal `zip` plus `quick-xml` parser over GPL `epub` if license policy requires |
| Kindle | Evaluate `boko` for EPUB/MOBI/AZW3/KFX, but isolate because it currently requires nightly |
| MOBI/AZW | Existing `mobi` crate for legacy MOBI and AZW-like PalmDB content |
| DjVu | Existing `djvu-rs`, text-layer only |
| FB2 | `fb2` crate or `quick-xml` direct FictionBook traversal |
| DOCX | Existing `zip` plus `quick-xml` over `word/document.xml` |
| RTF | Evaluate `scrivener-rtf`; fallback to internal plain-text extractor if crate quality is insufficient |
| Legacy DOC | `cfb` for container plus limited WordDocument text extraction; do not overclaim full fidelity |
| CBZ | Existing `zip`; extract `ComicInfo.xml`, embedded text files, and image inventory |
| CBR | Feature-gated RAR backend only if license/build posture is acceptable; otherwise explicit unsupported |
| CHM/LIT | Evaluate `chmlib`/CHM options; no baseline C dependency unless approved |
| IBA | ZIP package inspection; extract embedded HTML/XHTML/EPUB-like content when present |
| PDB | Treat as Palm Database ebook only after signature sniffing; do not confuse with Microsoft debug PDB |
| LRF/LRX | Recognize and report; LRX is DRM-oriented and must not claim readable text by default |

## Implementation Sequence

1. 247 wires the current adapter into the live read path and fixes capability truth.
2. 248 adds the format registry, signature sniffing, and full capability matrix.
3. 249 upgrades the document model so all adapters share metadata, units, chunks, quality flags, and provenance.
4. 250 upgrades PDF extraction.
5. 251 upgrades EPUB, HTML, XHTML, and package-style XHTML extraction.
6. 252 upgrades MOBI/AZW/AZW3/KFX handling.
7. 253 adds FB2 and DjVu structure-aware handling.
8. 254 adds TXT, RTF, DOCX, and legacy DOC handling.
9. 255 adds comic/package formats: CBZ, CBR, and IBA.
10. 256 adds legacy/exotic support policy for CHM, LIT, PDB, LRF, and LRX.
11. 257 replaces byte chunking with token and structure-aware chunking.
12. 258 adds budget-aware whole-document work planning.
13. 259 adds persistent document cache and change detection.
14. 260 adds hybrid retrieval and source diversity.
15. 261 adds citation-grounded ebook chat answering.
16. 262 adds transcript-native telemetry and user controls.
17. 263 adds fixtures, stress gates, and real CLI validation.

## Non-Goals

- Do not add Python runtime dependencies.
- Do not auto-install Calibre, Tesseract, Poppler, or system tools.
- Do not claim DRM-protected or image-only files are readable unless Elma has actually extracted text.
- Do not route document behavior with hardcoded user-word triggers.
- Do not reduce full-document processing coverage for performance without explicit approval.

## Master Acceptance Criteria

- Every listed format has a capability entry and a fixture-backed expected behavior.
- Full-text formats produce normalized `DocumentUnit` and `DocumentChunk` records with source provenance.
- Unsupported or degraded formats fail cleanly with actionable, transcript-visible explanations.
- Large ebooks are processed through a budget-aware plan instead of naive truncation.
- Ebook chat answers cite page/chapter/chunk provenance or state that the provided document context is insufficient.

