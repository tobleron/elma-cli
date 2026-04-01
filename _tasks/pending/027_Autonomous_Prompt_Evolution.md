# Task 031: Autonomous Prompt Evolution

## Context
The `sync_and_upgrade_profiles` mechanism currently uses hardcoded patches. The agent should be able to improve its own prompts based on feedback.

## Objective
Implement a mechanism for autonomous prompt refinement:
- When a `critic` or `outcome_verifier` identifies a recurring reasoning failure, propose a prompt tweak.
- Allow the user to review and "commit" prompt upgrades to their local configuration.

## Success Criteria
- System improves its own performance over time through self-correction.
- Reduced need for manual prompt engineering by developers.
