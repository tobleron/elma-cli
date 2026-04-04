# Task D002_815f1e: Surgical Refactor SRC BACKEND

## Objective
Senior Refactoring Engineer. Reduce estimated modification risk below the applicable drag target without fragmenting cohesive modules. Extract highlighted 'Hotspots' into sub-modules only when the resulting split stays within the preferred size policy. The file should remain a clear 'Orchestrator' or 'Service' boundary, with only truly dense or isolated logic moved to specialized siblings.


## Work Items
### 🔧 Action: De-bloat
**Directive:** Decompose & Flatten: Use guard clauses to reduce nesting and extract dense logic into private helper functions.
- [ ] **../../src/app_chat_builders_advanced.rs** (Metric: [Nesting: 1.80, Density: 0.02, Coupling: 0.01] | Drag: 2.82 | LOC: 576/450  ⚠️ Trigger: Oversized beyond the preferred 470-570 LOC working band.) → Refactor in-place (keep near ~520 LOC and above 260 LOC floor)
- [ ] **../../src/execution_steps.rs** (Metric: [Nesting: 1.80, Density: 0.02, Coupling: 0.01] | Drag: 2.97 | LOC: 738/450  ⚠️ Trigger: Oversized beyond the preferred 470-570 LOC working band.) → 🏗️ Split into 2 modules (target 470-570 LOC each, center ~520 LOC, floor 260 LOC)
- [ ] **../../src/json_tuning.rs** (Metric: [Nesting: 2.40, Density: 0.06, Coupling: 0.00] | Drag: 3.64 | LOC: 612/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.) → Refactor in-place (keep near ~560 LOC and above 260 LOC floor)
- [ ] **../../src/optimization_tune.rs** (Metric: [Nesting: 3.60, Density: 0.05, Coupling: 0.00] | Drag: 4.75 | LOC: 604/450  ⚠️ Trigger: Drag above target (2.60) with file already at 604 LOC.) → Refactor in-place (keep near ~560 LOC and above 260 LOC floor)
- [ ] **../../src/orchestration_planning.rs** (Metric: [Nesting: 1.80, Density: 0.04, Coupling: 0.01] | Drag: 2.92 | LOC: 652/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.) → Refactor in-place (keep near ~560 LOC and above 260 LOC floor)
- [ ] **../../src/routing_parse.rs** (Metric: [Nesting: 3.00, Density: 0.09, Coupling: 0.00] | Drag: 4.30 | LOC: 464/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 310-410 LOC working band if you extract helpers.) → Refactor in-place (keep near ~360 LOC and above 260 LOC floor)
- [ ] **../../src/verification.rs** (Metric: [Nesting: 3.00, Density: 0.04, Coupling: 0.00] | Drag: 4.28 | LOC: 500/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 390-490 LOC working band if you extract helpers.) → Refactor in-place (keep near ~440 LOC and above 260 LOC floor)

## 🔎 Programmatic Verification
- Baseline artifacts: `_dev-system/tmp/D002_815f1e_Surgical_Refactor_SRC_BACKEND/verification.json` (files at `_dev-system/tmp/D002_815f1e_Surgical_Refactor_SRC_BACKEND/files/`).
- Run `cargo run --manifest-path _dev-system/analyzer/Cargo.toml --bin spec_diff -- --baseline _dev-system/tmp/D002_815f1e_Surgical_Refactor_SRC_BACKEND/verification.json --targets <refactored files>` once the refactor is ready to ensure the function surface matches the captured snapshots.

