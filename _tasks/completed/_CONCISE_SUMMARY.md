# Concise Task Summary (Completed, Older Tasks)

## JSON Reliability

- **001_Hybrid_JSON** — Implemented 5-layer defense-in-depth JSON reliability (GBNF grammar, few-shot examples, auto-repair, schema validation, fallback values); superseded by masterplan.
- **001_JSON_Reliability_Masterplan** — Built complete JSON reliability pipeline across 7 phases: grammar infrastructure, injection, schema validation, auto-repair, repair intel units, few-shot examples; achieved 99.9% parse success.
- **003_Complete_JSON_Fallback_Integration** — Created unified error handler module with circuit breaker, safe defaults for all components, user-facing error messages, and global fallback tracking.
- **008_Harden_OODA_Loop_And_Critic_JSON** — Planned retry-loop strategy diversification, critic JSON hardening, and reasoning path sanitization; pending architecture updates.
- **008_JSON_Reliability_Pipeline** — Completed 3-phase JSON pipeline: circuit breaker + fallbacks, content grounding for hallucination detection, schema validation + deterministic fix + 4 intel units (text generator, converter, verifier, repair).
- **013_Verify_JSON_Pipeline_For_Small_Models** — Investigated whether intel units use plain-text-first generation vs direct JSON for 3B model reliability; identified gaps and recommended plain-text extraction pipeline.
- **018_Improve_JSON_Repair** — Enhanced multi-stage JSON repair with aggressive extraction, validation, improved json_outputter prompt, and repair metrics; superseded.
- **T044_Eliminate_Critic_JSON** — Replaced critic JSON output with simple text format (`ok: reason` / `retry: reason`) to eliminate parse errors in verification loop.

## Intel Units

- **006_Extend_Narrative_To_All_Intel_Units** — Migrated all intel units from noisy JSON blobs to plain-text narrative input; improved model reasoning consistency across critic, sufficiency, reviewers, and evidence modes.
- **012_Review_Intel_Unit_Atomicity** — Assessed intel units for atomicity and 3B model suitability; identified loaded units needing splitting into single-responsibility atomic units.
- **034_Formalize_Intel_Unit_Interfaces** — Created `IntelUnit` trait with pre-flight/post-flight/fallback interface, `IntelContext` and specialized output types; 6 ladder profiles created; phase 2 deferred.
- **T001_verify_and_delete_obsolete_intel_units** — Verified 16 identified intel units are all in use or tested; none were obsolete; all retained.

## Classification & Routing

- **002_Fix_Speech_Act_Classification** — Updated speech_act prompts to principle-based distinctions (INSTRUCTION changes state, INFO provides answer, CHAT is conversation); superseded by 007.
- **007_Decouple_Classification_From_Execution** — Proposed treating classifier outputs as probabilistic feature vectors rather than hard decisions; enables orchestrator reasoning over rigid rule-following.
- **010_Elma_Helper_Intention_Clarification** — Implemented intention clarifier intel unit that runs before speech act classification, translating ambiguous user input into actionable language (ACTION/INFO/CHAT prefix).
- **012_Entropy_Based_Flexibility** — Added entropy calculation and noise injection to routing outputs to prevent overconfident 100% distributions; superseded.
- **014_Confidence_Based_Routing** — Implemented obvious-chat pattern detection and confidence-based fallback (defaults to safe CHAT route when entropy > 0.8 or margin < 0.15).
- **T001_Terminology_Broke_Classification** — Diagnosed and resolved classification breakage caused by Task 045 terminology overhaul (CHAT->CONVERSATION etc.) that desynchronized router model from code mappings; reverted to original terms.
- **T045_Fix_Info_Instruction_Classification** — Fixed misclassification of implicit action requests (e.g., "what is current date?") as INFO instead of INSTRUCTION; added prompt guidance and post-classification override for shell-command-requiring questions.

## Planning & Formulas

