# Task 077: Cross-Scenario Correlation

**Status:** POSTPONED until P0-1, P0-2, P0-3, P0-4 complete

**Reason:** Per REPRIORITIZED_ROADMAP.md, these advanced features are blocked until the 4 foundational pillars are stable:
- P0-1: JSON Reliability (Tasks 001-004)
- P0-2: Context Narrative (Tasks 005-007)
- P0-3: Workflow Sequence (Tasks 008-011)
- P0-4: Reliability Tasks (Tasks 012-018)

**Do not start work on this task** until all P0-1 through P0-4 tasks are complete.

---

# Task 032: Cross-Scenario Correlation Tuning

## Context
Scenario probes (`./run_intention_scenarios.sh`) provide valuable performance data that is currently only manually reviewed.

## Objective
Integrate scenario results into the runtime:
- Automatically adjust `router_calibration.toml` or profile `temperature` based on scenario success/failure rates.
- Create a closed-loop tuning system that optimizes the agent for the specific workspace it's running in.

## Success Criteria
- Improved "Intention" accuracy after running probes.
- Automated optimization of calibration parameters.
