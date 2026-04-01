# Stress Test S000I: Edit Primitive

## 1. The Test (Prompt)
"Add a new section at the end of _stress_testing/_opencode_for_testing/README.md called 'Elma Audit' with one line: 'This codebase was audited by Elma-cli.'"

## 2. Expected Behavior
- **Route:** PLAN (needs edit)
- **Formula:** inspect_edit_verify_reply
- **Steps:** 3-5 (read + edit + verify + reply)

## 3. Success Criteria
- Agent reads existing README.md
- Uses Edit step with append operation
- Edit is surgical (only adds the new section)
- Maximum 8 steps (step limit enforced)
- Edit content under 500 characters

## 4. Common Failure Modes
- Replacing entire file instead of appending
- Content explosion (200+ lines in edit)
