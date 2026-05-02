# Master Plan

Last updated: 2026-05-02 (added Task 494, marked 444-450 complete, registered new module architecture)

This is the execution index for current pending tasks. Use it to choose work in dependency order, not as a replacement for each task file. Each task file remains the implementation detail, verification commands, and done criteria.

## New Architecture (Task 494)

The pyramid hierarchy is now wired end-to-end:

```
ComplexityAssessment → RouteDecision → FormulaSelection → WorkGraph (pyramid)
    → Goal → SubGoal → Plan → Instruction → PersistedTask → Step
```

Key modules:
- `src/work_graph.rs` — Pyramid types + builder with `from_complexity()` gating
- `src/approach_engine.rs` — Approach branching with `fork_new_approach()` and `with_complexity()`
- `src/task_persistence.rs` — Session task persistence + `_elma-tasks/NNN_{auto|user}_{slug}_{uid}.md`

File naming:
- Auto-generated (workflow): `001_auto_Read_Cargo_toml_instr_001.md`
- User-initiated: `001_user_Add_dark_mode_usr_001.md`

## Operating Rules

- Move a task from `_tasks/pending/` to `_tasks/active/` before implementation.
- Do not mark a task complete until its own verification section passes.
- Do not modify `src/prompt_core.rs` or `TOOL_CALLING_SYSTEM_PROMPT` unless a task explicitly records user approval for that change.
- Prefer rust-native/offline tools over shell and network tools.
- Keep intel-unit JSON simple: one nested object level maximum, three required fields by default, five total fields absolute maximum.
- Surface routing, tool discovery, retries, compaction, stop reasons, and decomposition as transcript rows.
- Failed approaches do not continue down the objective hierarchy; retry with a new approach toward the same original objective.

## Sequencing Principles

- Stability and truthfulness gates come first, especially anything that can make builds fail, providers fail, or tool declarations lie.
- Policy and ownership decisions come before broad implementation, so later tasks do not duplicate architecture.
- Low-effort, high-gain cleanup is early when it unblocks safer work; cosmetic cleanup and deletion happen after regression coverage.
- Optional network, browser, MCP, delegation, and watcher features stay late and disabled/offline-gated by default.
- Deferred and postponed tasks keep their historical numbers, but overlapping tasks now include reconciliation notes pointing to the current pending successor tasks.

## Phase 1: Stabilization And Architecture Decisions

These tasks reduce immediate failure risk and settle the policy contracts that later work depends on.

### 437 Clippy Lint Gate Baseline Cleanup
- Make the current lint baseline green and decide enforcement policy.
- File: `_tasks/pending/437_Clippy_Lint_Gate_Baseline_Cleanup.md`

### 438 Provider Connectivity And Endpoint Hardening
- Fix duplicated endpoint construction and unify provider-aware connectivity checks.
- File: `_tasks/pending/438_Provider_Connectivity_And_Endpoint_Hardening.md`

### 439 Task Ledger Drift And Completed Claim Reconciliation
- Reconcile duplicate lifecycle entries and completed task claims that no longer match source.
- File: `_tasks/pending/439_Task_Ledger_Drift_And_Completed_Claim_Reconciliation.md`

### 440 Absolute Path Whole-System Access User Policy Decision
- Make a product-level decision on absolute paths and whole-system file access.
- File: `_tasks/pending/440_Absolute_Path_Whole_System_Access_User_Policy_Decision.md`

### 441 Workspace Policy Files Ignore And Protected Paths
- Define ignore/protect policy for all local tools.
- File: `_tasks/pending/441_Workspace_Policy_Files_Ignore_Protected.md`

### 442 Core Tool Path Boundary And Argument Hardening
- Harden read/search/observe/write/delete path handling and shell argument construction.
- File: `_tasks/pending/442_Core_Tool_Path_Boundary_And_Argument_Hardening.md`

