# Task 003: Consolidate Verification And Outcome Module

## Objective
Unify Elma's fragmented verification logic into a single `src/verification.rs` module and make outcome validation strong enough to improve solve rate, not just code cleanliness.

## Context
Recent sessions show the main weakness is not routing alone. Elma often:
- runs a command that technically executes but does not satisfy the user's intent
- recovers into a weaker workflow after a failure
- treats parse failure in critic/sufficiency as a soft success path
- recalls formula memories that should have been demoted after bad reuse

Those are all verification problems before they are orchestration problems.

## Work Items
- [ ] Extract `claim_check_once`, `guard_repair_semantics_once`, `check_execution_sufficiency_once`, and `preflight_command_once` from `src/intel.rs` into a dedicated `src/verification.rs`.
- [ ] Standardize verdict shapes across verification functions so parse/repair handling is consistent.
- [ ] Add `OutcomeVerification`:
  - function: `verify_outcome_match_intent`
  - inputs: user message, step purpose, program step metadata, observed shell/edit result
  - output: whether the outcome materially achieved the step purpose and user intent
- [ ] Define a strict parse-failure policy for nontrivial workflow verification:
  - verification parse failure must mark the turn as unclean
  - it must not silently collapse into an implicit `ok`
  - it must either trigger recovery, honest failure, or skip formula-memory save
- [ ] Add formula-memory demotion hooks driven by verification results:
  - failed reuse increments failure counts
  - repeated failure can disable a memory
  - successful reuse can strengthen memory priority
- [ ] Update `src/execution_steps.rs`, `src/orchestration.rs`, and `src/app_chat.rs` to depend on the new module.

## Acceptance Criteria
- Verification logic lives behind a coherent module boundary.
- Outcome verification can distinguish "command ran" from "task was actually satisfied."
- Parse failures in critic/sufficiency no longer degrade into quiet success for nontrivial workflows.
- Formula-memory save/demotion behavior is informed by verification results.
- Existing behavior remains intact where verification already works well.

## Verification
- `cargo build`
- `cargo test`
- rerun known failing scenarios, including:
  - selecting files and then showing the wrong ones
  - content requests that only list filenames
  - critic/sufficiency parse-failure paths
- confirm failed reuse can demote or skip formula memory
