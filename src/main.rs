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

mod app;
mod config_cmd;
mod session_browser;
mod approach_engine; // Task 390: Approach Branch Retry And Prune Engine
mod app_bootstrap;
mod app_bootstrap_core;
mod app_bootstrap_modes;
mod app_bootstrap_profiles;
mod app_chat;
mod app_chat_builders_advanced;
mod app_chat_builders_audit;
mod app_chat_builders_basic;
mod app_chat_builders_probes;
mod app_chat_core;
mod app_chat_fast_paths;
mod app_chat_handlers;
mod app_chat_helpers;
mod app_chat_loop;
mod app_chat_orchestrator;
mod app_chat_orchestrator_tests;
mod app_chat_patterns;
mod app_chat_trace;
mod auto_compact; // Task 114: Auto-Compact (Context Window Management)
mod background_task; // Task 268: Background Task Management
mod claude_ui;
mod command_budget; // Task 121: Command Budget & Rate Limiting
mod config_validate; // Task 583: Config validation at startup
mod continuity; // Task 380: Semantic Continuity Tracking

mod decomposition; // Task 023: Hierarchical decomposition
mod defaults;
mod defaults_core;
mod defaults_evidence;
mod defaults_evidence_core;
mod defaults_router;
mod diagnostics;
mod dirs;
mod document_adapter; // Task 197: Document intelligence skill stack
mod effective_history; // Task 310: Deferred Pre-Turn Summary
mod env_utils; // Task 290: Clean environment injection for persistent shell
mod evaluation;
mod evaluation_response;
mod evaluation_routing;
mod evaluation_workflow;
mod evidence_ledger; // Task 287: Evidence Ledger
mod evidence_summary; // Task 287: Evidence Summarization
mod event_log; // Task 470: Action-Observation Event Log
mod execution;
mod execution_ladder; // Execution ladder for minimum-sufficient orchestration
mod execution_steps;
mod execution_steps_compat;
mod execution_steps_edit;
mod execution_steps_read;
mod execution_steps_search;
mod execution_steps_selectors;
mod execution_steps_shell;
mod execution_steps_shell_exec;
mod execution_steps_shell_preflight;
mod execution_profiles; // Task 459: Sandboxed Execution Profile System
mod repo_map; // Task 463: Symbol Aware Repo Map And Tag Cache
mod interpreter_tools; // Task 461: Local Code Interpreter Tool Wrappers
mod file_scout; // Task 198: Read-only whole-system file scout
mod final_answer; // Task 384: Clean-Context Finalization Enforcement
mod format;
mod formulas;
pub mod recipes; // Task 451: Recipe And Subrecipe Workflow System
mod fs_intel; // Task 072: Specialized Filesystem Intel
mod goal_seeding; // T305: Goal seeding from multi-step requests
mod guardrails; // State-aware guardrails for context drift (Task 011)
mod guardrails_refinement; // Guardrails refinement phase (Task 011)
mod hook_system; // Tasks 123, 124, 125: Extensible hook framework
mod hybrid_search; // Task 273: Hybrid Search Memory System With FTS And Vector Search
mod input_parser; // Task 013: Smart Input Prefixes And Command Modes
mod instruction_repair; // Task 391: Instruction-Level Repair And Result Recombiner
mod intel_narrative; // Narrative transformation for intel units
mod intel_narrative_advanced; // Advanced assessment narrative functions
mod intel_narrative_intent; // Intent analysis narrative functions
mod intel_narrative_planning; // Planning-related narrative functions
mod intel_narrative_steps; // Step-related narrative functions and helpers
mod intel_narrative_utils; // Shared narrative utility helpers
mod intel_trait; // Intel unit trait and interfaces
mod intel_units; // Migrated intel units (complexity, evidence, action, workflow)
mod json_error_handler; // JSON error handling with circuit breaker
mod json_grammar; // GBNF grammar loading and injection
mod json_parser; // Robust JSON parsing for intel unit outputs
mod json_parser_extract; // Extraction helpers for json_parser
mod json_repair; // Deterministic JSON repair pipeline (Task 378)
mod json_tuning; // JSON temperature tuning
mod llm_config;
mod llm_provider; // Task 278: Native Rust LLM API Client
mod model_capabilities; // Task 448: Model Capability Registry And Token Budgeting
mod token_counter; // Task 499: tiktoken-rs integration
mod logging;
mod markdown_ansi; // Markdown-to-ANSI terminal rendering
mod metrics;
mod models_api;
mod optimization;
mod optimization_eval;
mod optimization_tune;
mod orchestration;
mod orchestration_core;
mod orchestration_helpers;
mod orchestration_loop;
mod orchestration_loop_helpers; // Orchestration Loop - Helper Functions
mod orchestration_loop_reviewers;
mod orchestration_loop_verdicts;
mod orchestration_planning; // Planning Prior and Hierarchical Decomposition Module
mod orchestration_retry; // Retry orchestration and meta-review
mod paths;
mod permission_gate; // Task 117: Permission Gate for Destructive Commands
mod protected_paths; // Task 551: Protected path blocking
mod patch_executor; // Task 455: Patch Tool Multi-File Atomic
mod persistent_shell; // Task 288: Persistent Guarded Shell
mod profile_sets;
mod program;
mod program_policy;
mod program_policy_level;
mod program_policy_tests;
mod program_steps;
mod program_utils;
mod project_guidance;
mod project_init;
mod prompt_constants;
mod prompt_core; // Task 313: Protected Core System Prompt
mod pubsub; // Task 019: Generic Pub/Sub Broker
mod retry; // Task 570: Bounded retry with exponential backoff
mod sanitize; // Task 577: ANSI escape sanitization boundary
mod refinement;
mod reflection;
mod repo_explorer; // Task 196: Repo explorer and analyzer skill
mod routing;
mod routing_calc;
mod routing_config; // Routing configuration for confidence-based decisions
mod routing_infer;
mod routing_parse;
mod runtime_task;
mod safe_mode; // Task 272: Safe Mode Toggle System For Permission Levels
mod scenarios;
mod session;
mod session_cleanup;
mod session_display;
mod session_error;
mod session_flush; // Task 283: Session Transcript Flush
mod session_gc; // Task 282: Session Garbage Collector
mod session_hierarchy;
mod session_index; // Task 282: Session Index
mod session_paths;
mod session_store; // Task 277: SQLite Session Storage
mod session_write;
mod shell_preflight; // Task 116: Destructive Command Detection & Preflight
mod shutdown; // Task 017: Graceful Shutdown And Panic Recovery
mod skills;
mod snapshot;
mod stop_policy;
mod storage;
mod strategy; // Multi-strategy planning with fallback chains (Task 010)
mod streaming_tool_executor; // Task 115: Streaming Token Execution
mod system_monitor; // Right-side panel system resource monitor
mod task_steward; // Task 202: Project task steward skill
mod task_persistence; // Task 494: Session task persistence & _elma-tasks/
mod temp;
mod text_utils;
mod thinking_content;
mod tool_calling;
mod tool_discovery;
mod tool_loop;
mod tool_registry;
mod tool_result_storage; // Task 113: Tool Result Budget & Disk Persistence
mod tools; // Tools Module - tool caching, discovery, validation, and execution
mod trajectory; // Task 271: Trajectory Compression For Long-Running Sessions
mod trash;
mod tune;
mod tune_runtime;
mod tune_scenario;
mod tune_scenario_helpers;
mod tune_setup;
mod tune_summary;
mod tuning_support;
mod types;
mod types_api;
mod types_core;
mod types_core_impl;
mod types_hierarchy;
mod ui;
mod ui_status_thread; // Task 311: Persistent Status Thread
mod verification;
mod verification_evidence;
mod work_graph; // Task 389: Pyramid Work Graph
mod work_graph_bridge; // Task 494: Bridge graph → tasks → steps
mod workspace;
mod workspace_policy; // Task 441: Workspace ignore/protect policy
mod workspace_tree; // Task 169: Claude Code-style Terminal UI