### 443 Execution Metadata Truth And Risk Source Unification
- Unify read-only/destructive/risk metadata across tools, steps, preflight, and policy.
- File: `_tasks/pending/443_Execution_Metadata_Truth_And_Risk_Source_Unification.md`

### 444 Tool Registry Ownership Consolidation ✓ (completed 2026-05-01)
### 445 Tool Declaration And Executor Parity Reconciliation ✓ (completed 2026-05-01)
### 446 Tool Policy Metadata Unification For Tool Calling ✓ (completed 2026-05-01)
### 447 Tool Arsenal Context Budget Adapter ✓ (completed 2026-05-02)
### 448 Model Capability Registry And Token Budgeting ✓ (completed 2026-05-02)
### 449 Startup Performance And Repeated Scan Reduction ✓ (completed 2026-05-02)
### 450 Prompt Contract Principle-First Audit Non-Core ✓ (completed 2026-05-02)

### 451 Recipe And Subrecipe Workflow System
- Add versioned external recipe files (TOML) that fill pyramid graph layers.
- Recipes define which formula → which graph layer → which step types.
- File: `_tasks/pending/451_Recipe_And_Subrecipe_Workflow_System.md`

### 494 Full Hierarchy Integration Task Persistence Workflow (IN PROGRESS)
- Wire Complexity → Graph depth, approaches as siblings, task persistence, _elma-tasks/ auto-gen.
- File: `_tasks/pending/494_Full_Hierarchy_Integration_Task_Persistence_Workflow.md`

### 452 User Clarification And Completion Tools
- Ask concise follow-up questions when required information is missing.
- File: `_tasks/pending/452_User_Clarification_And_Completion_Tools.md`

### 453 Request Pattern Builder Decomposition And Recipe Migration
- Migrate brittle request-shape keyword builders into recipes, tests, or focused intel units.
- File: `_tasks/pending/453_Request_Pattern_Builder_Decomposition_And_Recipe_Migration.md`

## Phase 2: Core Local Execution And File Tools

These tasks build safer rust-first local capabilities on top of the policy foundation.

### 454 Search Tool Rust-First Execution Rewrite
- Rewrite model-facing search to avoid shell-string construction and honor schema fields.
- File: `_tasks/pending/454_Search_Tool_Rust_First_Execution_Rewrite.md`

### 455 Patch Tool Multi-File Atomic Changes
- Make the patch tool executable, transactional, and rollback-aware.
- File: `_tasks/pending/455_Patch_Tool_Multi_File_Atomic.md`

### 456 File Context Tracker And Stale Read Gate
- Track reads, edits, external modifications, and stale write risk.
- File: `_tasks/pending/456_File_Context_Tracker_And_Stale_Read_Gate.md`

### 457 Rust-First File Operation Tool Completeness
- Add native stat/copy/move/mkdir/trash/touch/path tools.
- File: `_tasks/pending/457_Rust_First_File_Operation_Tool_Completeness.md`

### 458 Shell Mutation Snapshot And Rollback Coverage Revisit
- Decide rollback coverage for shell-driven file mutations.
- File: `_tasks/pending/458_Shell_Mutation_Snapshot_And_Rollback_Coverage_Revisit.md`

### 459 Sandboxed Execution Profile System
- Define local/restricted/container execution profiles.
- File: `_tasks/pending/459_Sandboxed_Execution_Profile_System.md`

### 460 Background Job Tool And Notify-On-Complete
- Add start/status/output/stop tools for long-running jobs.
- File: `_tasks/pending/460_Background_Job_Tool_And_Notify_On_Complete.md`

### 461 Local Code Interpreter Tool Wrappers
- Add structured Python/Node interpreter wrappers through process APIs.
- File: `_tasks/pending/461_Local_Code_Interpreter_Tool_Wrappers.md`

### 462 Native Git Inspection And Worktree Tool
- Add rust-first git status/diff/log inspection.
- File: `_tasks/pending/462_Native_Git_Inspection_And_Worktree_Tool.md`

