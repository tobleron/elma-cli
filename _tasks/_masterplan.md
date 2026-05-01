# Master Plan

Last updated: 2026-05-01 (expanded for source-agent tool parity, rust-first tools, pyramid orchestration, and plain-text terminal output)

This is the execution index for all current pending tasks. Use it to choose work in dependency order, not as a replacement for each task file. Each task file remains the implementation detail, verification commands, and done criteria.

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Do not mark a task complete until its own verification section passes.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly has user approval for that change.
- Prefer rust-native/offline tools over shell and network tools.
- Keep intel-unit JSON simple: one nested object level maximum, three required fields by default, five total fields absolute maximum.
- Surface routing, tool discovery, retries, compaction, stop reasons, and decomposition as transcript rows.
- Failed approaches do not continue down the objective hierarchy; retry with a new approach toward the same original objective.

## Wave 1: Highest-Gain Reliability And Tool Awareness

These tasks remove current wrong-answer causes and give Elma a reliable view of its tool arsenal.

### 386 Source-Agent Tool Parity Gap Matrix
- [x] Inventory all tool families under `_knowledge_base/_source_code_agents/`.
- [x] Map each source-agent tool family to an Elma equivalent or pending task.
- [x] Mark preferred implementation mode: rust-native, shell fallback, network optional, or extension.
- File: `_tasks/completed/386_Source_Agent_Tool_Parity_Gap_Matrix_DONE.md`

### 376 Replace Length Heuristic With LLM Route Inference
- [x] Remove short-prompt tool suppression (replaced `line.len() < 30` with `annotate_and_classify`).
- [x] Route evidence needs through model confidence and later tool discovery.
- File: `_tasks/completed/376_Replace_Length_Heuristic_With_LLM_Route_Inference_DONE.md`

### 377 Remove Trivial Chat Bypass In Orchestration
- [x] Ensure all turns can reach retry, repair, and tool discovery.
- [x] Stop direct hallucinated `reply_only` answers from bypassing orchestration.
- File: `_tasks/completed/377_Remove_Trivial_Chat_Bypass_In_Orchestration_DONE.md`

### 378 JSON Complexity Constraint And Repair
- [x] Add `validate_schema_complexity()` with tests for all constraint rules.
- [x] Create `src/json_repair.rs` with 4-stage deterministic repair pipeline.
- [x] Wire new module into crate; 700 tests pass.
- [ ] Split complex schemas (workflow, complexity, scope) into focused units.
- File: `_tasks/completed/378_JSON_Complexity_Constraint_And_Repair_DONE.md`

### 387 Rust-Native Tool Preference And Shell Fallback Policy
- [x] Add tool metadata: `ImplementationKind` enum, `shell_equivalents`, `workspace_scoped`.
- [x] Prefer native tools before shell for equivalent operations.
- [x] 7 new tests prove metadata correctness (priority ranking, offline capability, equivalents).
- File: `_tasks/completed/387_Rust_Native_Tool_Preference_And_Shell_Fallback_Policy_DONE.md`

### 388 Model-Driven Tool Discovery And Capability Routing
- [x] Create `CapabilityDiscoveryUnit` intel unit with 3-field JSON output (Task 378 compliant).
- [x] Add `find_tools_for_capability()` with rust-native ranking via `ImplementationKind::selection_priority()`.
- [x] Add `auto_discover_tools()` that discovers capability, searches registry, caps at 5, loads results.
- [x] 7 tests prove discovery, ranking, rust-native preference, and cap behavior.
- File: `_tasks/completed/388_Model_Driven_Tool_Discovery_And_Capability_Routing_DONE.md`

## Wave 2: Pyramid Orchestration And Semantic Reliability

These tasks make complex requests manageable for small models by decomposing and repairing the smallest failing unit.

### 389 Pyramid Work Graph Complexity Assessment
- [x] `WorkGraph` + `WorkGraphBuilder` types with 5 node kinds and approach tracking.
- [x] `GraphComplexityUnit` intel unit (3-field JSON compliant with Task 378).
- [x] 13 tests for graph construction, depth, children, kind filtering, topological order.
- File: `_tasks/completed/389_Pyramid_Work_Graph_Complexity_Assessment_DONE.md`

