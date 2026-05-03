# Task 468: Data Analysis Mode

**Status:** Active
**Priority:** MEDIUM
**Estimated effort:** 3-5 days
**Dependencies:** Task 389, Task 465, Task 466 (completed), Task 467
**References:** Directive 002, Proposal 005, objectives.md evidence-grounded answers

## Objective

Implement a dispatchable `DataAnalysis` mode for document and dataset analysis after document capability truth and extraction resource bounds are reconciled.

## Scope

Supported local inputs should include already-supported formats where available:

- PDF (fully supported)
- TXT (fully supported)
- Markdown (fully supported)
- HTML (fully supported)
- **EPUB, MOBI, DjVu, DOCX, RTF**: Currently marked as unsupported (Task 466 reconciliation). Will be added when full implementations are complete.
- CSV/structured text when feasible

## Implementation Plan

1. Add `ExecutionMode::DataAnalysis` if it does not already exist.
2. Route mode selection through existing command/mode infrastructure without keyword triggers.
3. Use existing document adapters before shell or external tools.
4. Build the analysis flow through the pyramid graph:
   - objective
   - analysis goals
   - evidence chunks
   - extraction instructions
   - final synthesis
5. Keep every intel-unit JSON output within Task 378 limits.
6. Add transcript rows for extraction, chunking, evidence selection, and final synthesis.
7. Render final responses as plain text; create markdown artifacts only through Task 392 when requested.

## Non-Scope

- Do not modify `src/prompt_core.rs` without explicit approval.
- Do not make network calls for data enrichment unless optional network tools are enabled.
- Do not load entire large documents into one model call.

## Verification

```bash
cargo test data_analysis
cargo test document
cargo test evidence
cargo build
```

Manual probe:

```bash
cargo run -- --mode data-analysis
```

## Done Criteria

- Local document analysis is grounded in extracted evidence.
- Large inputs are chunked and summarized safely.
- Mode changes are visible in the transcript.
- Plain text remains the terminal default.
