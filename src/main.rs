#![recursion_limit = "256"]
//! @efficiency-role: orchestrator

pub(crate) use anyhow::{Context, Result};
pub(crate) use clap::Parser;
pub(crate) use miette::IntoDiagnostic;
pub(crate) use reqwest::Url;
pub(crate) use serde::de::DeserializeOwned;
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use std::collections::HashMap;
pub(crate) use std::fs::OpenOptions;
pub(crate) use std::io::{self, IsTerminal, Write};
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::Command;
pub(crate) use std::sync::{Mutex, OnceLock};
pub(crate) use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod orchestration;
mod abstractions; // Task 576: Injectable abstractions for testing
mod agent_fsm; // Task 562: Agent FSM lifecycle
mod app;
mod app_bootstrap;
mod app_bootstrap_core;
mod app_bootstrap_modes;
mod app_bootstrap_profiles;
mod app_chat;
mod app_chat_core;
mod app_chat_handlers;
mod app_chat_helpers;
mod app_chat_loop;
mod app_chat_orchestrator;
mod app_chat_trace;
mod approach_engine; // Task 390: Approach Branch Retry And Prune Engine
mod approach_rehydration; // Task 652: Approach branch rehydration and failure taxonomy
mod artifact_verifier; // Task 688: Artifact deliverable verifier
mod atomic_write; // Task 575: Atomic file writes
mod auto_compact; // Task 114: Auto-Compact (Context Window Management)
mod auto_verification; // Task 675: Auto lint, test, and verification planner
mod background_task; // Task 268: Background Task Management
mod budget_forecaster; // Task 653: Budget forecaster & context envelope management
mod certification_suite; // Task 676: JSON tool calling certification suites
mod claude_ui;
mod code_index; // Task 668: Persistent offline code index
mod command_budget; // Task 121: Command Budget & Rate Limiting
mod complexity_gate; // Task 653: Complexity gate for work graph depth control
mod config_cmd;
mod config_validate; // Task 583: Config validation at startup
mod context_budget; // Task 568: Context window budget tracking
mod continuity;
mod data_analysis; // Task 671: Offline data analysis mode
mod debloat_audit; // Task 678: Dead code deprecation and large module debloating audit
mod dependency_audit; // Task 674: Cargo dependency feature hygiene and supply risk audit
mod portability_gate; // Task 673: Cross-platform portability gate
mod security_audit; // Task 677: Release risk security audit gate
mod session_browser; // Task 380: Semantic Continuity Tracking

