# Task 721: Substantive Report Artifacts Still Recovered Evidence Dumps

## Type

Artifact Synthesis / Semantic Completion

## Severity

Critical

## Evidence

Round 7 prompt testing still produced raw recovered artifacts for report prompts:

- `project_tmp/security_report.md`
- `project_tmp/report.md`
- `project_tmp/testing_report.md`

They still begin with:

```text
# Recovered Artifact: ...
This file was generated from captured tool evidence because model-based artifact synthesis failed or returned empty content.
```

Final answers still report:

```text
Completed the requested artifact work.
Created or updated:
- `project_tmp/security_report.md`
```

Relevant sessions:

- `project_tmp/round7_sessions/prompt_01_s_1778142976_493585000`
- `project_tmp/round7_sessions/prompt_02_s_1778143030_235992000`
- `project_tmp/round7_sessions/prompt_05_s_1778143178_46509000`

## Problem

Task 715 did not achieve substantive report writing. Elma is still claiming successful completion when the artifact is an evidence dump. This is a semantic continuity failure and must be fixed before dense-model testing.

## Requirements

- Add an artifact state enum or equivalent persisted status:
  - `complete_model_authored`
  - `complete_deterministic_structured`
  - `partial_evidence_recovery`
  - `failed`
- Never return `Completed the requested artifact work` for `partial_evidence_recovery`.
- Build a deterministic structured report fallback for common report requests using evidence already gathered:
  - objective summary
  - findings table
  - evidence references with file paths and line numbers
  - no-findings conclusion when appropriate
  - limitations/uncertainty section
- Add a verifier that rejects files whose top heading is `Recovered Artifact` when the user requested a report.
- Persist artifact state in trace, session metadata, and transcript-native notice rows.
- Keep model-based synthesis bounded; do not solve this by adding giant examples.

## Acceptance Criteria

- [ ] Prompt 01 creates a security report, not a raw evidence dump.
- [ ] Prompt 02 creates a config/security report with recommendations, not a raw evidence dump.
- [ ] Prompt 05 creates a testing report, not a raw evidence dump.
- [ ] Final answer explicitly says partial if only evidence recovery was possible.
- [ ] Session status is not plain `completed` when required artifacts are only partial.

