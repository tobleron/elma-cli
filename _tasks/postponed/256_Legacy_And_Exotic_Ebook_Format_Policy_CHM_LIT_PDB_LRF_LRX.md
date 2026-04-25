# 256 - Legacy And Exotic Ebook Format Policy CHM LIT PDB LRF LRX

Status: Pending
Priority: P2
Depends on: 248, 249

## Goal

Handle legacy and exotic ebook formats honestly: recognize them, extract when a safe Rust-native path is feasible, and provide clear unsupported/degraded reports otherwise.

## Format Scope

| Format | Baseline target |
|---|---|
| `.chm` | Recognize; evaluate extraction backend; do not enable C dependency silently |
| `.lit` | Recognize; likely unsupported/degraded |
| `.pdb` | PalmDoc/Palm Database only, after signature sniffing |
| `.lrf` | Recognize; likely unsupported/degraded |
| `.lrx` | Recognize; unsupported if DRM-oriented |

## CHM Requirements

CHM is a compiled HTML Help container. Rust options are weak or C-backed.

- Evaluate `chmlib`/CHM crates and license/build implications.
- Prefer a pure Rust implementation only if one can extract internal HTML and TOC reliably.
- If C bindings are needed, make them feature-gated and off by default unless approved.
- Reuse HTML/XHTML extraction after unpacking internal pages.
- Never execute CHM content.

## LIT Requirements

`.lit` is related to CHM-era Microsoft Reader content and may involve DRM.

- Recognize signature/extension.
- Do not claim full extraction without fixtures.
- If an extractor is implemented, it must avoid executing embedded content and must preserve internal HTML provenance.

## PDB Requirements

`.pdb` is ambiguous. The Rust `pdb` crate is for Microsoft debug symbols, not Palm ebooks.

- Detect Palm Database signatures before treating `.pdb` as an ebook.
- Implement PalmDoc text extraction only if the record format and compression are fixture-backed.
- Otherwise report unsupported/degraded with ambiguity explained.

## LRF/LRX Requirements

- Recognize Sony BBeB extensions.
- Treat `.lrx` as unsupported when DRM is detected or assumed.
- Do not add weak parsers that only dump binary strings.

## Acceptance Criteria

- Each format has a capability matrix entry.
- Each format has a fixture-backed recognized unsupported/degraded path.
- `.pdb` debug-symbol files are not misclassified as ebooks.
- No CHM/LIT content is executed.

## Verification

- `cargo fmt`
- `cargo build`
- `cargo test legacy_ebook_capabilities`
- Fixture tests for extension recognition and safe failure messages.

