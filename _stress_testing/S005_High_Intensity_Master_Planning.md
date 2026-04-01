# Stress Test S005: High-Intensity Master Planning

## 1. The Test (Prompt)
"Develop a Master Plan to implement an Audit Log system for Elma CLI. This log should record every tool call, its parameters, and the model's reasoning into sessions/audit/. Plan the implementation phases, then implement the first phase: the core trait for auditing."

## 2. Expected Behavior
- **Route:** MASTERPLAN
- **Formula:** masterplan_reply
- **Steps:** 8-12 (MasterPlan step + phased implementation + reply)

## 3. Success Criteria
- Agent creates MasterPlan step with phases
- Implements first phase (core trait)
- Maximum 12 steps (absolute limit enforced)
- No duplicate steps (>50% duplicates = fail)

## 4. Common Failure Modes
- Plan is too vague to be actionable
- Plan collapse (35+ identical steps)
- Context explosion
