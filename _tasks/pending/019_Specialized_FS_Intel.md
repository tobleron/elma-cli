# Task 035: Specialized "File-System" Intel Units

## Context
Shell-based inspection is generic. Specialized parsers for project-specific files (Cargo.toml, package.json, etc.) would be more efficient.

## Objective
Add "Structured Observation" units:
- Implement lightweight Rust parsers for common config formats.
- Provide these structured facts to the planner instead of raw `cat` or `grep` output.
- Reduce token usage and increase accuracy for project-discovery tasks.

## Success Criteria
- Faster and more accurate project summaries.
- Reduced context noise from config file content.