### Pre-split snapshot for `src/app_chat_builders_advanced.rs`
- `src/app_chat_builders_advanced.rs` (1 functions, fingerprint 09a59d357d96e2665937c0766929d1351046de3bb0c4da686ba5ccd2f3a15f1a)
    - Grouped summary:
        - build_shell_path_probe_program × 1 (lines: 16)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/execution_steps.rs`
- `src/execution_steps.rs` (19 functions, fingerprint 2bfa534c3b8a0f15c663a46255cfeb6ea5ba0c97eb5cbd57c818996b6b91d6ae)
    - Grouped summary:
        - budget_evidence × 1 (lines: 90)
        - chat_once_get_text × 1 (lines: 199)
        - gather_artifacts × 1 (lines: 21)
        - handle_decide_step × 1 (lines: 423)
        - handle_master_plan_step × 1 (lines: 382)
        - handle_plan_step × 1 (lines: 340)
        - handle_program_step × 1 (lines: 483)
        - handle_reply_step × 1 (lines: 460)
        - handle_select_step × 1 (lines: 235)
        - handle_summarize_step × 1 (lines: 305)
        - is_relative_path_match × 1 (lines: 79)
        - mk_chat_req × 1 (lines: 175)
        - mk_step_result × 1 (lines: 214)
        - normalize_selected_items_against_evidence × 1 (lines: 34)
        - normalize_single_item × 1 (lines: 50)
        - select_items_via_unit × 1 (lines: 108)
        - selector_normalizes_unique_suffix_to_exact_relative_path × 1 (lines: 716)
        - selector_prefers_shallow_grounded_path_when_basename_is_ambiguous × 1 (lines: 728)
        - skip_selection × 1 (lines: 101)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/json_tuning.rs`
- `src/json_tuning.rs` (23 functions, fingerprint b1e8dc1570b6d835a020d126e567c3a3c4729c6010f10d527047275ef325d892)
    - Grouped summary:
        - apply_json_tuning_temperature × 1 (lines: 594)
        - build_chat_request × 1 (lines: 167)
        - build_tuning_report_content × 1 (lines: 548)
        - classify_json_response × 1 (lines: 181)
        - compute_scores × 1 (lines: 212)
        - default_temp_result × 1 (lines: 383)
        - emit_full_cached_progress × 1 (lines: 353)
        - emit_temp_result_progress × 1 (lines: 362)
        - find_latest_tuning_file × 1 (lines: 438)
        - find_optimal_temperature × 1 (lines: 395)
        - find_recommended_temperature × 1 (lines: 407)
        - from_str × 1 (lines: 24)
        - load_cached_json_tuning_result × 1 (lines: 424)
        - load_json_tuning_manifest × 1 (lines: 82)
        - parse_cached_result × 1 (lines: 466)
        - parse_temp_result_line × 1 (lines: 515)
        - process_temperature × 1 (lines: 237)
        - run_json_temperature_tuning × 1 (lines: 263)
        - save_json_tuning_report × 1 (lines: 578)
        - test_json_at_temperature × 1 (lines: 124)
        - test_scenario × 1 (lines: 90)
        - try_last_resort_repair × 1 (lines: 199)
        - try_load_cached_temp × 1 (lines: 345)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/optimization_tune.rs`
- `src/optimization_tune.rs` (8 functions, fingerprint fb0c659b4ccbf9f752929ccffec6847b0575ed7e771b33926493df94717f2995)
    - Grouped summary:
        - eval_stability × 1 (lines: 10)
        - json_score_for_temp × 1 (lines: 40)
        - new_search_candidate × 1 (lines: 49)
        - optimize_model × 1 (lines: 101)
        - orchestration × 1 (lines: 83)
        - response × 1 (lines: 91)
        - router × 1 (lines: 75)
        - update_beam_state × 1 (lines: 22)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/orchestration_planning.rs`