- **001_Revise_And_Perfect_Existing_Formulas** — Transformed formulas from hardcoded command lists into abstract intent patterns with cost/value/risk scoring and runtime efficiency tracking.
- **004_Revise_Core_Formulas_Reply_Family** — Revised 6 reply-family formulas (reply_only, capability_reply, execute_reply, inspect_reply, inspect_summarize_reply, inspect_decide_reply) with principle-based prompts.
- **005_Revise_Core_Formulas_Plan_Family** — Revised 4 plan-family formulas (plan_reply, masterplan_reply, cleanup_safety_review, code_search_and_quote) with principle-based prompts and hierarchical decomposition integration.
- **010_Multi_Strategy_Planning_With_Fallback_Chains** — Implemented strategy chain system (Direct, InspectFirst, PlanThenExecute, SafeMode, Incremental, Delegated) with automatic fallback on failure and strategy effectiveness logging.
- **024_Revise_And_Perfect_Existing_Formulas** — Umbrella task for iterative trial-and-error refinement of all shipped formulas; defined use cases, evidence patterns, failure modes per formula; superseded.

## Execution & Ladder

- **011_State_Aware_Guardrails** — Implemented context drift monitor that compares current Program/StepResult against original Goal at each OODA step, triggering mandatory refinement on divergence.
- **023_Hierarchical_Decomposition** — Implemented complexity-triggered hierarchical decomposition: OPEN_ENDED tasks generate masterplans (3-5 phases), saved to session; prevents massive single-step commands.
- **023_Implement_Complexity_Triggered_Hierarchical_Decomposition** — Original planning document for hierarchical decomposition with 5-level hierarchy (Goal->Subgoal->Task->Method->Action) and parent-child tracking.

## Reflection & Reasoning

- **001_Enable_Reflection_For_All_Tasks** — Removed `should_skip_intel()` check so reflection runs for ALL tasks regardless of complexity, catching hallucination before execution even for simple DIRECT tasks.

## UI & UX

- **007_Optimize_Workspace_Context** — Replaced basic find/ls workspace context with structured tree view (3 levels deep) using Rust `ignore` crate; filters noise, highlights important files, reduces tokens by 30%+.

## Infrastructure

- **009_Align_Tuning_With_Current_Runtime_Architecture** — Established safe tuning policy: captured llama.cpp runtime defaults as protected baseline, added variance penalties, unit-type-specific parameter bands, model behavior integration; prohibited prompt mutation.
- **015_Autonomous_Tool_Discovery** — Implemented CLI tool discovery using `which` crate with caching (7-day TTL, PATH-based invalidation); detects 40+ tools, project-specific tools, and custom scripts.
- **T059_Troubleshoot_Direct_Shell_Path_And_Runtime_Stalls** — Fixed 7 root causes of CLI instability: broken startup config, direct-shell fast path, Unicode truncation panics, chat path stalls, selector contract issues, placeholder handoff bugs, and evidence-free DECIDE hallucination; verified all 12 stress scenarios in real CLI mode.

## Transient Context & Expert Responder

- **023_Expert_Responder_Transient_Context** — Ensured expert responder output stays transient and is never stored in `runtime.messages`; only user/Elma turns persist in chat context.

## UI & UX

- **059_CLI_UI_Basic_Enhancements** — Lightened Elma's output color to pink, improved visibility of classification/planning/reflection steps, and removed decorative emoticons for cleaner terminal UX.
- **102_Surgical_Code_Syntax_Highlighting** — Implemented syntax highlighting for code blocks using `syntect` crate with language detection, ANSI terminal rendering, and caching for large blocks.
- **103_Interactive_Selection_Menus** — Built keyboard-navigable interactive menus with `crossterm` event loop, fuzzy search filtering, and Vim key compatibility for model/profile/theme selection.

## Stability & Reliability

