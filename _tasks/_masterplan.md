# Master Plan

Last updated: 2026-05-07 (round 8 prompt testing complete; tasks 724-728 opened)

## Phase 1: COMPLETED ✅

All 8 Phase 1 tasks (635-642) are done:
- `src/ui_runtime_event.rs` — canonical UiRuntimeEvent enum with 30+ event variants, tests
- `src/ui_view_state.rs` — pure UiViewState struct (renderer-only state)
- `src/ui_reducer.rs` — pure reducer function mapping events to view state, 11 tests
- `src/session_persistence_adapter.rs` — extract session persistence from ClaudeRenderer
- `src/input_controller.rs` — separated input buffer, cursor, history, picker, command parsing, 12 tests
- `src/reasoning_visibility.rs` — ReasoningVisibilityPolicy, normalized thinking display, 16 tests
- `src/footer_contract.rs` — FooterContract validation (model/tokens/elapsed only), 17 tests
- `src/ui_snapshot.rs` — deterministic UI regression capture harness, 6 fixtures
- Phase 1 635-642 archived in `_tasks/completed/` as `_DONE.md`

## Phase 2-8: IN PROGRESS

Core modules created for:
- 643: `src/model_capability_probe.rs` — ModelCapabilityProbe + ProviderResponseAdapter, 18 tests
- 649: `src/complexity_gate.rs` — ComplexityGate heuristic assessment, 16 tests
- 653: `src/budget_forecaster.rs` — BudgetForecaster + BudgetEnvelope, 18 tests
- 654: `src/tool_degradation.rs` — ToolDegradationPlanner + RetryPlan, 10 tests
- 655: `src/tool_trait.rs` — unified Tool trait + ToolExecutor, 14 tests
- 657: `src/event_ledger.rs` — ToolExecutionEvent ledger + RawPayloadStore, 10 tests
- 666: `src/session_forensics.rs` — ForensicReport + SessionForensics + TraceReducer, 18 tests
- 668: `src/code_index.rs` — CodeIndex with Rust symbol extraction, 19 tests

Remaining: integrate these modules into the runtime, complete remaining task files.

This is the execution guide for the active pending queue. Use it to pick work in dependency order and to avoid touching the same source surfaces out of sequence. Each task file remains the implementation detail, acceptance criteria, and verification contract.

## Immediate Round 8 Stabilization Queue

These tasks were created from direct prompt testing against `_testing_prompts/01_prompt.txt` through `_testing_prompts/08_prompt.txt` after tasks 720-723 were implemented. Do these before dense-model troubleshooting because they are still visible with the thinking model and will be worse on dense coder models.

Latest follow-up from round 8 validation:

1. `724_Tool_Argument_Schema_And_Path_Recovery_Coherence.md`
2. `726_Workspace_Discovery_Must_Exclude_Generated_And_Vendor_Trees_By_Default.md`
3. `725_Substantive_Artifact_Synthesis_Must_Not_Fall_Back_To_Raw_Evidence_Dumps.md`
4. `727_Final_Answer_And_Artifact_Manifest_Must_Use_The_Actual_Delivered_Path.md`
5. `728_Shell_Idle_Timeout_Recovery_Should_Split_Compound_Verification_Commands.md`
6. `729_Noninteractive_Piped_Input_Should_Exit_After_Final_Response.md`

Recommended order:

1. Task 724 first, because malformed `read`/`exists` calls are the main upstream cause of stagnation and weak report evidence.
2. Task 726 second, because default discovery is still polluted by `.kilo`, `.trash`, `_knowledge_base`, backups, and generated sessions; clean evidence improves every later task.
3. Task 725 third, because reports must become substantive artifacts instead of raw evidence dumps once tool evidence is cleaner.
4. Task 727 fourth, because final answers and manifests must refer to the exact delivered artifacts after synthesis is fixed.
5. Task 728 fifth, because shell timeout recovery is important but narrower than the tool/schema/artifact path.
6. Task 729 should be fixed before relying on automated suites in CI, because piped one-shot prompts can currently hang after producing the final answer.

Round 8 observed progress:

- Prompts 02-08 completed within the prompt-suite timeout after the snapshot storage blocker was patched.
- Prompt 04 no longer triggers recursive session/archive growth; session size stayed small and disk remained stable.
- Endpoint probing still traces detected model type and context window:
  `kind=thinking thinking=true json_mode=true ctx_max=262144`.
- The helper model remains disabled cleanly; traces show `auxiliary_helper_disabled`.
- Backup source scoping is now much safer: prompt 08 copied `src` only and verified 322 files.

Remaining blocker:

- Report-writing prompts still often end as partial recovered evidence dumps after empty `read` or duplicate discovery loops. This is the next major reliability barrier.
- Default discovery still includes generated/vendor/reference trees, which causes the model to choose poor evidence paths.
- Finalization can drift from the actual written artifact path to a stale inferred `project_tmp/report.md`.
- Piped stdin smoke testing can render the final answer and then keep the process alive instead of exiting.

