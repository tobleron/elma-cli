# 224: Text Encoding Support via `encoding_rs`

## Status
`pending`

## Crate
`encoding_rs` — Decode/encode common text encodings.

## Rationale
Skill documents, config files, and user workspace files may use encodings other than UTF-8 (e.g., Windows-1252, Shift-JIS, GBK). `encoding_rs` handles all IANA-registered encodings efficiently and is the same library used by Firefox. Essential for Elma's document processing pipeline to avoid garbled output on non-UTF-8 files.

## Implementation Boundary
- Add `encoding_rs = "0.8"` to `Cargo.toml`.
- Create `src/encoding.rs`:

  ```rust
  use encoding_rs::{Encoding, UTF_8};
  use std::borrow::Cow;

  pub fn decode(data: &[u8], hint_encoding: Option<&'static str>) -> Cow<str> {
      let encoding = hint_encoding
          .and_then(encoding_rs::Encoding::for_label)
          .unwrap_or(UTF_8);
      let (decoded, _, had_errors) = encoding.decode(data);
      if had_errors {
          UTF_8.decode(data).0
      } else {
          decoded
      }
  }
  ```

- Integrate into the document ingestion pipeline (ebook extraction, skill doc reading, workspace file processing).
- Apply `decode()` before any `String` conversion on non-UTF-8 input.
- Use `encoding_rs::Encoder` for any output encoding needs.
- Do NOT replace `String`/`&str` in memory — only at the I/O boundary.

## Verification
- `cargo build` passes.
- `decode(b"hello", None)` returns `"hello"`.
- `decode(b"\xc0\xe4\xe4", Some("windows-1252"))` returns `"äö"`.
- Existing UTF-8 document processing unchanged.