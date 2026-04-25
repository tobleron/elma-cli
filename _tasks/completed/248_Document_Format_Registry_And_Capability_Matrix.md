# 248 - Document Format Registry And Capability Matrix

Status: completed
Priority: P1
Depends on: 247

## Problem

The adapter currently routes mostly by extension. That is not enough for ebooks because several requested formats share containers, have misleading extensions, or are commonly DRM/image-only:

`.azw`, `.mobi`, `.pdb`, `.doc`, `.chm`, `.lit`, `.iba`, `.cbz`, and `.cbr` all need more than filename matching.

## Goal

Create a canonical document format registry that defines detection, capability state, backend, quality limits, and user-facing explanations for every supported or recognized format.

## Required Format Matrix

| Format | Baseline state | Notes |
|---|---|---|
| `.txt` | Full text | Encoding-aware plain text |
| `.html`, `.xhtml` | Full text | HTML cleanup and structure labels |
| `.epub` | Full text | OPF/spine/TOC/chapter-aware |
| `.pdf` | Full text when text layer exists | No OCR by default |
| `.mobi` | Full text | Legacy MOBI text and metadata |
| `.azw` | Full/degraded | Usually MOBI-like; may be DRM |
| `.azw3` | Full/degraded | Evaluate `boko`; DRM remains unsupported |
| `.kfx` | Full/degraded | Evaluate `boko`; DRM remains unsupported |
| `.fb2` | Full text | XML FictionBook |
| `.djvu` | Full text when text layer exists | Image-only files fail clearly |
| `.rtf` | Full/degraded | Evaluate parser and fallback cleaner |
| `.docx` | Full text | ZIP/XML extraction |
| `.doc` | Degraded/full if feasible | Legacy CFB Word parsing is limited |
| `.cbz` | Metadata/degraded | Text only if metadata/text sidecars exist unless OCR feature lands |
| `.cbr` | Metadata/degraded or unsupported | RAR backend must be feature/license gated |
| `.iba` | Full/degraded | ZIP package with embedded HTML/XHTML/EPUB-like assets |
| `.lit` | Unsupported/degraded | CHM-derived legacy format; no false claims |
| `.pdb` | Full/degraded for PalmDoc only | Must not confuse with Microsoft debug PDB |
| `.lrf` | Unsupported/degraded | Legacy Sony BBeB |
| `.lrx` | Unsupported | DRM-oriented Sony BBeB variant |
| `.chm` | Degraded/unsupported | CHM extraction backend requires explicit decision |

## Implementation Requirements

- Add a `DocumentFormat` enum or equivalent canonical type.
- Add signature sniffing using existing `infer`, `mime_guess`, magic bytes, and container probes.
- Add a capability report with `supported`, `degraded`, `unsupported`, `requires_feature`, and `requires_text_layer` states.
- Make every user-facing unsupported message stable and actionable.
- Add tests for extension mismatch, uppercase extension, missing extension, and wrong-extension files.

## Acceptance Criteria

- Every requested format appears in `document_capabilities()`.
- The read path can distinguish ZIP-based EPUB/DOCX/CBZ/IBA by container contents when possible.
- `.pdb` is not blindly treated as an ebook.
- DRM or image-only files are not described as successfully read.
- Unsupported formats return structured capability errors suitable for transcript display.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test document_format`
- Fixture tests for magic-byte and extension fallback behavior.

