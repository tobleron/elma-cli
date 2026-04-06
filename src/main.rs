#![recursion_limit = "256"]
//! @efficiency-role: orchestrator

pub(crate) use anyhow::{Context, Result};
pub(crate) use clap::Parser;
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
#[cfg(test)]
mod app_chat_orchestrator_tests;
mod app_chat_patterns;
mod app_chat_trace;
mod auto_compact; // Task 114: Auto-Compact (Context Window Management)
mod command_budget; // Task 121: Command Budget & Rate Limiting
mod decomposition; // Task 023: Hierarchical decomposition
mod defaults;
mod defaults_core;
mod defaults_evidence;
mod defaults_evidence_core;
mod defaults_router;
mod evaluation;
mod evaluation_response;
mod evaluation_routing;
mod evaluation_workflow;
mod execution;
mod execution_ladder; // Execution ladder for minimum-sufficient orchestration
mod execution_steps;
mod execution_steps_compat;
mod execution_steps_edit;
mod execution_steps_read;
mod execution_steps_search;
#[cfg(test)]
mod execution_steps_selectors;
mod execution_steps_shell;
mod execution_steps_shell_exec;
mod execution_steps_shell_preflight;
mod formulas;
mod guardrails; // State-aware guardrails for context drift (Task 011)
mod guardrails_refinement; // Guardrails refinement phase (Task 011)
mod hook_system; // Tasks 123, 124, 125: Extensible hook framework
mod intel_narrative; // Narrative transformation for intel units
mod intel_narrative_planning; // Planning-related narrative functions
mod intel_narrative_steps; // Step-related narrative functions and helpers
mod intel_narrative_utils; // Shared narrative utility helpers
mod intel_trait; // Intel unit trait and interfaces
mod intel_units; // Migrated intel units (complexity, evidence, action, workflow)
mod json_error_handler; // JSON error handling with circuit breaker
mod json_grammar; // GBNF grammar loading and injection
mod json_parser; // Robust JSON parsing for intel unit outputs
mod json_parser_extract; // Extraction helpers for json_parser
mod json_tuning; // JSON temperature tuning
mod metrics;
mod models_api;
mod optimization;
mod optimization_eval;
mod optimization_tune;
mod orchestration;
mod orchestration_core;
mod orchestration_helpers;
mod orchestration_loop;
mod orchestration_loop_helpers;
mod orchestration_loop_reviewers;
mod orchestration_loop_verdicts;
mod orchestration_planning;
mod orchestration_retry;
mod orchestration_retry_tests;
mod paths;
mod permission_gate; // Task 117: Permission Gate for Destructive Commands
mod profile_sets;
mod program;
mod program_policy;
mod program_policy_level;
mod program_policy_tests;
mod program_steps;
mod program_utils;
mod prompt_constants;
mod refinement;
mod reflection;
mod routing;
mod routing_calc;
mod routing_infer;
mod routing_parse;
mod scenarios;
mod session;
mod session_error;
mod session_hierarchy;
mod session_paths;
mod session_seq;
mod session_write;
mod shell_preflight; // Task 116: Destructive Command Detection & Preflight
mod snapshot;
mod storage;
mod streaming_tool_executor; // Task 115: Streaming Tool Execution
mod strategy; // Multi-strategy planning with fallback chains (Task 010)
mod text_utils;
mod thinking_content;
mod tool_calling;
mod tool_discovery;
mod tool_loop;
mod tool_result_storage; // Task 113: Tool Result Budget & Disk Persistence
mod tools;
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
mod ui_chat;
mod ui_colors;
mod ui_context_bar; // Task 099: Context Window Usage Visualizer
mod ui_effort; // Task 107: Visual Effort Indicator
mod ui_interact; // Task 110: Inquire Interaction Integration
mod ui_layout; // Structured layout with borders and sections
mod ui_markdown; // Terminal markdown renderer
mod ui_progress; // Task 109: Indicatif Progress Integration
mod ui_spinner; // Task 101: Verb-Driven Loading Spinners
mod ui_state;
mod ui_syntax; // Task 111: Syntect Syntax Highlighting
mod ui_trace;
mod ui_tui; // Task 133: Ratatui TUI with Gruvbox + Persistent Status Bar
mod verification;
mod verification_evidence;
mod workspace;
mod workspace_tree;

pub(crate) use decomposition::*; // Task 023
pub(crate) use defaults::*;
pub(crate) use defaults_evidence::*; // JSON pipeline intel functions
pub(crate) use evaluation::*;
pub(crate) use execution::*;
pub(crate) use execution_ladder::*; // Execution ladder types and functions
pub(crate) use guardrails::*; // State-aware guardrails (Task 011)
pub(crate) use guardrails_refinement::*; // Guardrails refinement phase (Task 011)
pub(crate) use intel_trait::*; // Intel unit trait and interfaces
pub(crate) use intel_units::*; // Migrated intel units
pub(crate) use json_error_handler::*; // JSON error handling
pub(crate) use json_grammar::*; // GBNF grammar loading and injection
pub(crate) use json_tuning::*; // JSON temperature tuning
pub(crate) use metrics::*;
pub(crate) use models_api::*;
pub(crate) use optimization::*;
pub(crate) use orchestration::*;
pub(crate) use orchestration_helpers::*;
pub(crate) use paths::*;
pub(crate) use profile_sets::*;
pub(crate) use program::*;
pub(crate) use prompt_constants::*;
pub(crate) use refinement::*;
pub(crate) use reflection::*;
pub(crate) use routing::*;
pub(crate) use scenarios::*;
pub(crate) use session::*;
pub(crate) use snapshot::*;
pub(crate) use storage::*;
pub(crate) use strategy::*; // Multi-strategy planning (Task 010)
pub(crate) use text_utils::*;
pub(crate) use thinking_content::*;
pub(crate) use tune::*;
pub(crate) use tuning_support::*;
pub(crate) use types::*;
pub(crate) use ui::*;
pub(crate) use verification::*;
pub(crate) use workspace::*;

#[tokio::main]
async fn main() -> Result<()> {
    app::run().await
}
