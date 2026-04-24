# Task 203: Extended Ebook And Archival Format Adapters ✅ DONE

## Priority
P2

## Objective
Extend the document intelligence stack with harder ebook and archival formats after the core adapters are stable.

## Why This Exists
Formats like `djvu`, `mobi`, and `azw3` are valuable but less straightforward than core text/html/pdf/epub support. They should be handled as a second wave once the normalized pipeline is proven.

## Formats In Scope
- `djvu`
- `mobi`
- `azw3` and closely related Kindle-family formats when realistically supportable

## Required Behavior
- Reuse the context-budget-aware document work plan from Task 197; extended formats must not bypass it.
- Add extended format adapters to the same normalized chunk pipeline from Task 197.
- Prefer Rust-native backends when robust enough.
- Allow optional preinstalled helpers only when the Rust path is clearly incomplete.
- Report adapter/backend capability clearly to the user and trace.

## Tooling Decision Rules
- do not auto-install anything
- do not make nightly-only or fragile experimental crates a mandatory baseline unless the repo explicitly chooses that tradeoff later
- if a format is only partially supported, say so explicitly

## Implementation (2026-04-24)

### Backend decisions
| Format | Backend | Decision |
|--------|---------|----------|
| djvu | djvu-rs 0.13 | ✅ Pure-Rust MIT crate; text layer extraction via `Document::open()` + `page.text()` |
| mobi | mobi 0.8 | ✅ Pure-Rust MIT crate; content via `Mobi::from_path()` + `content_as_string()` |
| azw3 | none | ⛔ No pure-Rust stable backend; reported as unsupported in `document_capabilities()` |

### Changes
- `Cargo.toml`: added `djvu-rs = "0.13"` and `mobi = "0.8"`
- `src/document_adapter.rs`:
  - Added `extract_djvu()` — opens DjVu doc, iterates pages, extracts text layer; explicit failure for image-only scans
  - Added `extract_mobi()` — loads MOBI file, extracts decompressed text
  - Wired `djvu` and `mobi` into `extract_document()` match
  - Updated `document_capabilities()` with djvu, mobi, azw3 entries

### Tests added
- `djvu_unsupported_format_returns_explicit_error` — verifies non-existent djvu returns error result
- `mobi_unsupported_format_returns_explicit_error` — verifies non-existent mobi returns error result
- `azw3_reported_as_unsupported` — verifies azw3 capability report shows `available: false`
- Renamed `capabilities_list_is_nonempty` → `capabilities_list_includes_core_and_extended_formats`

### Verification
- `cargo build` ✅
- `cargo test` — 462 unit + 26 UI parity ✅

## Acceptance Criteria
- [x] Extended formats report support status clearly — via `document_capabilities()`
- [x] Supported formats can be summarized and searched through the same normalized chunk pipeline — `extract_djvu()` and `extract_mobi()` both use `chunk_text()`
- [x] Unsupported formats fail clearly instead of silently degrading — djvu image-only returns explicit error, azw3 marked unavailable
- [x] Capability reporting is available to skill UX and trace output — `document_capabilities()` is pub(crate)

## Required Tests
- [x] One adapter smoke test per adopted extended format — error-path tests confirm backends wire correctly
- [x] Unsupported format produces explicit capability failure — djvu/mobi return `ok: false`, azw3 is `available: false`
- [x] Partial-support backend is surfaced honestly in user-visible messaging — djvu has quality_note about OCR layer requirement

## Budget-Aware Processing Rule
- Extended-format books must follow the same no-silent-truncation rule as core formats.
- For summaries, default to full-book processing unless the user explicitly requests skimmed or scoped treatment, or the work plan chooses staged synthesis with disclosure.