### 463 Symbol-Aware Repo Map And Tag Cache
- Add token-budgeted symbol map slices and cache invalidation.
- File: `_tasks/pending/463_Symbol_Aware_Repo_Map_And_Tag_Cache.md`

### 464 LSP Diagnostics And Code Intelligence Tool
- Add optional local diagnostics/code-intelligence tool.
- File: `_tasks/pending/464_LSP_Diagnostics_And_Code_Intelligence_Tool.md`

## Phase 3: Memory, Documents, Sessions, And Events

These tasks improve grounded continuity and long-running session reliability.

### 465 Persistent Project Memory And RAG Index
- Add grounded project memory with provenance and staleness.
- File: `_tasks/pending/465_Persistent_Project_Memory_And_RAG_Index.md`

### 466 Document Capability Truth And Adapter Reconciliation
- Align document capability reports, docs, completed claims, and actual adapter code.
- File: `_tasks/pending/466_Document_Capability_Truth_And_Adapter_Reconciliation.md`

### 467 Document Extraction Resource Bounds And Cache Followthrough
- Bound document sniffing/extraction and make document cache behavior real or explicit.
- File: `_tasks/pending/467_Document_Extraction_Resource_Bounds_And_Cache_Followthrough.md`

### 468 Data Analysis Mode
- Add document/data analysis mode using local extraction and evidence.
- File: `_tasks/pending/468_Data_Analysis_Mode.md`

### 469 Session Runtime State Ownership Audit
- Define canonical ownership for transcript, artifacts, summaries, event log, index, and store.
- File: `_tasks/pending/469_Session_Runtime_State_Ownership_Audit.md`

### 470 Action Observation Event Log For Tool Calling
- Add a typed runtime timeline for current tool-calling sessions.
- File: `_tasks/pending/470_Action_Observation_Event_Log_For_Tool_Calling.md`

### 471 Tool Calling Certification Suites For Current Architecture
- Certify current tool-calling behavior without reviving DSL-specific tests.
- File: `_tasks/pending/471_Tool_Calling_Certification_Suites_Current_Architecture.md`

### 472 Session Rewind And Checkpoint Restore UX
- Add event/snapshot-backed checkpoint inspect and restore.
- File: `_tasks/pending/472_Session_Rewind_And_Checkpoint_Restore_UX.md`

## Phase 4: Diagnostics, Release Gates, And Cleanup Safety

These tasks make regressions visible before cosmetic cleanup or deletion begins.

### 473 Provider Fault Injection And Error Recovery Harness
- Test provider failures, malformed streams, context overflow, and recovery.
- File: `_tasks/pending/473_Provider_Fault_Injection_And_Error_Recovery_Harness.md`

### 474 Diagnostics Bundle And Doctor Command
- Add local doctor checks, redaction, and session diagnostic bundles.
- File: `_tasks/pending/474_Diagnostics_Bundle_And_Doctor_Command.md`

### 475 Release Risk And Security Audit Gate
- Add local release risk scanner for sensitive modules and hidden characters.
- File: `_tasks/pending/475_Release_Risk_And_Security_Audit_Gate.md`

### 476 Cross Platform Portability Gate
- Audit Unix-only APIs, temp paths, shell assumptions, and PATH scans.
- File: `_tasks/pending/476_Cross_Platform_Portability_Gate.md`

### 477 Cargo Dependency And Feature Hygiene Audit
- Audit dependencies, optional features, deprecated dev deps, and manifest hygiene.
- File: `_tasks/pending/477_Cargo_Dependency_And_Feature_Hygiene_Audit.md`

### 478 Headless Event API And SDK Harness
- Add a headless runner with deterministic event output.
- File: `_tasks/pending/478_Headless_Event_API_And_SDK_Harness.md`

### 479 Auto Lint/Test And Verification Planner
- Detect focused verification commands from manifests and changed files.
- File: `_tasks/pending/479_Auto_Lint_Test_And_Verification_Planner.md`

