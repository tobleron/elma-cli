# Stress Test S000B: Shell Primitive

## 1. The Test (Prompt)
"List the files in _stress_testing/_opencode_for_testing/ and identify the primary entry point of this codebase."

## 2. Expected Behavior
- **Route:** SHELL
- **Formula:** execute_reply or inspect_reply
- **Steps:** 2-4 (ls command + optional inspection + reply)

## 3. Success Criteria
- Agent executes ls on the test directory
- Correctly identifies main entry point (main.rs, Cargo.toml, etc.)
- Maximum 8 steps (step limit enforced)
- No duplicate/repeated commands

## 4. Common Failure Modes
- Incorrect pathing (wrong directory)
- Plan collapse (40+ identical steps)
- Duplicate step loops
