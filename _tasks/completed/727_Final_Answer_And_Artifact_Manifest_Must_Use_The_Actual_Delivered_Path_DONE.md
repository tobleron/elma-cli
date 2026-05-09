# Task 727: Final Answer And Artifact Manifest Must Use The Actual Delivered Path

## Type

Final Answer / Artifact Tracking / Evidence Alignment

## Severity

Medium

## Evidence

Prompt 06 wrote a specific artifact:

```text
> **write:** running: project_tmp/insecure_endpoints_report.md
> TOOL OK [write] id=DtnkP127: written
```

It then responded with completion, but finalization emitted:

```text
Partial completion:
- project_tmp/report.md (evidence recovery — contains raw tool output, not a substantive report)
```

The actual file on disk is also inconsistent with casing and naming:

- `project_tmp/insecure_endpoints_report.md`
- `project_tmp/INSECURE_ENDPOINTS_REPORT.md`
- `project_tmp/report.md`

The evidence gate also flagged broad unsupported claims even though the write tool evidence had a concrete delivered path.

Session:

- `sessions/s_1778147724_760976000/session.md`

## Problem

Artifact tracking and finalization can drift away from the actual file written by the model. This confuses the user, weakens resumability, and causes false partial-completion reports even when a concrete artifact exists.

## Requirements

- When the model writes an artifact, register that exact path in the artifact manifest.
- If the requested filename was inferred as generic `project_tmp/report.md` but the model writes a more specific report path, reconcile the required artifact to the delivered path when semantic intent matches.
- Normalize case-sensitive duplicates only when safe; do not overwrite user files.
- Final answer must list the actual delivered path, not a stale inferred path.
- Evidence alignment should cite the write event and any verification events used to validate the file.

## Likely Files

- `src/artifact_verifier.rs`
- `src/finalization_verifier.rs`
- `src/final_answer.rs`
- `src/evidence_ledger.rs`
- `src/tool_calling.rs`

## Acceptance Criteria

- [ ] Prompt 06 final answer names `project_tmp/insecure_endpoints_report.md` or the actual delivered artifact.
- [ ] No false `project_tmp/report.md` partial message appears when a specific report was written and verified.
- [ ] Artifact manifest records model-written paths and inferred required paths with a reconciliation reason.
- [ ] Tests cover generic requested report path replaced by a semantically specific delivered report.