Direct fixes applied during round 8 validation:

- `src/snapshot.rs` now excludes generated/archive-heavy trees from snapshots and caps snapshot file count/size.
- `src/tool_calling.rs` now canonicalizes `exists.paths[0]` to `path`, supports multi-path `exists`, and returns successful structured status for successful `exists`.

Do not recreate completed tasks 720-723 unless a new task cites new round 8 evidence. Tasks 724-728 are regressions or remaining gaps confirmed after those tasks were completed.

Completed stabilization tasks from the prior cycle should remain archived; do not recreate them unless a new regression is confirmed.

1. `701_Minimal_Turn_Context_Narrative_And_Prompt_Packet_For_Dense_Models.md`
2. `703_Tool_Call_Schema_Recovery_For_Empty_Read_And_Bogus_Copy_Repairs.md`
3. `704_Finalization_Evidence_Gate_And_Artifact_Persistence_For_Dense_Models.md`
4. `705_Safe_Backup_Copy_Tool_And_Verification_Workflow.md`
5. `702_Document_Data_Retrieval_Mode_And_Source_Code_Adapter_Router.md`
6. `706_No_Color_Debug_Transcript_Regression_Reopen.md`
7. `707_Auxiliary_Helper_Disabled_UI_And_Trace_Contract.md`

Execution notes:
- Tasks 701, 703, and 704 all touch `src/tool_loop.rs`; implement them in order to avoid conflicting changes.
- Task 703 should land before Task 705 so backup/copy failures do not keep receiving bogus repaired paths.
- Task 702 is a major offline intelligence feature and should stay above network/remote-channel work.
- Task 706 is a reopened observability regression even though Task 700 is completed.
- Do not modify `src/prompt_core.rs`; solve dense-model issues through adapters, context packets, tool contracts, and finalization structure.

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Do not mark a task complete until its own verification section passes.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly records user approval for that change.
- Current active architecture is strict JSON/tool-calling plus compact intel-unit JSON. Do not revive DSL action protocols.
- Prefer rust-native, local-first, offline tools and tests over network-dependent integrations.
- Surface routing, tool discovery, retries, compaction, stop reasons, budget decisions, and decomposition as transcript rows.
- Failed approaches fork sibling approaches from the same objective; do not continue down a failing branch.
- When a task touches UI internals, preserve the footer contract: model name, token count, elapsed time only.

## Phase 1: UI Runtime Architecture And Visibility

Do these first because later session, trace, and tool work need a clean UI/event boundary.

1. `635_UI_Runtime_Event_Reducer_And_Service_Boundaries.md`
2. `636_UI_Transcript_Virtualization_Render_Cache_And_Per_Frame_Budget.md`
3. `637_Prompt_Input_Controller_And_Command_Mode_Boundaries.md`
4. `638_Tool_Thinking_Notice_State_Machines_For_UI.md`
5. `639_Terminal_UI_Regression_Capture_Harness.md`
6. `640_UI_Renderer_Ownership_And_Legacy_Deprecation_Decision.md`
7. `641_Status_Bar_Contract_And_Transcript_Native_Operational_Visibility.md`
8. `642_Reasoning_Visibility_Redaction_And_Thinking_Model_UI_Contract.md`

Execution notes:
- Do not remove renderer modules before Task 639 has snapshot coverage.
- Keep UI changes architecture-focused; visual polish belongs after these tasks, not before.
- Tasks 635, 638, 641, and 667 should agree on event names and notice metadata.

## Phase 2: Model And Strict JSON Robustness

These tasks make Elma work reliably with modern thinking models and dense coder models.

1. `643_Model_Capability_Probe_And_Provider_Response_Adapter.md`
2. `644_Dense_Coder_Model_Output_Sanitizer_And_Finalization_Guards.md`
3. `645_Unified_Strict_Tool_Argument_Parsing_And_Model_Facing_Error_Contract.md`
4. `646_Provider_Fault_Injection_And_Stream_Error_Recovery_Harness.md`
5. `647_Model_Profile_Tuning_Lifecycle_And_Reliability_Matrix.md`
6. `648_Strict_JSON_Documentation_And_Task_Drift_Cleanup.md`

Execution notes:
- Keep prompt changes out of `src/prompt_core.rs`; use adapters, schemas, fixtures, and focused intel units first.
- Provider behavior must be proven by fixtures or probes before it influences runtime requests.
- Task 648 should remove active DSL drift from docs and `_tasks/_fix.md`.

## Phase 3: Complexity, Work Graph, Budget, And Retry Semantics

These tasks protect semantic continuity and prevent wasted loops.

1. `649_Model_Inferred_Complexity_Gate_And_Route_Continuity.md`
2. `650_Runtime_Keyword_Gate_Audit_And_Analyzer_Rule.md`
3. `651_Work_Graph_Task_Persistence_And_Full_Hierarchy_Integration.md`
4. `652_Approach_Branch_Rehydration_And_Failure_Taxonomy.md`
5. `653_Budget_Forecasting_Dynamic_Iteration_And_Context_Envelopes.md`
6. `654_Tool_Set_Degradation_And_Retry_Planner.md`

