//! @efficiency-role: service-orchestrator
//!
//! Execution Profile System for shell and code-running tools.
//!
//! Provides configurable execution environments: local (default), restricted workspace,
//! containerized (future), and remote backends (future).

use crate::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

static ACTIVE_PROFILE: OnceLock<ExecutionProfile> = OnceLock::new();

/// Initialize the global execution profile.
pub(crate) fn init_execution_profile(profile: ExecutionProfile) {
    let _ = ACTIVE_PROFILE.set(profile);
}

/// Get the active execution profile.
pub(crate) fn get_execution_profile() -> Option<&'static ExecutionProfile> {
    ACTIVE_PROFILE.get()
}

/// Default local execution profile - uses current workdir, no restrictions.
pub(crate) fn default_local_profile() -> ExecutionProfile {
    ExecutionProfile {
        version: 1,
        name: "local".to_string(),
        backend: "local".to_string(),
        workdir_root: String::new(), // uses current workdir
        network_policy: "allowed".to_string(),
        writable_paths: vec![],
        readonly_paths: vec![],
        allowed_commands: vec![],
        denied_commands: vec![],
        env_passthrough: vec!["PATH".to_string(), "HOME".to_string(), "USER".to_string()],
    }
}

/// Restricted workspace profile - limits writes to workdir only.
pub(crate) fn restricted_profile(workdir: &PathBuf) -> ExecutionProfile {
    ExecutionProfile {
        version: 1,
        name: "restricted".to_string(),
        backend: "local".to_string(),
        workdir_root: workdir.display().to_string(),
        network_policy: "localhost_only".to_string(),
        writable_paths: vec![workdir.display().to_string()],
        readonly_paths: vec![],
        allowed_commands: vec![],
        denied_commands: vec![
            "rm".to_string(),
            "rmdir".to_string(),
            "dd".to_string(),
            "mkfs".to_string(),
        ],
        env_passthrough: vec!["PATH".to_string()],
    }
}

/// Check if a command is allowed under the given execution profile.
pub(crate) fn is_command_allowed(profile: &ExecutionProfile, command: &str) -> bool {
    // Check denied commands first
    let cmd_lower = command.to_lowercase();
    for denied in &profile.denied_commands {
        if cmd_lower.contains(&denied.to_lowercase()) {
            return false;
        }
    }

    // If allowed_commands is empty, all commands are allowed (that aren't denied)
    if profile.allowed_commands.is_empty() {
        return true;
    }

    // Otherwise, check if command is in allowed list
    for allowed in &profile.allowed_commands {
        if cmd_lower.contains(&allowed.to_lowercase()) {
            return true;
        }
    }

    false
}

/// Get the effective workdir based on execution profile.
pub(crate) fn get_effective_workdir(profile: &ExecutionProfile, default_workdir: &PathBuf) -> PathBuf {
    if profile.workdir_root.is_empty() {
        default_workdir.clone()
    } else {
        PathBuf::from(&profile.workdir_root)
    }
}

/// Check if a path is writable under the given execution profile.
pub(crate) fn is_path_writable(profile: &ExecutionProfile, path: &PathBuf) -> bool {
    // If writable_paths is empty, all paths are writable
    if profile.writable_paths.is_empty() {
        return true;
    }

    let path_str = path.display().to_string();
    for writable in &profile.writable_paths {
        if path_str.starts_with(writable) {
            return true;
        }
    }

    false
}

/// Load execution profile from config file.
pub(crate) fn load_execution_profile(config_root: &str, profile_name: &str) -> Result<ExecutionProfile> {
    let path = std::path::Path::new(config_root).join("execution_profiles.toml");
    if !path.exists() {
        // Return default local profile if no config exists
        return Ok(default_local_profile());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;

    let profiles: ExecutionProfilesFile = toml::from_str(&content)
        .context("parse execution_profiles.toml")?;

    for profile in profiles.profiles {
        if profile.name == profile_name {
            return Ok(profile);
        }
    }

    // Fall back to local default if profile not found
    Ok(default_local_profile())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ExecutionProfilesFile {
    profiles: Vec<ExecutionProfile>,
}
