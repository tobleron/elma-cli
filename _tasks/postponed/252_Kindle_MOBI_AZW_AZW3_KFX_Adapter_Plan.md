# 252 - Kindle MOBI AZW AZW3 KFX Adapter Plan

Status: Pending
Priority: P1
Depends on: 249

## Goal

Support Kindle-family formats honestly: full text when unencrypted and parseable, degraded metadata when only partial extraction works, and explicit unsupported state for DRM or unsupported containers.

## Format Scope

| Format | Initial strategy |
|---|---|
| `.mobi` | Use existing `mobi` crate; improve metadata and content cleanup |
| `.azw` | Try MOBI-compatible path first; report DRM/unknown variants |
| `.azw3` | Evaluate `boko`; otherwise explicit unsupported/degraded |
| `.kfx` | Evaluate `boko`; otherwise explicit unsupported/degraded |

## Backend Evaluation

`boko` is a Rust library and CLI that advertises reading EPUB, KFX, AZW3, and MOBI and compiles content into a semantic IR. It currently requires nightly, so it cannot be blindly added to Elma's stable baseline. This task must decide whether to:

- add `boko` behind an explicit feature gate,
- vendor or adapt only stable pieces,
- wait until the crate supports stable Rust,
- or keep AZW3/KFX as recognized unsupported formats.

## Implementation Requirements

- Improve `.mobi` metadata extraction:
  - title
  - author
  - publisher
  - language
  - ISBN
  - publication date
- Normalize extracted HTML-like content through the shared HTML/XHTML cleaner.
- Detect DRM/encryption indicators when exposed by the backend.
- Do not attempt to bypass DRM.
- For `.azw`, try the MOBI path only after signature/container checks suggest it is MOBI-like.
- For `.azw3` and `.kfx`, do not claim support until fixtures pass.

## Acceptance Criteria

- A normal MOBI fixture produces metadata and chunks.
- `.azw` either extracts as MOBI-compatible content or reports a precise unsupported/degraded reason.
- `.azw3` and `.kfx` have tested capability states.
- DRM-protected Kindle files produce a clear refusal/error, not garbage text.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test kindle_document_adapter`
- Fixture tests for MOBI, AZW-like MOBI, AZW3 recognized unsupported/degraded, KFX recognized unsupported/degraded.

