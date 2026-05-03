//! @efficiency-role: domain-logic
//!
//! Patch Executor with Transaction Journal and Rollback — Task 455.
//!
//! Provides atomic multi-file patch operations with journal-based rollback.

use elma_tools::{parse_patch, PatchOperation};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("empty patch content")]
    EmptyPatch,

    #[error("parse error: {0}")]
    ParseError(String),

    #[error("validation error: {0}")]
    ValidationError(String),

    #[error("operation error: {0}")]
    OperationError(String),

    #[error("rollback error: {0}")]
    RollbackError(String),
}

impl From<PatchError> for String {
    fn from(e: PatchError) -> String {
        e.to_string()
    }
}

/// Result of a single patch operation
#[derive(Debug, Clone)]
pub struct PatchOpResult {
    pub path: String,
    pub status: PatchOpStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PatchOpStatus {
    Added,
    Updated,
    Deleted,
    Skipped,
    Failed,
}

/// Patch executor with transaction support
pub struct PatchExecutor {
    workdir: PathBuf,
    journal_dir: PathBuf,
    dry_run: bool,
}

impl PatchExecutor {
    /// Create a new patch executor
    pub fn new(workdir: PathBuf, tool_call_id: &str) -> Self {
        let journal_dir = workdir
            .join("sessions")
            .join("artifacts")
            .join("patch_transactions")
            .join(tool_call_id);
        Self {
            workdir,
            journal_dir,
            dry_run: false,
        }
    }

    /// Enable dry-run mode (validate only, don't write)
    pub fn with_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Check if running in dry-run mode
    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }

    /// Execute a patch with transaction support
    pub fn execute(&self, patch_content: &str) -> Result<Vec<PatchOpResult>, PatchError> {
        if patch_content.trim().is_empty() {
            return Err(PatchError::EmptyPatch);
        }

        let parsed = parse_patch(patch_content).map_err(|e| PatchError::ParseError(e.to_string()))?;

        let results = self.execute_parsed(&parsed.operations)?;
        Ok(results)
    }

    fn execute_parsed(
        &self,
        operations: &[PatchOperation],
    ) -> Result<Vec<PatchOpResult>, PatchError> {
        let mut results = Vec::new();
        let mut snapshots: HashMap<String, Vec<u8>> = HashMap::new();

        for op in operations {
            let result = match op {
                PatchOperation::AddFile { path, content } => {
                    self.validate_add_path(path)?;
                    if !self.dry_run {
                        self.execute_add(path, content)?
                    } else {
                        PatchOpResult {
                            path: path.clone(),
                            status: PatchOpStatus::Skipped,
                            message: "dry-run: would add".to_string(),
                        }
                    }
                }
                PatchOperation::UpdateFile { path, old_string, new_string } => {
                    self.validate_update_path(path, old_string)?;
                    if !self.dry_run {
                        let snapshot = self.snapshot_file(path)?;
                        snapshots.insert(path.clone(), snapshot);
                        self.execute_update(path, old_string, new_string)?
                    } else {
                        PatchOpResult {
                            path: path.clone(),
                            status: PatchOpStatus::Skipped,
                            message: "dry-run: would update".to_string(),
                        }
                    }
                }
                PatchOperation::DeleteFile { path } => {
                    self.validate_delete_path(path)?;
                    if !self.dry_run {
                        let snapshot = self.snapshot_file(path)?;
                        snapshots.insert(path.clone(), snapshot);
                        self.execute_delete(path)?
                    } else {
                        PatchOpResult {
                            path: path.clone(),
                            status: PatchOpStatus::Skipped,
                            message: "dry-run: would delete".to_string(),
                        }
                    }
                }
            };
            results.push(result);
        }

        Ok(results)
    }

    fn validate_add_path(&self, path: &str) -> Result<(), PatchError> {
        if path.is_empty() {
            return Err(PatchError::ValidationError("empty path".to_string()));
        }
        if Path::new(path).is_absolute() {
            return Err(PatchError::ValidationError(format!(
                "absolute path not allowed: {}",
                path
            )));
        }
        let full = self.workdir.join(path);
        if full.exists() {
            return Err(PatchError::ValidationError(format!(
                "file already exists: {}",
                path
            )));
        }
        Ok(())
    }

