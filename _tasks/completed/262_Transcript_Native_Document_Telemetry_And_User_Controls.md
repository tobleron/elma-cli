# 262 - Transcript-Native Document Telemetry And User Controls

Status: Pending
Priority: P2
Depends on: 247, 258, 259, 260, 261

## Goal

Expose document extraction, indexing, retrieval, and context-budget decisions in the chat transcript without adding noisy footer/status-bar state.

## Transcript Rows

Add compact, collapsible rows for:

- detected format and backend
- extraction quality warnings
- page/chapter/unit counts
- chunk count and token estimate
- cache hit/miss
- chosen work plan
- retrieval result count and source spread
- unsupported/degraded format explanations
- optional feature requirements such as OCR or CBR backend

## User Controls

Add or extend commands only after the core pipeline works:

- `/documents` to list loaded document indexes in the current session.
- `/document info <path-or-id>` to show metadata and quality report.
- `/document forget <path-or-id>` to remove a cached session reference.
- `/document reindex <path-or-id>` to force cache invalidation.

## UI Constraints

- Do not use the bottom status/footer bar for document notifications.
- Do not reintroduce legacy Elma chrome.
- Keep rows Claude-like and transcript-native.
- Keep verbose details collapsed by default.

## Acceptance Criteria

- Reading a large ebook shows what Elma extracted and what strategy it chose.
- Unsupported formats explain the reason in the transcript.
- Cache behavior is visible when relevant.
- Users can inspect document state without digging through debug logs.

## Verification

- `cargo fmt`
- `cargo build`
- Relevant UI/transcript tests.
- Real CLI validation with PDF, EPUB, CBZ metadata-only, and unsupported KFX/LRX fixture.

