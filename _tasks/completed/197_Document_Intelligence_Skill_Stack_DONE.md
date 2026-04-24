# Task 197: Document Intelligence Skill Stack

## Priority
P1

## Objective
Add a document-reading skill stack that can summarize and search local documents through a normalized extraction pipeline rather than one-off format handling.

## Why This Exists
Document work is a perfect fit for a specialized skill. The model should not guess how to process each file type on every turn. It should use stable, format-aware adapters with offline-first extraction and a consistent normalized text pipeline.

## Supported Core Formats In This Task
- `txt`
- `md`
- `html`
- `pdf`
- `epub`

Extended formats such as `djvu`, `mobi`, and `azw3` belong to Task 203.

## Required Types
- `DocumentAdapter`
- `DocumentBackend`
- `DocumentChunk`
- `DocumentExtractionResult`
- `DocumentCapabilityReport`

## Required Adapter Contract
Each adapter must define:
- format sniffing / file matching
- extraction method
- metadata capture
- chunking strategy
- source citation labeling
- failure behavior
- backend identity for trace/UI exposure

## Normalized Pipeline
All supported formats must normalize into a common representation:
- document id / source path
- chunk index
- chunk text
- section/page label when available
- byte/page/chapter provenance when available

## User Jobs Supported
- summarize one document
- summarize several documents
- find a needle in a haystack with grounded citations
- provide backend/capability disclosure when extraction is partial or unavailable

## Required Planning Behavior
Before summarizing or extracting from one or more documents, the skill must build a document work plan that includes:
- file list and format list
- extracted or estimated word/character counts
- rough token estimate for raw content and for any intermediate summaries
- available context budget or working budget for the current model
- chosen strategy: full-read, staged synthesis, grouped summarization, or scoped search

## Core Processing Policy
- For summary requests, default to full-document processing of each selected book or document.
- Do not silently truncate a document and then summarize as if it were fully processed.
- If all requested material cannot fit in one pass, use staged summarization or grouped synthesis and disclose that strategy.
- Only switch to skim/partial/chapter/topic processing when:
  - the user explicitly asks for that scope, or
  - the request is a search/query task targeting a specific topic, keyword, chapter, or passage.
- Ask the user only when the request is ambiguous enough that the system cannot choose a safe full-read or staged strategy on its own.

## Tooling Policy
- Prefer Rust-native backends first.
- Optional preinstalled helpers are allowed only when the Rust path is meaningfully incomplete.
- No runtime self-installation.
- Offline-first only.

## Initial Backend Direction
- plain text / markdown: native file read
- html: local html-to-text normalization
- pdf: Rust-native extraction where viable
- epub: Rust-native extraction where viable

## Boundaries
- Read-only only.
- No OCR in this task.
- No remote document fetching in this task.
- No extended ebook/archive formats in this task.

## Acceptance Criteria
- Multi-document summarization does not silently truncate source material without disclosure.
- The skill can choose between full-read and staged synthesis based on actual budget awareness.
- Elma can summarize supported documents.
- Elma can locate specific requested content and state where it was found.
- Extraction behavior is backend-aware instead of ad hoc per callsite.
- Failure is explicit when a backend is unavailable or extraction quality is insufficient.

## Required Tests
- one fixture per supported core format
- summary path over extracted chunks
- multi-document work-plan test that chooses staged synthesis when budget is insufficient
- needle-search path with source citations
- explicit failure path when backend capability is missing