Execution notes:
- Complexity assessment gates graph depth before work begins.
- Do not replace keyword gates with larger prompts; use narrower strict JSON units or typed deterministic state.
- Runtime task persistence must link graph node, approach id, evidence, and session artifacts.

## Phase 4: Tooling, Shell Safety, Transactions, And Workspace Policy

These tasks touch the same execution surface and should be sequenced carefully.

1. `655_Unified_Tool_Trait_Migration_And_Executor_Parity.md`
2. `656_Tool_Metadata_Policy_And_Discoverable_Workspace_Info.md`
3. `657_Tool_Execution_Event_Ledger_With_Raw_Payload_References.md`
4. `658_Parser_Backed_Shell_Exec_Policy_And_Permission_Cache.md`
5. `659_Process_Group_Cleanup_And_Background_Job_Runtime.md`
6. `660_Snapshot_Coverage_For_Shell_Mutations_And_Rollback_Integrity.md`
7. `661_Transactional_Patch_And_Durable_JSON_Write_Layer.md`
8. `662_Workspace_Policy_Relative_Path_And_Symlink_Hardening.md`

Execution notes:
- Task 655 should land before broad tool behavior changes so declarations and executors stop drifting.
- Task 658 must distinguish command syntax parsing from forbidden user-intent keyword matching.
- Task 661 should become the persistence primitive used by session/task/config writes.

## Phase 5: Sessions, Forensics, And Replay

These tasks make the session folder the authority for what happened.

1. `663_Session_Store_Typed_Message_Parts_And_Runtime_State_Ownership.md`
2. `664_Session_Rewind_Checkpoint_Restore_And_Compaction_Recovery.md`
3. `665_Diagnostics_Bundle_And_Doctor_Command.md`
4. `666_Session_Forensics_Runner_From_Fix_Task.md`
5. `667_Replayable_Trace_Reducer_And_Raw_Payload_Bundle.md`

Execution notes:
- Prefer one canonical store plus generated projections over duplicate mutable state.
- Session forensics must create tasks only for evidence-backed problems.
- Trace reducer events should align with UI events from Phase 1.

## Phase 6: Offline Intelligence And Local Tools

These improve offline Elma capability after core reliability is stable.

1. `668_Persistent_Offline_Document_Code_Index_With_Citations.md`
2. `669_Local_Project_Memory_With_Security_Scanning.md`
3. `670_Offline_LSP_Diagnostics_And_Code_Intelligence_Tool.md`
4. `671_Offline_Data_Analysis_Mode_With_Bounded_Local_Execution.md`
5. `672_Search_Result_Analysis_Intel_Unit_And_Evidence_Ranking.md`

Execution notes:
- Retrieval and memory must be evidence-backed and workspace-scoped.
- Local tools should degrade gracefully when optional language servers or interpreters are missing.
- Never require internet for these tasks.

## Phase 7: Enterprise Gates

These tasks verify portability, dependency hygiene, and release readiness.

1. `673_Cross_Platform_Portability_Gate.md`
2. `674_Cargo_Dependency_Feature_Hygiene_And_Supply_Risk_Audit.md`
3. `675_Auto_Lint_Test_And_Verification_Planner.md`
4. `676_JSON_Tool_Calling_Certification_Suites_Current_Architecture.md`
5. `677_Release_Risk_Security_Audit_Gate.md`
6. `678_Dead_Code_Deprecation_And_Large_Module_Debloating_Audit.md`

Execution notes:
- Certification suites must run against current strict JSON/tool-calling behavior.
- De-bloating must be evidence-led; avoid broad refactors that do not improve reliability or maintainability.

## Phase 8: Low-Priority Optional Backlog

These are valuable but should remain below offline core architecture, stability, and UI internals.

1. `679_Headless_Event_API_And_SDK_Harness.md`
2. `680_Extension_State_MCP_And_Optional_Capability_Gateway_Offline_Gates.md`
3. `681_Bounded_Local_Subagent_Delegation_Framework.md`
4. `682_File_Watcher_AI_Comment_And_Autosave_Workflow_Low_Priority.md`
5. `683_Network_Fetch_Download_Browser_And_Offline_Search_Policy_Low_Priority.md`
6. `684_Remote_Daemon_Channel_And_Notification_Integrations_Low_Priority.md`
7. `685_Experimental_Reasoning_Tuning_And_Creative_Recovery_Backlog_Low_Priority.md`
8. `686_Extended_Ebook_And_Legacy_Document_Adapters_Low_Priority.md`

Execution notes:
- Network and remote-channel work is opt-in and disabled by default.
- Experimental reasoning work must not mutate core prompts or add broad example-heavy prompts.
- Optional integrations must surface state and decisions in the transcript.
