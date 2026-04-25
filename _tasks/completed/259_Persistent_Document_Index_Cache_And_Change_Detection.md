# 259 - Persistent Document Index Cache And Change Detection

Status: Pending
Priority: P1
Depends on: 257, 258

## Goal

Avoid re-extracting and re-chunking unchanged ebooks while keeping cached indexes correct and safe.

## Requirements

- Compute a source signature from canonical path, file size, modified time, and content hash.
- Store extracted metadata, units, chunks, quality report, and backend version.
- Invalidate cache when source content or adapter version changes.
- Keep cache files under Elma's existing storage/session conventions.
- Avoid writing into the user's source directory.
- Use atomic writes for cache updates.
- Handle concurrent reads with locking or write-then-rename semantics.
- Add cache cleanup policy for stale document indexes.

## Storage Shape

The cache must be able to answer:

- Which source file produced this index?
- Which backend and version produced it?
- Which formats and quality flags apply?
- Are chunks still valid for current chunking config?
- Which embeddings or retrieval indexes are attached?

## Acceptance Criteria

- Re-reading an unchanged large EPUB uses the cache.
- Changing the source file invalidates the cache.
- Changing chunking config invalidates chunk-dependent cache entries.
- Corrupt cache files are ignored and rebuilt.
- Cache hits and misses are visible in transcript telemetry when relevant.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_cache`
- Tests for unchanged file, modified file, corrupt cache, backend-version bump, and concurrent read behavior.

