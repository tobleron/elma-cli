# 216: Human-Readable Size Formatting via `humansize`

## Status
`pending`

## Crrate
`humansize` — Human-readable file sizes and byte counts.

## Rationale
Elma displays file sizes, token counts, context usage, and model budget information in transcript output. Raw numbers (`1048576 bytes`) are harder to parse than `1.0 MB`. `humansize` provides configurable decimal/binary formatting with minimal API surface.

## Implementation Boundary
- Add `humansize = "4.0"` to `Cargo.toml`.
- Audit places where Elma outputs byte counts, token counts, or size information:
  - Context window usage display (compact boundary / footer)
  - File sizes in transcript (e.g., in the repo explorer skill output)
  - Cache/model response sizes if displayed
  - Skill document sizes if shown to users
- Create a shared formatter in `src/format.rs`:

  ```rust
  use humansize::{format_size, BINARY, DECIMAL};

  pub fn file_size(bytes: u64) -> String {
      format_size(bytes, BINARY)
  }

  pub fn token_count(n: usize) -> String {
      n.to_string()
  }
  ```

- Replace at least two raw `bytes`/`size` numeric displays with `humansize` formatting.
- Keep the original number in machine-readable form when needed (e.g., compact boundary tooltip).
- Do NOT change data types or internal calculations — only the display layer.

## Verification
- `cargo build` passes.
- `file_size(1048576)` returns `"1.0 MiB"`.
- `file_size(1500)` returns `"1.5 KiB"`.
- Existing output layout preserved.