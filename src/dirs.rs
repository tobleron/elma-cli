//! @efficiency-role: domain-logic
//!
//! Cross-platform path resolution for Elma.
//!
//! Wraps `directories::ProjectDirs` to provide stable paths for:
//! - config_dir: elma.toml, profiles
//! - cache_dir: model responses, embeddings
//! - data_dir: sessions, skills

use anyhow::{Context, Result};
use directories::ProjectDirs;
use std::fs;
use std::path::{Path, PathBuf};

pub struct ElmaPaths {
    config_dir: PathBuf,
    cache_dir: PathBuf,
    data_dir: PathBuf,
}

impl ElmaPaths {
    /// Initialize paths using system standards
    pub fn new() -> Option<Self> {
        let proj = ProjectDirs::from("rs", "elma", "elma-cli")?;
        Some(Self {
            config_dir: proj.config_dir().to_path_buf(),
            cache_dir: proj.cache_dir().to_path_buf(),
            data_dir: proj.data_dir().to_path_buf(),
        })
    }

    /// Get the configuration directory
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get the data directory
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Ensure all directories exist on the filesystem
    pub fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.config_dir).context("Failed to create elma config directory")?;
        fs::create_dir_all(&self.cache_dir).context("Failed to create elma cache directory")?;
        fs::create_dir_all(&self.data_dir).context("Failed to create elma data directory")?;
        Ok(())
    }

    /// Get path to elma.toml
    pub fn elma_toml(&self) -> PathBuf {
        self.config_dir.join("elma.toml")
    }

    /// Get path to profiles directory
    pub fn profiles_dir(&self) -> PathBuf {
        self.config_dir.join("profiles")
    }

    /// Get path to sessions directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.data_dir.join("sessions")
    }
}