mod decomposition; // Task 023: Hierarchical decomposition
mod defaults;
mod defaults_core;
mod defaults_evidence;
mod defaults_evidence_core;
mod defaults_router;
mod dense_coder_sanitizer; // Task 644: Dense coder output sanitization
mod diagnostics;
mod diagnostics_bundle; // Task 665: Diagnostics bundle and doctor command
mod dirs;
mod document_adapter; // Task 197: Document intelligence skill stack
mod effective_history; // Task 310: Deferred Pre-Turn Summary
mod env_utils; // Task 290: Clean environment injection for persistent shell
mod errors; // Task 564: Structured error types
mod evaluation;
mod evaluation_response;
mod evaluation_routing;
mod evaluation_workflow;
mod event_ledger; // Task 657: Tool Execution Event Ledger
mod event_log; // Task 470: Action-Observation Event Log
mod evidence_ledger; // Task 287: Evidence Ledger
mod evidence_summary; // Task 287: Evidence Summarization
mod execution;
mod execution_profiles; // Task 459: Sandboxed Execution Profile System
mod execution_steps;
mod execution_steps_compat;
mod execution_steps_edit;
mod execution_steps_read;
mod execution_steps_search;
mod execution_steps_selectors;
mod execution_steps_shell;
mod execution_steps_shell_exec;
mod execution_steps_shell_preflight;
mod experimental_reasoning; // Task 685: Experimental reasoning tuning
mod extension_gateway; // Task 680: Extension state MCP with offline gates
mod file_scout; // Task 198: Read-only whole-system file scout
mod file_watcher; // Task 682: File watcher and autosave workflow
mod final_answer; // Task 384: Clean-Context Finalization Enforcement
mod finalization_hardener; // Task 766: Strengthen finalization against stale artifacts
mod finalization_verifier; // Task 690: Evidence grounded finalization honesty
mod footer_contract; // Task 641: Footer contract for core-metrics-only status bar
mod format;
mod formulas;
mod fs_intel; // Task 072: Specialized Filesystem Intel
mod goal_seeding; // T305: Goal seeding from multi-step requests
mod guardrails; // State-aware guardrails for context drift (Task 011)
mod guardrails_refinement; // Guardrails refinement phase (Task 011)
mod headless_api; // Task 679: Headless event API and SDK harness
mod hook_system; // Tasks 123, 124, 125: Extensible hook framework
mod hybrid_search; // Task 273: Hybrid Search Memory System With FTS And Vector Search
mod input_controller; // Task 637: Input controller
mod input_parser; // Task 013: Smart Input Prefixes And Command Modes
mod instruction_repair; // Task 391: Instruction-Level Repair And Result Recombiner
mod intel_narrative; // Narrative transformation for intel units
mod intel_narrative_steps; // Step-related narrative functions and helpers
mod intel_narrative_utils; // Shared narrative utility helpers
mod intel_trait; // Intel unit trait and interfaces
mod intel_units; // Migrated intel units (complexity, evidence, action, workflow)
mod interpreter_tools; // Task 461: Local Code Interpreter Tool Wrappers
mod json_error_handler; // JSON error handling with circuit breaker
mod json_grammar; // GBNF grammar loading and injection
mod json_parser; // Robust JSON parsing for intel unit outputs
mod json_parser_extract; // Extraction helpers for json_parser
mod json_repair; // Deterministic JSON repair pipeline (Task 378)
mod json_tuning; // JSON temperature tuning
mod keyword_audit; // Task 650: Keyword gate audit and analyzer rule
mod llm_config;
mod llm_provider; // Task 278: Native Rust LLM API Client
mod logging;
mod markdown_ansi; // Markdown-to-ANSI terminal rendering
mod metrics;
mod model_capabilities; // Task 448: Model Capability Registry And Token Budgeting
mod model_capability_probe; // Task 643: Model Capability Probe
mod models_api;
mod mutation_contract; // Task 699: Mutating request execution and verification contract
mod network_policy; // Task 683: Network fetch/download/browser and offline search policy
mod objective_state; // Task 763: Objective state and approach supervisor
mod offline_lsp; // Task 670: Offline LSP diagnostics and code intelligence tool
mod online_verification; // Task 694: Online verification policy and tool routing
mod optimization;
mod optimization_eval;
mod optimization_tune;
mod orchestration_core;
mod patch_executor; // Task 455: Patch Tool Multi-File Atomic
mod paths;
mod permission_gate; // Task 117: Permission Gate for Destructive Commands
mod persistent_shell; // Task 288: Persistent Guarded Shell
mod process_group; // Task 659: Process group cleanup and background job runtime
mod profile_sets;
mod program;
mod program_policy;
mod program_policy_level;
mod program_policy_tests;
mod program_steps;
mod output_truncation;
mod program_utils;
mod project_guidance;
mod project_init;
mod project_memory; // Task 669: Local project memory with security scanning
mod prompt_constants;
mod prompt_core; // Task 313: Protected Core System Prompt
mod protected_paths; // Task 551: Protected path blocking
mod provider_fault_injector; // Task 646: Fault injection and stream error recovery
mod pubsub; // Task 019: Generic Pub/Sub Broker
mod reasoning_visibility; // Task 642: Reasoning Visibility Policy
pub mod recipes; // Task 451: Recipe And Subrecipe Workflow System
mod refinement;
mod remote_daemon; // Task 684: Remote daemon channel and notification integrations
mod repo_explorer; // Task 196: Repo explorer and analyzer skill
mod repo_map; // Task 463: Symbol Aware Repo Map And Tag Cache
mod retry; // Task 570: Bounded retry with exponential backoff
mod provider_recovery; // Task 693: Provider finalization recovery
mod runtime_task;
mod safe_mode; // Task 272: Safe Mode Toggle System For Permission Levels
mod safe_operations; // Task 692: Safe file operation planning and verification
mod sanitize; // Task 577: ANSI escape sanitization boundary
mod scenarios;
mod scope_coverage; // Task 764: Scope coverage ledger as completion contract
mod search_ranker; // Task 672: Search result analysis intel unit
mod session;
mod session_cleanup;
mod session_display;
#[cfg(test)]
mod session_regression_test; // Task 769: Last session replay regression harness
mod session_error;
mod session_flush; // Task 283: Session Transcript Flush
mod session_gc; // Task 282: Session Garbage Collector
mod session_hierarchy;
mod session_index; // Task 282: Session Index
mod session_paths;
mod session_persistence_adapter; // Task 635: Session persistence adapter
mod session_state; // Task 554: Session-scoped state container
mod session_store; // Task 277: SQLite Session Storage
mod session_store_typed; // Task 663: Session store with typed message parts
mod session_write;
mod shell_exec_policy; // Task 658: Parser-backed shell execution policy and permission cache
mod shell_preflight; // Task 116: Destructive Command Detection & Preflight
mod shutdown; // Task 017: Graceful Shutdown And Panic Recovery
mod skills;
mod shell_timeout;
mod snapshot;
mod sse_stream; // Task 558: SSE byte stream parser
mod stop_policy;
mod storage;
mod stream_types; // Task 558: SSE streaming types
mod streaming_tool_executor; // Task 115: Streaming Token Execution
mod strict_tool_parser; // Task 645: Strict tool argument parsing and model-facing error contracts
mod subagent; // Task 681: Bounded local subagent delegation framework
mod system_monitor; // Right-side panel system resource monitor
mod task_persistence; // Task 494: Session task persistence & _elma-tasks/
mod task_steward; // Task 202: Project task steward skill
mod temp;
mod text_utils;
mod thinking_content;
mod token_counter; // Task 499: tiktoken-rs integration
mod tool_calling;
mod tool_degradation; // Task 654: Tool degradation and retry planning
mod tool_discovery;
mod tool_loop;
mod tool_metadata; // Task 656: Tool metadata policy and discoverable workspace info
mod tool_registry;
mod tool_repair; // Task 689: Schema guided tool argument repair
mod tool_result_storage; // Task 113: Tool Result Budget & Disk Persistence
mod tools; // Tools Module - tool caching, discovery, validation, and execution
mod trace_reducer; // Task 667: Replayable trace reducer and raw payload bundle
mod trajectory; // Task 271: Trajectory Compression For Long-Running Sessions
mod turn_context_packet; // Task 701: Minimal turn context packet for dense models
mod trash;
mod tune;
mod tune_runtime;
mod tune_scenario;
mod tune_setup;
mod tune_summary;
mod tune_scenario_helpers;
mod types;
mod types_api;
mod types_core;
mod types_core_impl;
mod types_hierarchy;
mod ui;
mod ui_reducer; // Task 635: Pure UI Reducer
mod ui_runtime_event; // Task 635: Canonical UI Runtime Event
mod ui_snapshot; // Task 639: UI snapshot harness
mod ui_status_thread; // Task 311: Persistent Status Thread
mod ui_view_state; // Task 635: Pure UI View State
mod verification;
mod verification_evidence;
mod work_graph; // Task 389: Pyramid Work Graph
mod work_graph_bridge; // Task 494: Bridge graph → tasks → steps
mod work_graph_persistence; // Task 651: Work graph task persistence
mod work_graph_runner; // Tasks 763-769: Work graph integration with tool loop
mod workspace;
mod workspace_path_resolver; // Task 765: Workspace path resolution and failed path recovery
mod workspace_policy; // Task 441: Workspace ignore/protect policy
mod workspace_tree; // Task 169: Claude Code-style Terminal UI