- **058_Incremental_Stability_Master_Plan** — Delivered grounded workspace discovery, hallucination mitigation, JSON stability repairs, CLI stress harness with semantic validation gates, and atomic config writes.
- **061_Role_Based_Temperature_And_Retry_Strategy_Calibration** — Established per-role temperature bands, failure-aware retry ladder, and strategy-chain integration to prevent stale retries and low-model hallucination.
- **062_Final_Answer_Presentation_And_Formatting_Reliability** — Hardened final-answer path with strict role contracts, deterministic terminal-safe formatting by default, and regression tests for over-formatting.
- **064_Real_CLI_Stress_Harness_And_Reliability_Gates** — Built CLI-grounded stress runner with semantic validation gates for answer presence, evidence grounding, sandbox confinement, and honest failure detection.
- **065_Improve_Crash_Reporting** — Implemented comprehensive crash reporting with `SessionError` struct, panic hook with stack traces, `error.json` per fatal error, and improved trace logging.
- **066_Limit_Summarize_Step_Output** — Added output size limits (50KB shell, 8K token summarize), preflight warnings for dangerous commands, and automatic batching for large operations.
- **069_Runtime_Profile_Validation_And_Config_Healthcheck** — Created `config_healthcheck.rs` validating 44 profiles for schema compatibility, grammar mappings, prompt sync coverage, with clear startup diagnostics.
- **072_Specialized_FS_Intel** — Planned lightweight Rust parsers for project-specific config files (Cargo.toml, package.json) to provide structured facts instead of raw shell output.

## Tool System

- **113_Tool_Result_Budget_And_Disk_Persistence** — Implemented tool result budgeting (50K char threshold) with disk persistence, `<persisted-output>` wrapper for model, and aggregate per-message budget (150K default).
- **114_Auto_Compact_Context_Window_Management** — Added token counting, auto-compact trigger near context limit, inline summary strategy for 3B models, and circuit breaker (max 3 failures).
- **115_Streaming_Tool_Execution** — Implemented streaming tool execution with parallel safe tools (read/search), serial unsafe tools (shell), order preservation, and shell error cascading.

## Troubleshooting

- **T059_Troubleshoot_Direct_Shell_Path_And_Runtime_Stalls** — Fixed 7 root causes: broken startup config, direct-shell fast path, Unicode truncation panics, chat path stalls, selector contract issues, placeholder handoff bugs, and evidence-free DECIDE hallucination.

## Terminal Parity & Claude Code UI

- **108_Real_Time_Thinking_Stream** — Removed broken thinking display; thinking extracted internally via `isolate_reasoning_fields()` but not shown in UI; final response shown clean with thinking tags already stripped.
- **110_Claude_Code_Style_Terminal_UI** — Enabled SSE streaming in `request_chat_final_text` with `stream: true`; parses SSE chunks, extracts thinking/content, falls back to non-streaming if unsupported.
- **167_Claude_Code_Source_Audit_And_Golden_Terminal_Harness** — Created `docs/claude_code_terminal_parity_spec.md` from Claude source audit; implemented pseudo-terminal test harness in `tests/ui_parity.rs` with `portable-pty`, `vt100` parsing, and fixture system.
- **169_Claude_Like_Terminal_Renderer_Shell** — Replaced five-frame Elma UI with `ClaudeRenderer` using ratatui; implemented `UiEvent` boundary, quarantined legacy renderer, added Ctrl+O transcript toggle, migrated to theme-aware rendering.
- **178_End_To_End_Claude_Parity_Stress_Gate** — Release gate for Tasks 166-177; passes 25 PTY fixtures covering startup, streaming, tools, pickers, modes, lifecycle, todo, compact, status; integrated fake OpenAI-compatible server for deterministic testing.

## Theme System