### 480 Tool Equivalent Certification Scenarios From Knowledge Base
- Add offline certification prompts for each source-agent tool family.
- File: `_tasks/pending/480_Tool_Equivalent_Certification_Scenarios_From_Knowledge_Base.md`

### 481 Benchmark Leaderboard And Eval Dashboard
- Normalize benchmark scenarios and reliability scoring.
- File: `_tasks/pending/481_Benchmark_Leaderboard_And_Eval_Dashboard.md`

### 482 Terminal UI Regression Capture Harness
- Add deterministic terminal snapshots for operational rows and footer rules.
- File: `_tasks/pending/482_Terminal_UI_Regression_Capture_Harness.md`

### 483 UI Renderer And Module Deprecation Decision
- Decide canonical UI renderer ownership and deprecate stale UI modules safely.
- File: `_tasks/pending/483_UI_Renderer_And_Module_Deprecation_Decision.md`

### 484 Dead Code And Deprecation Decision Audit
- Audit orphaned, legacy, unused, and misleading code paths before removal.
- File: `_tasks/pending/484_Dead_Code_And_Deprecation_Decision_Audit.md`

## Phase 5: Optional Network, Extension, And Workflow Expansion

These tasks stay late because they broaden the execution surface. They must remain offline-disabled or permission-gated by default.

### 485 Web Fetch Tool Security-Gated HTTP
- Implement disabled-by-default HTTP text retrieval with SSRF protections.
- File: `_tasks/pending/485_Web_Fetch_Tool_Security_Gated_HTTP.md`

### 486 Offline Search Provider And Web Search Policy
- Prefer local workspace/memory/docs search before optional web search.
- File: `_tasks/pending/486_Offline_Search_Provider_And_Web_Search_Policy.md`

### 487 Native Download And Attachment Tool
- Add controlled artifact export/download with network disabled by default.
- File: `_tasks/pending/487_Native_Download_And_Attachment_Tool.md`

### 488 Optional Browser Observation Tool
- Add disabled-by-default browser observation using the fetch/network policy.
- File: `_tasks/pending/488_Optional_Browser_Observation_Tool.md`

### 489 Versioned Extension State For Sessions
- Add namespaced, versioned extension state storage and migrations.
- File: `_tasks/pending/489_Versioned_Extension_State_For_Sessions.md`

### 490 MCP Extension Gateway With Offline Gates
- Add disabled-by-default MCP tools through unified metadata and permissions.
- File: `_tasks/pending/490_MCP_Extension_Gateway_With_Offline_Gates.md`

### 491 Source-Agent Command And Slash Action Parity
- Map high-value source-agent commands to Elma equivalents.
- File: `_tasks/pending/491_Source_Agent_Command_And_Slash_Action_Parity.md`

### 492 Bounded Subagent Delegation Framework
- Add bounded read-only explorer delegation before write delegation.
- File: `_tasks/pending/492_Bounded_Subagent_Delegation_Framework.md`

### 493 File Watcher And AI Comment Workflow
- Add scoped workspace watcher integrated with stale context tracking.
- File: `_tasks/pending/493_File_Watcher_And_AI_Comment_Workflow.md`

## Deferred And Postponed Policy

- Deferred and postponed task numbers are historical and are not part of the pending execution sequence.
- When a deferred/postponed task overlaps the current architecture, its file now carries a `Backlog Reconciliation (2026-05-02)` note pointing to the pending successor task.
- If a deferred/postponed task is revived, first compare it against the successor task and either merge the missing details or explicitly supersede the older task.

## Current First Picks

Tasks 437-443 remain first pick for stabilization. Tasks 444-450 are complete. Task 494 is actively in progress (Phase 1-2 done, Phase 3-6 remaining). Task 451 (recipes) is next logical successor once 494's hierarchy is fully wired.

Do not start UI deprecation, dead-code deletion, network tools, MCP, browser tools, subagents, or file watching until the relevant earlier policy and regression-gate tasks are complete.
