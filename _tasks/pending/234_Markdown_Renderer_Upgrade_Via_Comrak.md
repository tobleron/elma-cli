# 234: Markdown Renderer Upgrade via `comrak`

## Status
`pending`

## Crate
`comrak` — CommonMark/GFM Markdown parser and renderer.

## Rationale
Elma currently uses `pulldown-cmark` (0.12) for Markdown rendering in the TUI. `comrak` is a newer, actively maintained crate with full CommonMark + GFM support, better handling of tables, task lists, strikethrough, and footnotes — plus HTML sanitization via `ammonia` integration. Worth considering as a direct upgrade or complementary renderer alongside `pulldown-cmark`.

## Implementation Boundary
- Add `comrak = "0.5"` to `Cargo.toml`.
- Create `src/markdown_comrak.rs` as a parallel renderer:

  ```rust
  use comrak::{parse_document, Arena, Options};

  pub fn render_markdown(input: &str) -> String {
      let arena = Arena::new();
      let options = Options::default();
      let root = parse_document(&arena, input, &options);
      comrak::format_html(&root, &options, &mut String::new())
  }
  ```

- Evaluate `comrak` as a drop-in replacement for `pulldown_cmark` in the markdown rendering path (`src/renderer.rs` or wherever `pulldown_cmark` is used).
- Compare output quality: tables, task lists, strikethrough, code blocks, footnotes.
- Keep `pulldown_cmark` as the baseline if `comrak` doesn't improve visibly.
- Do NOT replace `pulldown_cmark` everywhere — add `comrak` as a configurable option.
- Add `#[cfg(feature = "comrak")]` or a runtime renderer selection.

## Verification
- `cargo build` passes.
- GFM tables, task lists, and strikethrough render correctly in `comrak`.
- Existing `pulldown_cmark` output unchanged unless explicitly switched.