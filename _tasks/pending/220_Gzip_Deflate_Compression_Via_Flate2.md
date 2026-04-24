# 220: Gzip/Deflate Compression via `flate2`

## Status
`pending`

## Crate
`flate2` — Gzip/zlib/deflate compression.

## Rationale
Elma communicates with local models (llama.cpp, Ollama) that often serve responses in gzip-compressed chunks (`Content-Encoding: gzip`). `reqwest` already handles this transparently with its `gzip` feature, but for skill document caching, embedded asset handling, or any custom compression needs, `flate2` provides a clean, no-dependency path. It's also useful for reading gzip-compressed skill archives or skill doc bundles.

## Implementation Boundary
- Add `flate2 = "1.0"` to `Cargo.toml` (already indirectly available via `reqwest` but explicit dep gives full control).
- Create `src/compress.rs` with:

  ```rust
  use flate2::{read::GzDecoder, write::GzEncoder, Compression};
  use std::io::{Read, Write};

  pub fn gzip_compress(data: &[u8]) -> Vec<u8> {
      let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
      encoder.write_all(data).unwrap();
      encoder.finish().unwrap()
  }

  pub fn gzip_decompress(data: &[u8]) -> anyhow::Result<Vec<u8>> {
      let mut decoder = GzDecoder::new(data);
      let mut out = Vec::new();
      decoder.read_to_end(&mut out)?;
      Ok(out)
  }
  ```

- Identify where gzip is needed: skill document compression in cache, potential skill archive reading.
- Do NOT add new file formats or replace existing streaming approaches.
- Do NOT use `brotli` or `zstd` yet unless a concrete need emerges.

## Verification
- `cargo build` passes.
- `gzip_compress(b"hello")` produces valid gzip output (verify with `gunzip`).
- `gzip_decompress(gzip_compress(b"hello")) == b"hello"`.
- Existing reqwest behavior unchanged.