pub(crate) use decomposition::*; // Task 023
pub(crate) use defaults::*;
pub(crate) use defaults_evidence::*; // JSON pipeline intel functions
pub(crate) use document_adapter::*; // Task 197: Document intelligence
pub(crate) use evaluation::*;
pub(crate) use execution::*;
pub(crate) use execution_ladder::*; // Execution ladder types and functions
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
pub(crate) use orchestration_helpers::*;
pub(crate) use config_validate::*;
pub(crate) use paths::*;
pub(crate) use profile_sets::*;
pub(crate) use protected_paths::*;
pub(crate) use retry::*;
pub(crate) use sanitize::*;
pub(crate) use program::*;
pub(crate) use project_guidance::*;
pub(crate) use project_init::*;
pub(crate) use prompt_constants::*;
pub(crate) use refinement::*;
pub(crate) use reflection::*;
pub(crate) use repo_explorer::*; // Task 196: Repo explorer
pub(crate) use routing::*;
pub(crate) use runtime_task::*;
pub(crate) use scenarios::*;
pub(crate) use session::*;
pub(crate) use session_display::*;
pub(crate) use session_flush::*; // Task 283: Session Transcript Flush
pub(crate) use skills::*;
pub(crate) use snapshot::*;
pub(crate) use stop_policy::*;
pub(crate) use storage::*;
pub(crate) use strategy::*; // Multi-strategy planning (Task 010)
pub(crate) use task_steward::*; // Task 202: Task steward
pub(crate) use text_utils::*;
pub(crate) use thinking_content::*;
pub(crate) use tune::*;
pub(crate) use tuning_support::*;
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
                config_cmd::handle_config_command(action);
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
