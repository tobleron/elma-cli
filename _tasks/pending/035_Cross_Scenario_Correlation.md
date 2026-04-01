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
