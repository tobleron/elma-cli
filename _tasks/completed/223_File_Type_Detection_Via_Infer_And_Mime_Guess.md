# 223: File Type Detection via `infer` and `mime_guess`

## Status
`pending`

## Crates
`infer` (#119) — Detect file type from magic bytes.
`mime_guess` (#120) — Guess MIME types from file extensions.

## Rationale
Elma processes diverse document formats (PDF, EPUB, DOCX, markdown, HTML). `infer` uses magic bytes to reliably detect file type regardless of extension — preventing user error when skill documents have wrong extensions. `mime_guess` fills in extension→MIME mapping for content-type-aware routing. Together they give Elma robust format detection.

## Implementation Boundary
- Add `infer = "0.5"` and `mime_guess` to `Cargo.toml`.
- Create `src/filetype.rs`:

  ```rust
  use infer::{Matcher, MatcherType};
  use mime_guess::mime;

  pub fn detect_type(data: &[u8]) -> Option<&'static str> {
      infer::get(data).map(|m| m.mime_type())
  }

  pub fn guess_mime(path: &Path) -> Option<&'static str> {
      mime_guess::from_path(path).first().map(|m| m.as_ref())
  }
  ```

- Replace any `path.extension()`-based format detection with `detect_type` as the authoritative path.
- Use `guess_mime` for HTTP `Content-Type` header construction.
- Apply to: skill document ingestion, file picker type filtering, repo explorer file type icons.
- Do NOT use magic bytes for files that are already definitively known (e.g., `.pdf` from `pdf-extract`).

## Verification
- `cargo build` passes.
- `detect_type(b"%PDF-1.4")` returns `"application/pdf"`.
- `detect_type(b"#!/bin/sh")` returns `"application/x-sh"`.
- Existing document processing unchanged.