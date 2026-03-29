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
mod app_chat;
mod decomposition;  // Task 023: Hierarchical decomposition
mod defaults;
mod evaluation;
mod evaluation_response;
mod evaluation_routing;
mod evaluation_workflow;
mod execution;
mod execution_steps;
mod intel;
mod metrics;
mod models_api;
mod optimization;
mod orchestration;
mod orchestration_helpers;
mod paths;
mod profile_sets;
mod program;
mod reflection;
mod refinement;
mod routing;
mod scenarios;
mod session;
mod snapshot;
mod storage;
mod text_utils;
mod thinking_content;
mod tool_discovery;
mod tune;
mod tune_runtime;
mod tune_scenario;
mod tune_setup;
mod tune_summary;
mod tuning_support;
mod types;
mod ui;
mod verification;
mod workspace;

pub(crate) use defaults::*;
pub(crate) use evaluation::*;
pub(crate) use execution::*;
pub(crate) use intel::*;
pub(crate) use metrics::*;
pub(crate) use models_api::*;
pub(crate) use optimization::*;
pub(crate) use decomposition::*;  // Task 023
pub(crate) use orchestration::*;
pub(crate) use paths::*;
pub(crate) use profile_sets::*;
pub(crate) use program::*;
pub(crate) use reflection::*;
pub(crate) use refinement::*;
pub(crate) use routing::*;
pub(crate) use scenarios::*;
pub(crate) use session::*;
pub(crate) use snapshot::*;
pub(crate) use storage::*;
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