- `src/orchestration_planning.rs` (23 functions, fingerprint 4e0bba34c9c04e0fb416ce5790c2b735ad681e6d848486800dcc45664a9896e5)
    - Grouped summary:
        - alignment_for_level × 1 (lines: 206)
        - build_chat_fallback × 1 (lines: 137)
        - build_ladder_chat × 1 (lines: 163)
        - complexity_from_ladder × 1 (lines: 193)
        - derive_planning_prior × 1 (lines: 277)
        - derive_planning_prior_with_ladder × 1 (lines: 438)
        - fallback_formula_for_route × 1 (lines: 117)
        - get_required_depth × 1 (lines: 373)
        - is_empty_workflow_plan × 1 (lines: 221)
        - persist_masterplan × 1 (lines: 176)
        - planning_intel_context × 1 (lines: 14)
        - planning_prior_from_workflow_plan × 1 (lines: 238)
        - should_use_uncertain_reply_default × 1 (lines: 227)
        - test_route_decision × 1 (lines: 600)
        - trait_assess_complexity × 1 (lines: 51)
        - trait_build_scope × 1 (lines: 67)
        - trait_plan_workflow × 1 (lines: 32)
        - trait_select_formula × 1 (lines: 88)
        - try_hierarchical_decomposition × 1 (lines: 395)
        - try_hierarchical_decomposition_with_ladder × 1 (lines: 566)
        - uncertain_reply_default_allows_chat_like_turns × 1 (lines: 632)
        - uncertain_reply_default_rejects_path_scoped_plan_requests × 1 (lines: 647)
        - uncertain_reply_default_rejects_path_scoped_shell_requests × 1 (lines: 640)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/routing_parse.rs`
- `src/routing_parse.rs` (25 functions, fingerprint 8e17704acbe0d42bab21ffe00c07cf663fc3cedbdfbf59d8718180fb58e73493)
    - Grouped summary:
        - collect_json_string_value × 1 (lines: 80)
        - collect_literal_value × 1 (lines: 115)
        - collect_nested_structure × 1 (lines: 99)
        - detect_repetition_loop × 1 (lines: 193)
        - extract_first_json_object × 1 (lines: 20)
        - extract_json_from_markdown_wrapped × 1 (lines: 339)
        - extract_json_from_pure_json × 1 (lines: 361)
        - extract_json_with_prose_after × 1 (lines: 369)
        - fix_orphaned_keys_in_arrays × 1 (lines: 163)
        - is_known_step_field × 1 (lines: 62)
        - is_orphaned_key_boundary × 1 (lines: 76)
        - parse_json_loose × 1 (lines: 285)
        - strip_markdown_wrappers × 1 (lines: 7)
        - strip_markdown_wrappers_handles_no_fences × 1 (lines: 318)
        - strip_markdown_wrappers_handles_prose_before_fence × 1 (lines: 325)
        - strip_markdown_wrappers_removes_code_fences × 1 (lines: 305)
        - test_detect_repetition_loop × 1 (lines: 415)
        - test_fix_orphaned_keys_in_steps × 1 (lines: 440)
        - test_normal_json_not_flagged_as_repetitive × 1 (lines: 429)
        - test_parse_error_preview_handles_unicode × 1 (lines: 457)
        - test_repair_malformed_llm_json × 1 (lines: 394)
        - try_absorb_orphaned_key × 1 (lines: 126)
        - try_parse_extracted_json × 1 (lines: 244)
        - try_repair_and_parse × 1 (lines: 267)
        - validate_no_repetition_loop × 1 (lines: 228)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/verification.rs`
- `src/verification.rs` (19 functions, fingerprint 9c907aa732fc9c7b9e572cab506c7e437569266dcef4c448d512b8010ace586e)
    - Grouped summary:
        - apply_verdict_to_result × 1 (lines: 377)
        - chat_and_parse × 1 (lines: 35)
        - check_execution_sufficiency_once × 1 (lines: 82)
        - claim_check_once × 1 (lines: 44)
        - gate_formula_memory_once × 1 (lines: 454)
        - ground_outcome_reason_if_needed × 1 (lines: 412)
        - guard_repair_semantics_once × 1 (lines: 62)
        - handle_schema_error × 1 (lines: 245)
        - handle_verify_error × 1 (lines: 289)
        - mark_result_ok × 1 (lines: 197)
        - mk_intel_req × 1 (lines: 11)
        - outcome_verifier_configs × 1 (lines: 103)
        - parse_verdict_from_json × 1 (lines: 189)
        - preflight_command_once × 1 (lines: 476)
        - truncate_output × 1 (lines: 99)
        - try_apply_downstream_validation × 1 (lines: 206)
        - try_skip_intermediate_evidence_step × 1 (lines: 227)
        - verify_nontrivial_step_outcomes × 1 (lines: 309)
        - verify_outcome_match_intent × 1 (lines: 114)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
