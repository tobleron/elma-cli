# 255 - Comic And Apple Book Package Adapters CBZ CBR IBA

## Backlog Reconciliation (2026-05-02)

Resume under Task 466 and Task 467 so format support claims, extraction cost, and cache behavior stay truthful.


Status: Pending
Priority: P2
Depends on: 248, 249, 251

## Goal

Recognize and process package-style ebook/comic formats without pretending that image pages are readable text.

## Format Scope

| Format | Strategy |
|---|---|
| `.cbz` | ZIP reader; metadata and embedded text sidecars |
| `.cbr` | RAR reader only behind explicit feature/license decision |
| `.iba` | ZIP/package inspection; extract embedded HTML/XHTML/EPUB-like content |

## CBZ Requirements

- Treat CBZ as a ZIP-based comic archive.
- Extract `ComicInfo.xml` metadata if present.
- Inventory image pages in sorted reading order.
- Extract any embedded `.txt`, `.html`, `.xhtml`, `.xml`, or OCR sidecar text.
- If only images are present, return metadata/degraded state with `requires_ocr_for_page_text`.

## CBR Requirements

- Evaluate `unrar` or another RAR-capable Rust crate.
- Do not add C/C++ unrar bindings to the baseline without explicit license/build acceptance.
- If no acceptable backend is chosen, recognize `.cbr` and report unsupported with a reason.
- If a backend is chosen, mirror CBZ behavior.

## IBA Requirements

Apple iBooks Author packages are ZIP-like bundles in many cases.

- Detect ZIP/package structure.
- Search for EPUB-like OPF/spine assets, XHTML/HTML pages, and metadata plist/XML.
- Reuse the EPUB/HTML extraction helpers.
- If the package is not extractable, report degraded/unsupported.

## Acceptance Criteria

- CBZ with `ComicInfo.xml` returns metadata.
- CBZ with OCR sidecar text returns chunks with archive-entry provenance.
- Image-only CBZ does not pretend to have readable page text.
- CBR capability state is explicit and tested.
- IBA extraction succeeds on a minimal package fixture or reports a stable degraded state.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test package_document_adapter`
- Fixtures for CBZ metadata-only, CBZ with text sidecar, image-only CBZ, CBR recognized state, and IBA minimal package.

