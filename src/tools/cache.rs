//! Tool Cache - Caches discovered tools to avoid rescanning on every startup

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCache {
    pub version: u32,
    pub cached_at: u64,
    pub path_hash: String,
    pub tools: Vec<CachedTool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTool {
    pub name: String,
    pub path: String,
    pub category: String,
    pub description: String,
}

impl ToolCache {
    pub fn new() -> Self {
        Self {
            version: 1,
            cached_at: current_timestamp(),
            path_hash: String::new(),
            tools: Vec::new(),
        }
    }

    pub fn load(cache_path: &Path) -> Option<Self> {
        if !cache_path.exists() {
            return None;
        }

        let content = fs::read_to_string(cache_path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self, cache_path: &Path) -> Result<()> {
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(cache_path, content)?;
        Ok(())
    }

    pub fn is_valid(&self, current_path_hash: &str) -> bool {
        // Cache valid for 7 days
        let age_seconds = current_timestamp() - self.cached_at;
        let seven_days = 7 * 24 * 60 * 60;

        self.path_hash == current_path_hash && age_seconds < seven_days
    }

    pub fn get_tools(&self) -> Vec<&CachedTool> {
        self.tools.iter().collect()
    }

    pub fn add_tool(&mut self, tool: CachedTool) {
        // Don't add duplicates
        if !self.tools.iter().any(|t| t.name == tool.name) {
            self.tools.push(tool);
        }
    }

    pub fn remove_missing(&mut self) {
        self.tools.retain(|tool| Path::new(&tool.path).exists());
    }
}

pub fn compute_path_hash() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();

    // Hash PATH environment variable
    if let Ok(path) = std::env::var("PATH") {
        path.hash(&mut hasher);
    }

    // Hash PATHEXT on Windows
    #[cfg(windows)]
    if let Ok(pathext) = std::env::var("PATHEXT") {
        pathext.hash(&mut hasher);
    }

    format!("{:x}", hasher.finish())
}

pub fn get_cache_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());

    PathBuf::from(home)
        .join(".elma-cli")
        .join("cache")
        .join("tool_registry.json")
}

pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn verify_tool_exists(path: &str) -> bool {
    Path::new(path).exists()
}

pub fn merge_caches(old_cache: &ToolCache, new_tools: Vec<CachedTool>) -> ToolCache {
    let mut merged = ToolCache {
        version: old_cache.version,
        cached_at: current_timestamp(),
        path_hash: old_cache.path_hash.clone(),
        tools: old_cache.tools.clone(),
    };

    for tool in new_tools {
        merged.add_tool(tool);
    }

    merged
}
