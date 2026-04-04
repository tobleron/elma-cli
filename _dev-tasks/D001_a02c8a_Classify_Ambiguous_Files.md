# Task D001_a02c8a: Classify Ambiguous Files

## Objective
Code Taxonomist. Classify unknown files to enable accurate analysis. Add an @efficiency-role: <role> tag to the file header (CRITICAL: must include the colon). Valid roles: **infra-adapter**: External API clients, database drivers, and third-party bindings.; **scenario-spec**: Scenario definitions and testing specifications.; **data-model**: Type definitions, schemas, and DTOs (low logic density).; **orchestrator**: App entry points and high-level flow control.; **infra-config**: Build scripts, project configuration, and environment setups.; **util-pure**: Side-effect free helper functions.; **service-orchestrator**: Complex coordination between multiple domain services.; **domain-logic**: Pure business logic, entities, and domain services.; **ignored**: Exclude this file from all efficiency metrics and tasks.


## Work Items
### 🔧 Action: Classify Ambiguous Files
**Directive:** Taxonomy Resolution: Add the required @efficiency-role: <role> tag (including colon) to help the analyzer apply the correct complexity limits.
- [ ] `../../src/app_chat_loop.rs`
- [ ] `../../src/execution_steps_selectors.rs`
- [ ] `../../src/orchestration_loop_verdicts.rs`
