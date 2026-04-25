# 222: Stream Adapters via `tokio-stream`

## Status
`pending`

## Crate
`tokio-stream` — Stream adapters for Tokio.

## Rationale
Elma already uses `tokio` and `futures`. `tokio-stream` bridges `AsyncRead`/`AsyncBufRead` with `Stream` semantics and provides `StreamExt` utilities (e.g., `for_each_concurrent`, `buffer_unordered`). Useful when Elma processes SSE streams, chunked downloads, or any async iteration over model responses.

## Implementation Boundary
- Add `tokio-stream = "0.1"` to `Cargo.toml`.
- Audit existing `futures::stream::StreamExt` usage (already imported via `futures`).
- Add `tokio_stream::StreamExt` alongside existing `futures::StreamExt` where `futures` doesn't cover the use case.
- Identify async iteration patterns that could use `tokio_stream`: model response streaming, file streaming, SSE parsing.
- Keep `futures::StreamExt` as the primary stream combinator library — `tokio-stream` extends it for `AsyncRead`/`AsyncBufRead` bridging.
- Do NOT replace `futures` entirely.

## Verification
- `cargo build` passes.
- Existing async streaming behavior unchanged.