pub(crate) use abstractions::*;
pub(crate) use agent_fsm::*;
pub(crate) use atomic_write::*;
pub(crate) use config_validate::*;
pub(crate) use context_budget::*;
pub(crate) use decomposition::*; // Task 023
pub(crate) use defaults::*;
pub(crate) use defaults_evidence::*; // JSON pipeline intel functions
pub(crate) use document_adapter::*; // Task 197: Document intelligence
pub(crate) use errors::*;
pub(crate) use evaluation::*;
pub(crate) use execution::*;
pub(crate) use file_scout::*; // Task 198: File scout
pub(crate) use guardrails::*; // State-aware guardrails (Task 011)
pub(crate) use guardrails_refinement::*; // Guardrails refinement phase (Task 011)
pub(crate) use intel_trait::*; // Intel unit trait and interfaces
pub(crate) use intel_units::*; // Migrated intel units
pub(crate) use json_error_handler::*; // JSON error handling
pub(crate) use json_grammar::*; // GBNF grammar loading and injection
pub(crate) use json_tuning::*; // JSON temperature tuning
pub(crate) use llm_config::*;
pub(crate) use metrics::*;
pub(crate) use models_api::*;
pub(crate) use optimization::*;
pub(crate) use orchestration::*;
pub(crate) use paths::*;
pub(crate) use profile_sets::*;
pub(crate) use program::*;
pub(crate) use program_utils::*;
pub(crate) use project_guidance::*;
pub(crate) use project_init::*;
pub(crate) use prompt_constants::*;
pub(crate) use protected_paths::*;
pub(crate) use reasoning_visibility::*;
pub(crate) use refinement::*;
pub(crate) use repo_explorer::*; // Task 196: Repo explorer
pub(crate) use retry::*;
pub(crate) use runtime_task::*;
pub(crate) use sanitize::*;
pub(crate) use scenarios::*;
pub(crate) use session::*;
pub(crate) use session_display::*;
pub(crate) use session_flush::*; // Task 283: Session Transcript Flush
pub(crate) use session_state::*;
pub(crate) use skills::*;
pub(crate) use snapshot::*;
pub(crate) use stop_policy::*;
pub(crate) use storage::*;
pub(crate) use task_steward::*; // Task 202: Task steward
pub(crate) use text_utils::*;
pub(crate) use thinking_content::*;
pub(crate) use tune::*;
pub(crate) use types::*;
pub(crate) use ui::*;
pub(crate) use verification::*;
pub(crate) use workspace::*;
pub(crate) use workspace_policy::*; // Task 441: Workspace ignore/protect policy

