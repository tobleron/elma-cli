# Task D002_17d66a: Surgical Refactor SRC BACKEND

## Objective
Senior Refactoring Engineer. Reduce estimated modification risk below the applicable drag target without fragmenting cohesive modules. Extract highlighted 'Hotspots' into sub-modules only when the resulting split stays within the preferred size policy. The file should remain a clear 'Orchestrator' or 'Service' boundary, with only truly dense or isolated logic moved to specialized siblings.


## Work Items
### 🔧 Action: De-bloat
**Directive:** Decompose & Flatten: Use guard clauses to reduce nesting and extract dense logic into private helper functions.
- [ ] **../../src/app_bootstrap.rs** (Metric: [Nesting: 1.80, Density: 0.07, Coupling: 0.00] | Drag: 2.95 | LOC: 674/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.) → 🏗️ Split into 2 modules (target 350-450 LOC each, center ~400 LOC, floor 220 LOC)
- [ ] **../../src/execution_steps_shell.rs** (Metric: [Nesting: 5.40, Density: 0.07, Coupling: 0.00] | Drag: 6.76 | LOC: 644/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.) → 🏗️ Split into 2 modules (target 350-450 LOC each, center ~400 LOC, floor 220 LOC)
- [ ] **../../src/optimization_tune.rs** (Metric: [Nesting: 3.00, Density: 0.06, Coupling: 0.00] | Drag: 4.26 | LOC: 484/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
- [ ] **../../src/orchestration_loop.rs** (Metric: [Nesting: 3.00, Density: 0.04, Coupling: 0.00] | Drag: 4.25 | LOC: 683/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.) → 🏗️ Split into 2 modules (target 350-450 LOC each, center ~400 LOC, floor 220 LOC)
- [ ] **../../src/tool_discovery.rs** (Metric: [Nesting: 3.00, Density: 0.05, Coupling: 0.01] | Drag: 4.19 | LOC: 404/400  ⚠️ Trigger: Drag above target (2.60) with file already at 404 LOC.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
- [ ] **../../src/tune_scenario.rs** (Metric: [Nesting: 3.60, Density: 0.04, Coupling: 0.00] | Drag: 5.25 | LOC: 531/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.) → Refactor in-place (keep near ~400 LOC and above 220 LOC floor)
### 🔧 Action: De-bloat
**Directive:** Right-size Surface: Keep the module as the orchestration boundary and extract only adjacent sections that reduce file length without fragmenting the public API.
- [ ] **../../src/types_core.rs** (Metric: [Nesting: 1.20, Density: 0.01, Coupling: 0.00] | Drag: 2.26 | LOC: 620/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.) → 🏗️ Split into 2 modules (target 350-450 LOC each, center ~400 LOC, floor 220 LOC) [Size-only candidate; drag already within target.]

## 🔎 Programmatic Verification
- Baseline artifacts: `_dev-system/tmp/D002_17d66a_Surgical_Refactor_SRC_BACKEND/verification.json` (files at `_dev-system/tmp/D002_17d66a_Surgical_Refactor_SRC_BACKEND/files/`).
- Run `cargo run --manifest-path _dev-system/analyzer/Cargo.toml --bin spec_diff -- --baseline _dev-system/tmp/D002_17d66a_Surgical_Refactor_SRC_BACKEND/verification.json --targets <refactored files>` once the refactor is ready to ensure the function surface matches the captured snapshots.

### Pre-split snapshot for `src/app_bootstrap.rs`
- `src/app_bootstrap.rs` (12 functions, fingerprint 7e3a2229f431ae8979e1438399f55e59c6276fa06bd077656a0a93d00b555c5e)
    - Grouped summary:
        - apply_prompt_upgrades × 1 (lines: 425)
        - bootstrap_app × 1 (lines: 8)
        - build_system_content × 1 (lines: 622)
        - emit_auto_tune_banner × 1 (lines: 176)
        - emit_startup_banner × 1 (lines: 635)
        - handle_special_modes × 1 (lines: 191)
        - load_profiles × 1 (lines: 286)
        - persist_workspace_intel × 1 (lines: 601)
        - prepare_session × 1 (lines: 590)
        - should_auto_tune_on_startup × 1 (lines: 169)
        - sync_and_upgrade_profiles × 1 (lines: 335)
        - validate_mode_flags × 1 (lines: 156)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/execution_steps_shell.rs`
- `src/execution_steps_shell.rs` (1 functions, fingerprint 7a17ae1e22b0027280b95da251f69b2ae61ea35a445756c46cfff4fe0bcddbd0)
    - Grouped summary:
        - handle_shell_step × 1 (lines: 9)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/optimization_tune.rs`
- `src/optimization_tune.rs` (1 functions, fingerprint 002c501ec2d68f43f017ef1f5ce5aa27e3d18cc2874f32f347ddb09436c7c1c4)
    - Grouped summary:
        - optimize_model × 1 (lines: 9)
    - Detailed entries are preserved in baseline JSON (`verification.json`) for machine-level diffs.
### Pre-split snapshot for `src/orchestration_loop.rs`
- `src/orchestration_loop.rs` (6 functions, fingerprint 414b0b152d05de4e8933600602e97f852b1eeb6dea644cc1312da42d2c16815c)
    - Grouped summary:
        - merged_program_from_history × 1 (lines: 14)
        - next_program_is_stale × 1 (lines: 25)
        - program_has_shell_or_edit × 1 (lines: 29)
        - run_autonomous_loop × 1 (lines: 159)
        - run_staged_reviewers_once × 1 (lines: 43)
        - step_results_have_shell_or_edit × 1 (lines: 33)
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
### Pre-split snapshot for `src/tune_scenario.rs`
- `src/tune_scenario.rs` (1 functions, fingerprint fb88e36d8f22e9296c1f3fd6249d337f4febf48e7c32420b792d618093ff2ec0)
    - Grouped summary:
        - evaluate_runtime_scenario × 1 (lines: 8)
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
