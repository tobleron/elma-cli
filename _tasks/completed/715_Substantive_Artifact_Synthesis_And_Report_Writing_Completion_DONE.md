# Task 715: Substantive Artifact Synthesis And Report Writing Completion

## Type

Artifact Completion / Finalization Quality

## Severity

Critical

## Evidence

Round 6 prompts 01-07 exited successfully and created final answer artifacts, but most requested report files were recovered evidence dumps rather than substantive reports:

- `project_tmp/security_report.md`
- `project_tmp/report.md`
- `project_tmp/testing_report.md`

These files begin with `# Recovered Artifact` and say they were generated because model-based synthesis failed or returned empty content. Final answers then claimed completion with only:

```text
Completed the requested artifact work.
Created or updated:
- project_tmp/security_report.md
```

Relevant traces:

- `finalization_stage=deterministic_artifact_completion`
- `StopReason: iteration_limit_reached`

## Problem

Elma is preserving liveness by creating fallback artifacts, but the fallback does not satisfy the user's actual report-writing request. This creates a semantic continuity failure: the user asked for a detailed analysis/report, and Elma produced raw evidence while reporting success.

## Requirements

- Distinguish three artifact states:
  - model-authored artifact completed
  - deterministic evidence recovery created a partial artifact
  - artifact could not be completed
- If synthesis fails, run a bounded clean-context synthesis pass that converts evidence into the requested report format before falling back to raw evidence.
- The fallback report must include:
  - direct answer to the user objective
  - evidence table or bullets with file paths and line numbers when available
  - findings grouped by severity/topic when the prompt asks for analysis
  - explicit uncertainty when evidence is insufficient
- Do not mark a recovered evidence dump as fully completed unless it meets a minimal report-quality verifier.
- Add a report-quality verifier for required sections and requested file path existence.
- Persist the verifier result in session trace and transcript-native notice rows.
- Keep model calls bounded and local-first.

## Acceptance Criteria

- [ ] Prompt 01 produces a security report with specific files/line numbers or a grounded no-findings conclusion.
- [ ] Prompt 02 produces a configuration security report, not a raw evidence dump.
- [ ] Prompt 05 produces a test coverage report, not only glob/search output.
- [ ] Final answers disclose partial completion when only evidence recovery was possible.
- [ ] Tests cover deterministic artifact recovery versus substantive artifact completion.