- **168_Pink_Monochrome_Theme_Token_System** — Implemented tokenized theme with Pink (#ff4fd8)/Cyan (#00e5ff) defaults; created `src/ui_theme.rs` with `ColorToken` struct; replaced Gruvbox constants with theme-mapped values; added unit tests and color snapshot tests.

## Message Rendering & Markdown

- **170_Message_Rows_Markdown_Transcript_And_Compact_Boundary_Rendering** — Integrated ratatui for flicker-free rendering; implemented `to_ratatui_lines` and `render_ratatui` methods; added compact boundary rows, expanded/collapsed transcript mode, markdown rendering with theme tokens.
- **179_Scroll_Behavior_Sticky_Header_And_New_Messages_Pill** — Implemented sticky prompt header (`❯ {truncated prompt}`) when scrolled up; added new messages pill (`─── N new messages ───` divider); added divider tracking and `count_unseen_assistant_turns()` helper.
- **180_Message_Row_Indicators_Spacing_And_Task_List** — Changed user prefix from `>` to `❯`; added user message truncation (10K char cap); added blank line before assistant messages; filtered to show only last thinking block; updated task symbols (◻/◼/✔/▸).
- **181_Transcript_Mode_Shortcuts_And_Model_Picker_Wiring** — Implemented `TranscriptMode` enum (Normal/Transcript/Search); added Vim-style shortcuts (g/G/j/k/b/Space for navigation, `/` for search); Ctrl+O now toggles transcript mode with enhanced keyboard navigation.
- **189_Deep_Markdown_Renderer** — Replaced primitive renderer with `pulldown-cmark` parser; added `syntect` syntax highlighting for code blocks; implemented headers, bold, italic, inline code, links, lists, blockquotes, tables, horizontal rules.
- **U007_Enhanced_Markdown_Table_Support** — Enhanced `claude_markdown.rs` to detect and render markdown tables with column width calculation, aligned rendering with borders using Spans.

## Tool UX & Permissions

- **172_Tool_Use_UX_Permissions_Progress_And_Output_Collapse** — Replaced `MessageRole::Tool` with Claude-style `ClaudeMessage::ToolStart/ToolResult`; implemented permission UX with `wait_for_permission()`; added long output collapse with `(X more lines)` indicator; implemented batch grouping for read/search sequences.
- **U001_Structured_Diff_Engine** — Created `src/ui/ui_diff.rs` with `StructuredDiff` struct using `ratatui` and `similar` crate; integrated diff generation into `execution_steps_edit.rs`; added color-coding for added/removed/modified lines.

## Input & Interaction

- **173_Prompt_Input_Slash_Picker_File_Mentions_And_Command_Modes** — Implemented Claude-style prompt input with multiline support (Shift+Enter); added `/` slash command picker with fuzzy filtering; added `@` file quick-open picker with workspace discovery; added `!` bash mode with Cyan indicator; implemented keybindings (Ctrl-A/E/B/F, Alt-B/F, Ctrl-U/K/W, Ctrl-_, double Esc/Ctrl-C/D).
- **174_Todo_Tool_And_Claude_Style_Task_List** — Added `update_todo_list` tool with actions (add/update/in_progress/completed/blocked/remove/list); implemented Claude-style task list rendering with checkmarks, status markers (✓/◐/○/◌), Ctrl-T toggle, hidden count indicator.
- **183_Input_And_Rendering_Enhancements** — Implemented Shift+Enter for multiline input with vertical growth; input area expands dynamically with proper cursor handling.
- **188_Recursive_File_Picker_Workspace_Discovery** — Updated `discover_workspace_files()` to use `ignore` crate for recursive walk (max 10 levels); respects `.gitignore` patterns, skips hidden files, limits to 10K files, truncates display to 30 results.
- **U005_Global_Search_Dialog** — Created `ui_modal_search.rs` with `SearchModal` struct using ratatui popup; integrated Ctrl+K key binding; modal handles query typing, navigation (Up/Down), Enter to select, Esc to close.
- **U006_Model_Picker** — Created `ui_model_picker.rs` with `ModelPicker` struct using ratatui popup; integrated Ctrl+M key binding; displays model name, max tokens, temperature; navigation with Up/Down, Enter to select.

## Session & Status

- **175_Context_Compaction_Status_Line_Notifications_And_Footer_State** — Replaced old `ui_context_bar` with Claude-style status line (`model=X  ctx=Y/Z`); implemented notifications with 5-second TTL via `notification_expiry` field; wired compact UI rows (manual `/compact` and auto-compact) with boundary/summary messages.
- **176_Session_Lifecycle_Resume_Clear_Exit_And_History_UX** — Implemented `/clear` with Claude system message; added `/resume` with session picker modal from `sessions/` directory; implemented double Ctrl-C/D exit semantics, double Esc prompt clear; added busy-input polling for queueing prompts during active turns.
- **U002_Context_Visualization** — Integrated context bar into Claude UI status line; added `FooterMetrics` in `ui_state.rs` for cumulative token tracking; dynamically scales based on model limits.
- **U003_Coordinator_Agent_Status** — Created `src/ui/ui_coordinator_status.rs` with `CoordinatorStatus` struct; renders at top of screen when active; wired into step execution with purpose description; uses Pink theme token.
- **U004_Virtual_Message_List** — Added scroll_offset to `ClaudeTranscript`; implemented scroll_up/down methods; modified `ClaudeRenderer` for auto-scroll and manual scroll; added Ctrl+U/D key bindings.

## Legacy Cleanup

- **177_Legacy_UI_Removal_And_Documentation_Consolidation** — Moved superseded UI tasks to `_tasks/postponed/` with `_SUPERSEDED_BY_` suffix; quarantined legacy modules (`ui_context_bar`, `ui_spinner`, `ui_progress`, `ui_interact`); old autocomplete superseded by Claude picker; ran `rg` scan to verify no stale UI in active path.
- **182_Legacy_Modal_Removal_And_Transcript_Unification** — Migrated all modal rendering from `draw_with_modal()` to Claude renderer; implemented `render_modal_claude()` with centered pane/border/title for all modal types (Confirm, Help, Select, Settings, Usage, ToolApproval, PermissionGate, PlanProgress, Notification, Splash); removed legacy `draw_with_modal()` method.

## Branding & Visual Polish

- **200_Branded_Splash_And_Compact_Header** — Added branded Elma startup splash from `logo/Elma_CLI_Logo.png` (3 seconds); implemented compact header with minimal logo, execution mode/formula label, workspace/session/model essentials; uses tokenized black/white/grey/pink palette.

## Troubleshooting Tasks (April 2026)

- **T001_Terminology_Broke_Classification** — Diagnosed ALL requests classified as CONVERSATION after Task 045 terminology overhaul; router model tuned on old terms (CHAT, SHELL, WORKFLOW) didn't match new code terms; reverted terminology to original terms.
- **T001_verify_and_delete_obsolete_intel_units** — Verified 16 identified intel units (EvidenceNeeds, ActionNeeds, PatternSuggestion, etc.); all have test coverage in `intel_units.rs`; none were obsolete; no deletions performed.
- **T044_Eliminate_Critic_JSON** — Replaced critic JSON output with simple text format (`ok: reason` / `retry: reason`); updated `orchestration_loop_reviewers.rs` and `verification.rs` to parse simple verdicts; 90% reduction in parse errors.
- **T045_Fix_Info_Instruction_Classification** — Fixed implicit action requests (e.g., "what is current date?") misclassified as INFO; updated `speech_act.toml` with guidance; added post-classification override in `app_chat_core.rs` for shell-command-requiring questions.
- **T001_Session_Storage_Location** — Fixed sessions not saving to expected "sessions" directory; fixed inconsistent path resolution between main app (system data dir) and session-gc (repo dir); modified `sessions_root_path()` to always use `repo_root().join(sessions_root)`.
- **T179_Terminal_UI_Hang_Triage** — Identified blocking `stdin().read_line()` in `permission_gate.rs`; added TTY check to detect PTY/TUI mode; modified to deny dangerous commands in non-TTY mode instead of hanging.
- **T180_Fix_Prompt_Non_Editable** — Fixed missing key input handlers in `poll_busy_submission()`; replaced blocking `wait_for_permission()` with async `request_permission()` using tokio oneshot channel; added trackpad scroll fix with crossterm mouse capture.
- **T206_Terminal_Markdown_And_Reasoning_Leak** — Fixed raw model reasoning leaking into transcript; stopped `reasoning_format=none` from being upgraded to `auto`; rewrote ratatui markdown renderer for proper bullet/inline code handling; strengthened final-answer presentation contract.
- **T207_Cheap_Storage_Query_Strategies** — Fixed expensive per-file enumeration for storage questions; added platform-aware command repair (macOS vs Linux); implemented strategy catalog for aggregate storage queries using `du`/`find` instead of per-file `stat` loops.
- **T208_Context_Bar_And_Model_Budget_Divergence** — Fixed misleading 100% context indicator when transcript bloated by large tool traces; introduced `model_context_tokens_estimate` vs `transcript_tokens_estimate`; footer now primarily reflects model-facing message budget.
- **T209_Proactive_Budget_Forecasting** — Added budget forecasting before expensive shell strategies; taught Elma to identify high-risk command shapes; auto-compaction no longer depends only on multi-turn conversations.
- **T210_Thinking_Auto_Collapse_Timer** — Added visual indicators for thinking states (spinner during streaming, collapse prefixes); implemented word-count-based auto-collapse timer (reading speed 300 WPM calculation).
- **T235_Image_1_UI_Representation_Recovery** — Fixed assistant gutter drifting onto blank lines; converted transcript to typed notices with ephemeral prompt hints; replaced string-built footer with typed `FooterModel`; restored live Thinking streaming; sanitized shell capture with PTY-backed subterminal.
- **T301_Shell_Timeout_On_Large_Workspace_Find** — Fixed `find` timing out due to `.opencode/node_modules/` with 250K+ files; added `.opencode/` to all exclusion lists in `workspace_tree.rs`, `workspace.rs`, `snapshot.rs`.
- **T302_Preflight_False_Positive** — Fixed `check_protected_paths()` using simple `contains()` that falsely blocked exclusion flags like `! -path "*/.git/*"`; added `find_exclusion_flag_ranges()` to skip protected path checks inside exclusion arguments.
- **T303_Retry_Loop_Without_Strategy_Change** — Diagnosed model retrying same `find` command 6 times with only cosmetic changes; identified need for failure classification and strategy diversity; timeout info not exposed to model for adaptation.
- **T304_Truncated_Final_Answer** — Diagnosed final answer truncated mid-sentence due to context budget exhausted by 6 failed tool calls; identified need for answer budget reservation (25% of max_tokens for final output).
- **T305_Empty_Goal_State_For_Multi_Step_Request** — Implemented goal seeding from multi-step requests; added `src/goal_seeding.rs` with word-boundary verb matching, clause splitting, dual seeding (multi-step → subgoals, single-step → objective-only).
- **T314_Persistent_Shell_Corruption_After_Timeout** — Fixed shell session dying after timeout with all subsequent commands failing with EOF; added `dead` flag to `PersistentShell`; implemented auto-recreation of shell session on timeout/EOF.
- **T315_No_Shell_Recovery_After_EOF_Error** — Diagnosed "Shell EOF before finding marker" error propagating 6 times without recovery; identified need for shell restart on EOF with transparent recovery to model.

## Old Feature Tasks (April 21)

- **011_Typeahead_And_Auto_Complete_System** — Planned intelligent typeahead with triggers (`/`, `@`, `!`, `?`); fuzzy matching via `src/ui/fuzzy.rs`; dropdown rendering with arrow key navigation; integration into `src/ui/composer.rs`.
- **016_Permission_System** — Implemented permission approval system for dangerous operations; created `src/permission.rs` with Permission enum (Read/Write/Execute/Delete/Network) and Decision enum (Allow/AllowSession/Deny); added `src/permission/service.rs` with caching; integrated UI dialog in `src/ui/permission_modal.rs`.
- **017_Graceful_Shutdown_And_Panic_Recovery** — Planned panic recovery via `std::panic::catch_unwind` in `src/panic.rs`; implemented `src/shutdown.rs` with broadcast channel for graceful cleanup; added `src/logging.rs` for persistent logs to `~/.elma-cli/logs/`.
- **018_Retry_With_Exponential_Backoff** — Planned retry logic in `src/retry.rs` with `RetryConfig` (max_retries=8, base_delay=1s, max_delay=60s, jitter=0.1); implemented retry function with retriable error detection (429, 500, 502, 503, 504); integrated into `src/api_client.rs`.
- **019_Generic_PubSub_Broker** — Implemented generic pub/sub pattern in `Broker<T>` with HashMap-based subscriber management; features include non-blocking publish, context-based subscription cleanup, thread-safety, shutdown support; used for skill discovery, permission notifications, file tracking, session events.
- **020_Interactive_Permission_System** — Implemented interactive permission system based on Crush's model; `PermissionRequest` struct with tool/action/path tracking; service interface with `GrantPersistent()`, `Grant()`, `Deny()`, `AutoApproveSession()`; allowlist support and path-based permissions.

## Dependency Additions (April 24-25)

- **206_Replace_Ad_Hoc_Error_Types** — Added `thiserror = "1.0"` for clean custom error types; convert module-level errors to `thiserror` enums with `#[derive(Error, Debug)]`; provides `From` impls for `?` propagation; pairs with `anyhow` for application-level errors.
- **207_Explicit_Directories_Dependency** — Added `directories = "6.0"` for cross-platform path resolution; replaces hardcoded path assumptions; uses `ProjectDirs`, `UserDirs`, `ConfigDirs` for portable config/session/data paths.
- **208_Structured_Logging** — Added `tracing = "0.1"` and `tracing-subscriber` for structured logging; created `src/logging.rs` with span-based instrumentation; replaces `println!/eprintln!` diagnostics; configurable via `RUST_LOG` or `--verbose`.
- **209_Diagnostic_Errors** — Added `miette = "7.0"` for diagnostic errors with source spans; created `src/diagnostics.rs` with `ElmaDiagnostic` enum; integrates with `anyhow` via `IntoDiagnostic`; provides labeled spans and help suggestions.
- **210_Shell_Completions** — Added `clap_complete = "4.5"` for shell completions via Clap; generates completion scripts for bash/zsh/fish/powershell; implemented `src/cli/completions.rs` for dynamic completion generation.
- **211_Confirmations_And_Selections** — Added `dialoguer = "0.11"` for confirmations and selections; provides `confirm!()`, `select!()`, `input!()` macros; integrated into permission dialogs and interactive prompts.
- **212_Safe_Temporary_Files** — Added `tempfile = "3.10"` for safe temporary files; uses `tempfile()` and `NamedTempFile` for automatic cleanup; prevents temp file leaks during tool execution.
- **213_Shell_String_Parsing** — Added `shlex = "1.3"` for shell string parsing; provides `quote()` and `split()` for safe command string handling; prevents shell injection in dynamic command building.
- **214_Beautiful_Panic_Reports** — Added `color_eyre = "0.6"` for beautiful panic reports; installs custom panic hook with colorized backtraces; integrates with `tracing` for context-aware error reports.
- **215_Terminal_Dimension** — Added `console = "0.15"` for terminal dimension and interaction helpers; provides `Term::stdout()` for size detection and `console::Key` for enhanced input handling.
- **216_Human_Readable_Size_Formatting** — Added `humansize = "2.1"` for human-readable size formatting; provides `format_size()` for bytes-to-readable-string conversion; used in file size displays and storage reports.
- **217_Safe_Delete** — Added `trash = "5.0"` for safe delete via system trash; implements `delete()` for macOS/Linux/Windows trash integration; prevents accidental permanent deletion during file operations.
- **227_Lazy_Static_Initialization** — Added `once_cell = "1.20"` for lazy static initialization; replaces `std::sync::LazyLock` where `INIT` patterns needed or older toolchain compatibility required; used for Ratatui theme, skill registry, formula catalog.
- **229_Chainable_Debug_Transforms** — Added `tap = "1.0"` for chainable `.tap()` debugging and transforms; allows inline value inspection without breaking chains; useful for debug logging in complex expressions.
- **230_Enum_Utilities** — Added `strum = "0.25"` with derive features for enum iteration, string conversion, and variant metadata; provides `#[derive(EnumString, Display, EnumIter, EnumCount)]` to eliminate boilerplate match arms.
