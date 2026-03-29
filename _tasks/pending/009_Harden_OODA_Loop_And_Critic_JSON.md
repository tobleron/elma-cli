# 009_Harden_OODA_Loop_And_Critic_JSON

## Objective
Address the failure modes identified in the recent evaluation (`s_1774765593_812544000`): Fix the repair loop's failure to diversify strategy and resolve JSON parsing errors in the multi-critic system.

## Findings from Last Evaluation
- **Failure 1: Strategy Fixation**: During a `retry` (triggered by `outcome_verification`), Elma repeated the exact same command instead of rethinkng the approach.
- **Failure 2: Critic JSON Corruption**: `logical_review_parse_error` and `efficiency_review_parse_error` indicate that critics are producing invalid JSON or mixing it with thinking tokens.
- **Failure 3: Unclean Reasoning**: `memory_gate_status=skip reason=unclean_reasoning_fallback` shows that reasoning extraction still needs hardening.

## Technical Tasks
- [ ] **Adaptive Repair**: Update `src/orchestration_helpers.rs`'s `request_recovery_program` (or equivalent) to explicitly ingest the `reason` from the previous `outcome_verification` and forbidden previous failed steps.
- [ ] **JSON Extraction Hardening**: Update the utility in `src/text_utils.rs` (or equivalent) to more robustly isolate JSON blocks from surrounding reasoning/thought tokens.
- [ ] **Critic Prompting**: Refine the prompt templates used by the logic, risk, and efficiency critics to enforce a strict JSON schema and avoid conversational filler.
- [ ] **Reasoning Path Sanitization**: Ensure that `reasoning_audit.jsonl` tracks clean tokens only, separate from the content that must be parsed as JSON.

## Verification
- Run a task that is likely to fail (e.g., requesting a summary of a non-existent file).
- Verify that when the `outcome_verification` triggers a retry, Elma chooses a *different* command (like `ls` or `find`) instead of repeating the failed one.
- Confirm that the `trace_debug.log` shows zero `parse_error` for critics.
- Check `memory_gate_status` in the session trace to ensure it is no longer skipping due to "unclean reasoning."
