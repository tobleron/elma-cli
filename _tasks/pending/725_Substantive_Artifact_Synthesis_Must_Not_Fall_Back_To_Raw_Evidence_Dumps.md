# Task 725: Substantive Artifact Synthesis Must Not Fall Back To Raw Evidence Dumps

## Type

Finalization / Artifact Quality / Autonomous Completion

## Severity

High

## Evidence

Round 8 prompt testing still produced raw recovered artifacts:

- Prompt 01 created `project_tmp/security_report.md` as:
  - `# Recovered Artifact: project_tmp/security_report.md`
  - `This file was generated from captured tool evidence because model-based artifact synthesis failed or returned empty content.`
- Prompt 03 ended:
  - `project_tmp/report.md (evidence recovery — contains raw tool output, not a substantive report)`
- Prompt 04 ended:
  - `project_tmp/report.md (evidence recovery — contains raw tool output, not a substantive report)`
- Prompt 05 ended:
  - `project_tmp/testing_report.md (evidence recovery — contains raw tool output, not a substantive report)`

These are visible in:

- `sessions/s_1778146613_189124000/session.md`
- `sessions/s_1778147182_938803000/session.md`
- `sessions/s_1778147274_655146000/session.md`
- `sessions/s_1778147612_177368000/session.md`

## Problem

Evidence recovery is useful as a forensic fallback, but it is not a completed user deliverable. Report requests require a synthesized artifact with findings, scope, methodology, limitations, and citations or file locations. Raw tool dumps are hard to use and often include broad workspace noise.

The system correctly labels some recovered artifacts as partial, but it still spends multiple continuations without reliably converting evidence into a substantive report.

## Requirements

- Add a deterministic structured-report synthesis fallback for common report artifacts when model synthesis fails.
- Use collected evidence to produce sections such as Summary, Scope, Methodology, Findings, Evidence, Limitations, and Next Steps.
- Preserve file paths and line numbers from evidence when present.
- Never mark `Recovered Artifact` evidence dumps as complete.
- If model-authored synthesis fails, deterministic synthesis should create a readable report instead of dumping raw tool output.
- Keep continuation bounded: after a recovered artifact is written, run one focused synthesis attempt or deterministic synthesis, then stop with `partial` only if quality remains insufficient.

## Likely Files

- `src/artifact_verifier.rs`
- `src/finalization_verifier.rs`
- `src/tool_loop.rs`
- `src/final_answer.rs`
- `src/evidence_ledger.rs`

## Acceptance Criteria

- [ ] Prompt 01 produces a substantive security report or explicitly partial final answer with a readable synthesized artifact.
- [ ] Prompt 03/04/05 no longer produce `# Recovered Artifact` as the primary deliverable.
- [ ] Artifact state distinguishes raw evidence recovery from deterministic structured synthesis.
- [ ] Tests cover empty model synthesis, raw recovered evidence, and deterministic report generation.