### 379 Dynamic Decomposition On Weakness
- [x] `FailureClass` enum with 7 variants and detection from error context.
- [x] `decompose_on_failure()` now returns true for stale/parse/multi-failure signals.
- [x] `strategy_for_failure()` maps each class to decomposition guidance.
- [x] Wired into retry loop: failure class detection on every retry after first.
- [x] 18 tests (11 new) for detection, labeling, strategies, and decomposition gating.
- File: `_tasks/completed/379_Dynamic_Decomposition_On_Weakness_DONE.md`

### 390 Approach Branch Retry And Prune Engine
- [x] `ApproachEngine` with `ApproachAttempt` state and `ApproachDecision` enum.
- [x] Pruning decision logic driven by FailureClass severity and configurable thresholds.
- [x] New-approach generator with strategy hint from `strategy_for_failure_by_label()`.
- [x] Wired into `orchestrate_with_retries()` — exhaust/continue/prune decisions per attempt.
- [x] 9 tests covering creation, continue, prune, exhaustion by attempts/approaches, graph state.
- File: `_tasks/completed/390_Approach_Branch_Retry_And_Prune_Engine_DONE.md`

### 391 Instruction-Level Repair And Result Recombiner
- [x] `InstructionOutcome`, `InstructionStatus`, `RepairAction` (3-field, Task 378 compliant).
- [x] `select_repair_action()` — non-keyword mapping: tool→native, parse→tighten, missing→evidence, timeout→split.
- [x] `try_repair()` / `create_repair_outcome()` — produce Running or Abandoned outcomes.
- [x] `recombine()` — merge successful sibling outcomes, fail-closed on missing evidence.
- [x] Wired into `orchestrate_with_retries()` — repair events logged per failed step.
- [x] 18 tests covering all repair actions, abandon logic, recombination, evidence gating.
- File: `_tasks/completed/391_Instruction_Level_Repair_And_Result_Recombiner_DONE.md`

### 380 Semantic Continuity Tracking
- Preserve original intent through routing, graph decomposition, execution, and finalization.
- Retry or block when output drifts from the original objective.
- File: `_tasks/pending/380_Semantic_Continuity_Tracking.md`

### 381 Transcript-Native Operational Visibility
- Show routing, formula, tool discovery, stop, retry, compaction, and decomposition events.
- Keep footer limited to model, token count, elapsed time.
- File: `_tasks/pending/381_Transcript_Operational_Visibility.md`

### 384 Clean-Context Finalization Enforcement
- Strip internal framing, stop reasons, and tool-loop artifacts.
- Keep final terminal answers direct and plain text.
- File: `_tasks/pending/384_Clean_Context_Finalization_Enforcement.md`

### 392 Plaintext Default And Markdown Output Tool
- Make ratatui output plain text by default.
- Provide markdown only as an explicit artifact/report path.
- File: `_tasks/pending/392_Plaintext_Default_And_Markdown_Output_Tool.md`

### 385 Persist Finalized Summaries As Markdown
- Persist summaries as optional markdown artifacts in session folders.
- Do not make markdown the terminal default.
- File: `_tasks/pending/385_Persist_Finalized_Summaries_As_Markdown.md`

## Wave 3: Offline Rust-Native Tool Equivalence

These tasks close high-value tool gaps while preserving local-first behavior.

### 393 Observe Metadata Inspection Tool
- Add rust-native metadata-only inspection before full reads.
- File: `_tasks/pending/393_Observe_Metadata_Inspection_Tool.md`

### 396 Workspace Policy Files Ignore And Protected Paths
- Define ignore/protect policy for all local tools.
- File: `_tasks/pending/396_Workspace_Policy_Files_Ignore_Protected.md`

### 395 File Context Tracker And Stale Read Gate
- Track reads, edits, external modifications, and stale write risk.
- File: `_tasks/pending/395_File_Context_Tracker_And_Stale_Read_Gate.md`

### 425 Rust-First File Operation Tool Completeness
- Add native stat/copy/move/mkdir/trash/touch/path tools.
- File: `_tasks/pending/425_Rust_First_File_Operation_Tool_Completeness.md`