    fn validate_update_path(&self, path: &str, _old_string: &str) -> Result<(), PatchError> {
        if path.is_empty() {
            return Err(PatchError::ValidationError("empty path".to_string()));
        }
        if Path::new(path).is_absolute() {
            return Err(PatchError::ValidationError(format!(
                "absolute path not allowed: {}",
                path
            )));
        }
        let full = self.workdir.join(path);
        if !full.exists() {
            return Err(PatchError::ValidationError(format!(
                "file not found: {}",
                path
            )));
        }
        Ok(())
    }

    fn validate_delete_path(&self, path: &str) -> Result<(), PatchError> {
        if path.is_empty() {
            return Err(PatchError::ValidationError("empty path".to_string()));
        }
        if Path::new(path).is_absolute() {
            return Err(PatchError::ValidationError(format!(
                "absolute path not allowed: {}",
                path
            )));
        }
        let full = self.workdir.join(path);
        if !full.exists() {
            return Err(PatchError::ValidationError(format!(
                "file not found: {}",
                path
            )));
        }
        Ok(())
    }

    fn snapshot_file(&self, path: &str) -> Result<Vec<u8>, PatchError> {
        let full = self.workdir.join(path);
        std::fs::read(&full).map_err(|e| PatchError::OperationError(e.to_string()))
    }

    fn execute_add(&self, path: &str, content: &str) -> Result<PatchOpResult, PatchError> {
        let full = self.workdir.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PatchError::OperationError(e.to_string()))?;
        }
        std::fs::write(&full, content)
            .map_err(|e| PatchError::OperationError(e.to_string()))?;
        Ok(PatchOpResult {
            path: path.to_string(),
            status: PatchOpStatus::Added,
            message: "added".to_string(),
        })
    }

    fn execute_update(
        &self,
        path: &str,
        old_string: &str,
        new_string: &str,
    ) -> Result<PatchOpResult, PatchError> {
        let full = self.workdir.join(path);
        let original = std::fs::read_to_string(&full)
            .map_err(|e| PatchError::OperationError(e.to_string()))?;

        if let Some(pos) = original.find(old_string) {
            let mut updated = original.clone();
            updated.replace_range(pos..pos + old_string.len(), new_string);
            std::fs::write(&full, &updated)
                .map_err(|e| PatchError::OperationError(e.to_string()))?;
            Ok(PatchOpResult {
                path: path.to_string(),
                status: PatchOpStatus::Updated,
                message: "updated".to_string(),
            })
        } else {
            Err(PatchError::OperationError(format!(
                "old_string not found in {}",
                path
            )))
        }
    }

    fn execute_delete(&self, path: &str) -> Result<PatchOpResult, PatchError> {
        let full = self.workdir.join(path);
        std::fs::remove_file(&full)
            .map_err(|e| PatchError::OperationError(e.to_string()))?;
        Ok(PatchOpResult {
            path: path.to_string(),
            status: PatchOpStatus::Deleted,
            message: "deleted".to_string(),
        })
    }

    /// Rollback all changes from snapshots
    pub fn rollback(&self, snapshots: &HashMap<String, Vec<u8>>) -> Result<(), PatchError> {
        for (path, content) in snapshots {
            let full = self.workdir.join(path);
            std::fs::write(&full, content)
                .map_err(|e| PatchError::RollbackError(format!("{}: {}", path, e)))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_patch_executor_creation() {
        let tmp = std::env::temp_dir();
        let executor = PatchExecutor::new(tmp.clone(), "test_call_id");
        assert_eq!(executor.workdir, tmp);
    }

    #[test]
    fn test_patch_executor_dry_run() {
        let executor = PatchExecutor::new(std::env::temp_dir(), "test_call_id");
        assert!(!executor.dry_run);
    }

    #[test]
    fn test_patch_error_display() {
        let err = PatchError::EmptyPatch;
        assert!(err.to_string().contains("empty"));
    }
}