#[tokio::main]
async fn main() {
    color_eyre::install().unwrap();
    let args = crate::types::Args::parse();
    logging::init_logging(args.debug_trace);

    if let Some(command) = &args.command {
        match command {
            crate::types::Commands::Completion { shell } => {
                use clap::CommandFactory;
                let mut cmd = crate::types::Args::command();
                clap_complete::generate(*shell, &mut cmd, "elma-cli", &mut std::io::stdout());
                return;
            }
            crate::types::Commands::Config { action } => {
                config_cmd::handle_config_command(action, &args.config_root);
                return;
            }
            crate::types::Commands::SessionGc {
                older_than_days,
                dry_run,
                confirm,
                compress,
                archive_dir,
            } => {
                // Convert sessions_root to PathBuf
                let sessions_root = match crate::paths::sessions_root_path(&args.sessions_root) {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Error resolving sessions root: {}", e);
                        return;
                    }
                };
                let gc_args = crate::session_gc::SessionGcArgs {
                    older_than_days: *older_than_days,
                    dry_run: *dry_run,
                    confirm: *confirm,
                    compress: *compress,
                    archive_dir: archive_dir.as_ref().map(PathBuf::from),
                };
                match crate::session_gc::run_session_gc(&sessions_root, &gc_args) {
                    Ok(output) => println!("{}", output),
                    Err(e) => eprintln!("Error: {}", e),
                }
                return;
            }
        }
    }

    if let Err(e) = app::run(args).await {
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}