### 394 Patch Tool Multi-File Atomic Changes
- Make the patch tool executable, transactional, and rollback-aware.
- File: `_tasks/pending/394_Patch_Tool_Multi_File_Atomic.md`

### 397 Symbol-Aware Repo Map And Tag Cache
- Add token-budgeted symbol map slices and cache invalidation.
- File: `_tasks/pending/397_Symbol_Aware_Repo_Map_And_Tag_Cache.md`

### 398 LSP Diagnostics And Code Intelligence Tool
- Add optional local diagnostics/code-intelligence tool.
- File: `_tasks/pending/398_LSP_Diagnostics_And_Code_Intelligence_Tool.md`

### 399 Auto Lint/Test And Verification Planner
- Detect focused verification commands from manifests and changed files.
- File: `_tasks/pending/399_Auto_Lint_Test_And_Verification_Planner.md`

### 421 Native Git Inspection And Worktree Tool
- Add rust-first git status/diff/log inspection.
- File: `_tasks/pending/421_Native_Git_Inspection_And_Worktree_Tool.md`

### 422 Tool Result Artifact And Reference Ledger
- Store large tool outputs and artifacts as stable evidence references.
- File: `_tasks/pending/422_Tool_Result_Artifact_And_Reference_Ledger.md`

### 417 Clean Room Shell Execution
- Keep unavoidable shell fallback clean, bounded, and sanitized.
- File: `_tasks/pending/417_Clean_Room_Shell_Execution.md`

## Wave 4: Execution Profiles, Jobs, And Local Code Tools

These tasks make Elma act like a practical local agent without relying on brittle shell prompts.

### 406 Sandboxed Execution Profile System
- Define local/restricted/container execution profiles.
- File: `_tasks/pending/406_Sandboxed_Execution_Profile_System.md`

### 418 Background Job Tool And Notify-On-Complete
- Add start/status/output/stop tools for long-running jobs.
- File: `_tasks/pending/418_Background_Job_Tool_And_Notify_On_Complete.md`

### 420 Local Code Interpreter Tool Wrappers
- Add structured Python/Node interpreter wrappers through process APIs.
- File: `_tasks/pending/420_Local_Code_Interpreter_Tool_Wrappers.md`

### 419 Native Download And Attachment Tool
- Add controlled artifact export/download with network disabled by default.
- File: `_tasks/pending/419_Native_Download_And_Attachment_Tool.md`

### 413 Session Rewind And Checkpoint Restore UX
- Add event/snapshot-backed checkpoint inspect and restore.
- File: `_tasks/pending/413_Session_Rewind_And_Checkpoint_Restore_UX.md`

## Wave 5: Optional Network And Extension Tools

These remain offline-disabled by default but give Elma equivalents for source agents that support fetch, browser, MCP, and external tools.

### 403 Web Fetch Tool Security-Gated HTTP
- Implement disabled-by-default HTTP text retrieval with SSRF protections.
- File: `_tasks/pending/403_Web_Fetch_Tool_Security_Gated_HTTP.md`

### 404 Optional Browser Observation Tool
- Add disabled-by-default browser observation using the fetch/network policy.
- File: `_tasks/pending/404_Optional_Browser_Observation_Tool.md`

### 405 MCP Extension Gateway With Offline Gates
- Add disabled-by-default MCP tools through unified metadata and permissions.
- File: `_tasks/pending/405_MCP_Extension_Gateway_With_Offline_Gates.md`

### 408 Versioned Extension State For Sessions
- Add namespaced, versioned extension state storage and migrations.
- File: `_tasks/pending/408_Versioned_Extension_State_For_Sessions.md`

### 426 Offline Search Provider And Web Search Policy
- Prefer local workspace/memory/docs search before optional web search.
- File: `_tasks/pending/426_Offline_Search_Provider_And_Web_Search_Policy.md`

## Wave 6: Workflows, Memory, Modes, And Delegation

These build higher-level agent behavior on top of the reliability and tool layers.

### 407 Recipe And Subrecipe Workflow System
- Add versioned external recipes without prompt bloat.
- File: `_tasks/pending/407_Recipe_And_Subrecipe_Workflow_System.md`

