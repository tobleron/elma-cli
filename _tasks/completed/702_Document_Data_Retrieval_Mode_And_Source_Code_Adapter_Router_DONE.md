# Task 702: Document/Data Retrieval Mode And Source Code Adapter Router

## Type

Offline Intelligence / Retrieval Architecture

## Severity

High

## User Requirement

Elma must distinguish normal readable documents and data files from source code files. Documents such as PDF, HTML, TXT, EPUB, DOCX, CSV, and similar formats should be converted or indexed as text when needed, with page/section/row citations. Source code should be inspected differently: first use imports, symbols, signatures, and relevant sections instead of loading whole files blindly.

The mode should be inferred from file type, user intent, and chat flow. It should not require a manual "data analysis mode" switch and should not be implemented as brittle keyword matching.

## Problem

Elma is optimized for code search/edit workflows. It needs an additional local-first retrieval path for "chat with PDF", "chat with data", book summarization, citation-backed retrieval, and exploratory document/data analysis.

## Proposed Solution

Create an adapter router that chooses between source-code inspection and readable-document/data retrieval.

Likely source areas:

- `src/document_adapter.rs`
- `src/data_analysis.rs`
- `src/code_index.rs`
- `src/repo_map.rs`
- `src/search_ranker.rs`
- `src/intel_units/intel_units_document_summarizer.rs`
- `src/tool_calling.rs`
- `src/tools/validation.rs`

Requirements:

- Add a typed file-kind classifier: source code, config, readable document, structured data, binary/unsupported.
- For source code, expose import/symbol/signature-first reading before full file reads.
- For documents, convert or extract text into chunks with stable citations: page number for PDF, heading/section for HTML/Markdown/EPUB, line range for TXT, row/column for CSV/TSV.
- For data files, support schema preview, sampled rows, column summaries, and retrieval by semantic question.
- Route based on file kind plus user intent confidence, not hardcoded word triggers.
- Persist extracted text/chunks under session evidence with source metadata.
- Keep all behavior offline by default; network is not required.

## Acceptance Criteria

- [ ] Asking about a PDF/HTML/TXT/EPUB/DOCX file routes to document retrieval and returns citations.
- [ ] Asking about CSV/TSV/JSON data routes to data preview/retrieval and cites row/field evidence.
- [ ] Asking about source code routes to symbol/import/signature-first inspection.
- [ ] The router decision is visible as a transcript row and trace event.
- [ ] Unsupported formats fail with a clear local capability message.

## Verification Plan

Add fixtures for:

- small PDF or generated PDF-equivalent text fixture with page markers
- HTML with headings
- TXT with line citations
- CSV with rows and columns
- Rust source file with imports and functions

Run prompt tests that ask retrieval questions against each fixture and assert citation format.

