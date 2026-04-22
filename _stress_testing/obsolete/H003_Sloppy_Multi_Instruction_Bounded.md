# Sloppy Human Test H003: Sloppy Multi-Instruction Bounded Workflow

## 1. The Test (Prompt)
"inside _stress_testing/_opencode_for_testing/ only, read README.md, tell me in 2 bullets what this repo is for, then identify the primary entry point by exact path, and do not modify anything"

## 2. Expected Behavior
- **Route:** WORKFLOW
- **Formula:** grounded read/search/select/reply style workflow
- **Scope:** must remain inside `_stress_testing/_opencode_for_testing/`

## 3. Success Criteria
- Elma reads grounded sandbox evidence instead of answering from chat alone.
- The answer includes exactly 2 bullets for repo purpose.
- The answer includes the exact grounded relative path for the primary entry point.
- No files are modified.
- No claims are made about files not observed in the sandbox.

## 4. Common Failure Modes
- Incorrectly routing to CHAT
- Hallucinating repo purpose from priors instead of README evidence
- Returning a fake or incomplete entry point
- Losing the exact path and replying with a softened basename only
