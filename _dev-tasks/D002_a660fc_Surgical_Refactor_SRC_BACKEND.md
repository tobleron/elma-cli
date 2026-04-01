# Task D002_a660fc: Surgical Refactor SRC BACKEND

## Objective
Senior Refactoring Engineer. Reduce estimated modification risk below the applicable drag target without fragmenting cohesive modules. Extract highlighted 'Hotspots' into sub-modules only when the resulting split stays within the preferred size policy. The file should remain a clear 'Orchestrator' or 'Service' boundary, with only truly dense or isolated logic moved to specialized siblings.


## Work Items
### 🔧 Action: De-bloat
**Directive:** Decompose & Flatten: Use guard clauses to reduce nesting and extract dense logic into private helper functions.
- [ ] **../../src/execution_steps_shell_exec.rs** (Metric: [Nesting: 3.00, Density: 0.06, Coupling: 0.00] | Drag: 4.28 | LOC: 413/400  ⚠️ Trigger: Drag above target (2.60) with file already at 413 LOC.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
- [ ] **../../src/intel.rs** (Metric: [Nesting: 1.80, Density: 0.00, Coupling: 0.00] | Drag: 2.80 | LOC: 757/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.) → 🏗️ Split into 2 modules (target 350-450 LOC each, center ~400 LOC, floor 220 LOC)
- [ ] **../../src/optimization_tune.rs** (Metric: [Nesting: 3.00, Density: 0.06, Coupling: 0.00] | Drag: 4.26 | LOC: 484/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
- [ ] **../../src/orchestration_loop.rs** (Metric: [Nesting: 3.00, Density: 0.05, Coupling: 0.01] | Drag: 4.27 | LOC: 533/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
- [ ] **../../src/tool_discovery.rs** (Metric: [Nesting: 3.00, Density: 0.05, Coupling: 0.01] | Drag: 4.19 | LOC: 404/400  ⚠️ Trigger: Drag above target (2.60) with file already at 404 LOC.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
### 🔧 Action: De-bloat
**Directive:** Right-size Surface: Keep the module as the orchestration boundary and extract only adjacent sections that reduce file length without fragmenting the public API.
- [ ] **../../src/types_core.rs** (Metric: [Nesting: 1.20, Density: 0.01, Coupling: 0.00] | Drag: 2.26 | LOC: 620/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.) → 🏗️ Split into 2 modules (target 350-450 LOC each, center ~400 LOC, floor 220 LOC) [Size-only candidate; drag already within target.]

## 🔎 Programmatic Verification
- Baseline artifacts: `_dev-system/tmp/D002_a660fc_Surgical_Refactor_SRC_BACKEND/verification.json` (files at `_dev-system/tmp/D002_a660fc_Surgical_Refactor_SRC_BACKEND/files/`).
- Run `cargo run --manifest-path _dev-system/analyzer/Cargo.toml --bin spec_diff -- --baseline _dev-system/tmp/D002_a660fc_Surgical_Refactor_SRC_BACKEND/verification.json --targets <refactored files>` once the refactor is ready to ensure the function surface matches the captured snapshots.

### Pre-split snapshot for `src/execution_steps_shell_exec.rs`
- `src/execution_steps_shell_exec.rs` (4 functions, fingerprint d0dca4babaa0f7b137fe44de0f579a61db38753d0b8110727bf720951fdc1df4)
    - Grouped summary:
        - execute_and_process_shell × 1 (lines: 10)
        - handle_artifact_recording × 1 (lines: 381)
        - handle_command_unavailable × 1 (lines: 191)
        - try_command_repair × 1 (lines: 256)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/intel.rs`
- `src/intel.rs` (14 functions, fingerprint 9b600019dfecf539d9679b185c11dcd884899c002ea8afd4f18b5cd02faa9f12)
    - Grouped summary:
        - assess_action_needs_once × 1 (lines: 118)
        - assess_complexity_once × 1 (lines: 36)
        - assess_evidence_needs_once × 1 (lines: 79)
        - build_scope_once × 1 (lines: 190)
        - classify_artifacts_once × 1 (lines: 636)
        - compact_evidence_once × 1 (lines: 596)
        - decide_evidence_mode_once × 1 (lines: 519)
        - generate_status_message_once × 1 (lines: 3)
        - plan_workflow_once × 1 (lines: 367)
        - present_result_once × 1 (lines: 672)
        - repair_command_once × 1 (lines: 721)
        - select_formula_once × 1 (lines: 274)
        - select_items_once × 1 (lines: 481)
        - suggest_pattern_once × 1 (lines: 157)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/optimization_tune.rs`
- `src/optimization_tune.rs` (1 functions, fingerprint 002c501ec2d68f43f017ef1f5ce5aa27e3d18cc2874f32f347ddb09436c7c1c4)
    - Grouped summary:
        - optimize_model × 1 (lines: 9)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/orchestration_loop.rs`
- `src/orchestration_loop.rs` (1 functions, fingerprint c0fcae2b51d6d9496a866a29b0491130538cd746c7819d0fcc5c7b7b02bbab30)
    - Grouped summary:
        - run_autonomous_loop × 1 (lines: 14)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/tool_discovery.rs`
- `src/tool_discovery.rs` (14 functions, fingerprint 53bda970fb3c0da544315e9ef27b3debedafe199da854ed78774f50884fcdc86)
    - Grouped summary:
        - add_tool × 1 (lines: 79)
        - command_exists × 1 (lines: 359)
        - discover_makefile_targets × 1 (lines: 242)
        - discover_npm_scripts × 1 (lines: 278)
        - discover_scripts × 1 (lines: 185)
        - discover_workspace_tools × 1 (lines: 164)
        - format_for_display × 1 (lines: 95)
        - get_tool × 1 (lines: 90)
        - needs_discovery × 1 (lines: 74)
        - new × 1 (lines: 65)
        - test_add_tool × 1 (lines: 382)
        - test_command_exists × 1 (lines: 397)
        - test_registry_needs_discovery × 1 (lines: 372)
        - verify_project_tools × 1 (lines: 303)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/types_core.rs`
- `src/types_core.rs` (10 functions, fingerprint 626268e283e695c05e9c1db97d07f763f3af7adc0b50b9021d4f8be8a93fd116)
    - Grouped summary:
        - add_pending_subgoal × 1 (lines: 53)
        - clear × 1 (lines: 65)
        - complete_subgoal × 1 (lines: 42)
        - default_runtime_safe × 1 (lines: 295)
        - from × 1 (lines: 513)
        - has_active_goal × 1 (lines: 72)
        - id × 1 (lines: 592)
        - kind × 1 (lines: 600)
        - new × 1 (lines: 27)
        - purpose × 1 (lines: 613)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