### 409 Headless Event API And SDK Harness
- Add a headless runner with deterministic event output.
- File: `_tasks/pending/409_Headless_Event_API_And_SDK_Harness.md`

### 410 Bounded Subagent Delegation Framework
- Add bounded read-only explorer delegation before write delegation.
- File: `_tasks/pending/410_Bounded_Subagent_Delegation_Framework.md`

### 411 Persistent Project Memory And RAG Index
- Add grounded project memory with provenance and staleness.
- File: `_tasks/pending/411_Persistent_Project_Memory_And_RAG_Index.md`

### 412 Data Analysis Mode
- Add document/data analysis mode using local extraction and evidence.
- File: `_tasks/pending/412_Data_Analysis_Mode.md`

### 423 Source-Agent Command And Slash Action Parity
- Map high-value source-agent commands to Elma equivalents.
- File: `_tasks/pending/423_Source_Agent_Command_And_Slash_Action_Parity.md`

### 428 User Clarification And Completion Tools
- Ask concise follow-up questions when required information is missing.
- File: `_tasks/pending/428_User_Clarification_And_Completion_Tools.md`

### 427 Tool Arsenal Context Budget Adapter
- Keep tool declarations small and relevant for constrained models.
- File: `_tasks/pending/427_Tool_Arsenal_Context_Budget_Adapter.md`

### 383 Lightweight Local Auxiliary LLM Helper
- Optional local helper for summarization/classification/compression.
- File: `_tasks/pending/383_Lightweight_Local_Auxiliary_LLM_Helper.md`

## Wave 7: Diagnostics, Certification, And Regression Gates

These tasks prove the system remains reliable as the tool surface grows.

### 382 Keyword Heuristic Decomposition Audit
- Replace semantic keyword heuristics with confidence/metadata/intel units.
- File: `_tasks/pending/382_Keyword_Heuristic_Decomposition_Audit.md`

### 400 Provider Fault Injection And Error Recovery Harness
- Test provider failures, malformed streams, context overflow, and recovery.
- File: `_tasks/pending/400_Provider_Fault_Injection_And_Error_Recovery_Harness.md`

### 401 Diagnostics Bundle And Doctor Command
- Add local doctor checks, redaction, and session diagnostic bundles.
- File: `_tasks/pending/401_Diagnostics_Bundle_And_Doctor_Command.md`

### 402 Terminal UI Regression Capture Harness
- Add deterministic terminal snapshots for operational rows and footer rules.
- File: `_tasks/pending/402_Terminal_UI_Regression_Capture_Harness.md`

### 414 Release Risk And Security Audit Gate
- Add local release risk scanner for sensitive modules and hidden characters.
- File: `_tasks/pending/414_Release_Risk_And_Security_Audit_Gate.md`

### 424 Tool Equivalent Certification Scenarios From Knowledge Base
- Add offline certification prompts for each source-agent tool family.
- File: `_tasks/pending/424_Tool_Equivalent_Certification_Scenarios_From_Knowledge_Base.md`

### 415 Benchmark Leaderboard And Eval Dashboard
- Normalize benchmark scenarios and reliability scoring.
- File: `_tasks/pending/415_Benchmark_Leaderboard_And_Eval_Dashboard.md`

### 416 File Watcher And AI Comment Workflow
- Add scoped workspace watcher integrated with stale context tracking.
- File: `_tasks/pending/416_File_Watcher_And_AI_Comment_Workflow.md`

## Final Sequencing Notes

- Start with Task 386, then Tasks 376-378. This gives the backlog truth and fixes the most direct wrong-answer causes.
- Do Tasks 387-388 before adding more tools. Tool discovery and rust-first policy decide how all future tools are exposed.
- Do Tasks 389-391 before broad workflow expansion. Pyramid orchestration is the core decomposition mechanism.
- Keep network/browser/MCP tools disabled by default and lower priority than offline equivalents.
- Any new intel unit added by later tasks must obey Task 378 JSON limits.
- Protect `src/prompt_core.rs` unless explicit user approval is recorded in the task.

