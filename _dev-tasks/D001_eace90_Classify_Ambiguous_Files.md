# Task D001_eace90: Classify Ambiguous Files

## Objective
Code Taxonomist. Classify unknown files to enable accurate analysis. Add an @efficiency-role: <role> tag to the file header (CRITICAL: must include the colon). Valid roles: **domain-logic**: Pure business logic, entities, and domain services.; **infra-adapter**: External API clients, database drivers, and third-party bindings.; **scenario-spec**: Scenario definitions and testing specifications.; **service-orchestrator**: Complex coordination between multiple domain services.; **orchestrator**: App entry points and high-level flow control.; **infra-config**: Build scripts, project configuration, and environment setups.; **ignored**: Exclude this file from all efficiency metrics and tasks.; **util-pure**: Side-effect free helper functions.; **data-model**: Type definitions, schemas, and DTOs (low logic density).


## Work Items
### 🔧 Action: Classify Ambiguous Files
**Directive:** Taxonomy Resolution: Add the required @efficiency-role: <role> tag (including colon) to help the analyzer apply the correct complexity limits.
- [ ] `../../src/decomposition.rs`
- [ ] `../../src/execution_steps_read.rs`
- [ ] `../../src/execution_steps_search.rs`
- [ ] `../../src/formulas/patterns.rs`
- [ ] `../../src/formulas/scores.rs`
- [ ] `../../src/orchestration_retry_tests.rs`
- [ ] `../../src/prompt_constants.rs`
- [ ] `../../src/tools/cache.rs`
- [ ] `../../src/tools/discovery.rs`
- [ ] `../../src/tools/registry.rs`
- [ ] `../../src/workspace_tree.rs`
