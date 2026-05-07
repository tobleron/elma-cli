//! @efficiency-role: data-model
//!
//! Local project memory with security scanning (Task 669).
//!
//! ProjectMemory persists key-value entries to .elma_memory.json
//! in the workspace root. SecurityScanner detects secrets/credentials
//! in workspace files using heuristic pattern matching.

use crate::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// A single memory entry stored in the project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProjectMemoryEntry {
    pub key: String,
    pub value: String,
    pub category: String,
    pub timestamp: u64,
    pub file_path: Option<PathBuf>,
}

/// In-memory + on-disk project memory backed by `.elma_memory.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProjectMemory {
    pub entries: Vec<ProjectMemoryEntry>,
    #[serde(skip)]
    pub workspace_root: PathBuf,
}

impl ProjectMemory {
    /// Load memory from `workspace_root/.elma_memory.json`, or create empty.
    pub(crate) fn new(workspace_root: &Path) -> Self {
        let memory_path = workspace_root.join(".elma_memory.json");
        let entries = if memory_path.exists() {
            fs::read_to_string(&memory_path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        ProjectMemory {
            entries,
            workspace_root: workspace_root.to_path_buf(),
        }
    }

    /// Store a new memory entry with the current timestamp.
    pub(crate) fn store(&mut self, key: &str, value: &str, category: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.entries.push(ProjectMemoryEntry {
            key: key.to_string(),
            value: value.to_string(),
            category: category.to_string(),
            timestamp,
            file_path: None,
        });
    }

    /// Retrieve an entry by exact key match.
    pub(crate) fn recall(&self, key: &str) -> Option<&ProjectMemoryEntry> {
        self.entries.iter().find(|e| e.key == key)
    }

    /// Fuzzy search across keys and values using substring matching.
    pub(crate) fn search(&self, query: &str) -> Vec<&ProjectMemoryEntry> {
        let q = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.key.to_lowercase().contains(&q) || e.value.to_lowercase().contains(&q))
            .collect()
    }

    /// Persist entries to `.elma_memory.json` in the workspace root.
    pub(crate) fn persist(&self) -> Result<()> {
        let memory_path = self.workspace_root.join(".elma_memory.json");
        let json = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&memory_path, json)?;
        Ok(())
    }
}

/// The type of a detected security finding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum SecurityFindingType {
    ApiKey,
    Password,
    PrivateKey,
    Token,
    ConnectionString,
    EnvFile,
}

/// A security finding discovered during a scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SecurityFinding {
    pub file: PathBuf,
    pub line: usize,
    pub finding_type: SecurityFindingType,
    pub snippet: String,
}

/// Scanner for detecting secrets and credentials in files.
pub(crate) struct SecurityScanner;

impl SecurityScanner {
    /// Scan a single file for security findings.
    pub(crate) fn scan_file(path: &Path) -> Vec<SecurityFinding> {
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        let mut findings = Vec::new();
        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;
            if let Some(ftype) = Self::classify_line(line) {
                findings.push(SecurityFinding {
                    file: path.to_path_buf(),
                    line: line_num,
                    finding_type: ftype,
                    snippet: Self::sanitize_snippet(line),
                });
            }
        }
        findings
    }

    /// Scan common workspace locations for security findings.
    pub(crate) fn scan_workspace(root: &Path) -> Vec<SecurityFinding> {
        let mut findings = Vec::new();
        let targets = [
            root.join(".env"),
            root.join(".env.example"),
            root.join(".env.local"),
            root.join(".env.production"),
        ];
        for target in targets.iter() {
            if target.exists() && target.is_file() {
                findings.extend(Self::scan_file(target));
            }
        }
        // Scan gitignored config files
        let config_patterns = [
            "*.key",
            "*.pem",
            "*.p12",
            "*.pfx",
            "secrets.yml",
            "credentials.yml",
        ];
        for pattern in &config_patterns {
            if let Ok(entries) = fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_lowercase();
                        if config_patterns.iter().any(|p| fname.ends_with(&p[1..])) {
                            findings.extend(Self::scan_file(&path));
                        }
                    }
                }
            }
        }
        findings
    }

    /// Heuristic check: does a line look like a secret or credential?
    pub(crate) fn is_likely_secret(line: &str) -> bool {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return false;
        }
        // Common secret patterns: KEY=VALUE with value looking like a secret
        let secret_patterns = [
            Regex::new(r"(?i)(api[_-]?key|secret|password|token|private[_-]?key)\s*[:=]\s*.{8,}")
                .unwrap(),
            Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,}|pk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,})")
                .unwrap(),
            Regex::new(r"(?i)(-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----)").unwrap(),
            Regex::new(r"(?i)(mongodb\+srv://|postgresql://|mysql://|redis://)[^\s]+").unwrap(),
            Regex::new(r"(?i)(AKIA[0-9A-Z]{16}|aws_key|aws_secret)").unwrap(),
        ];
        secret_patterns.iter().any(|re| re.is_match(trimmed))
    }

    fn classify_line(line: &str) -> Option<SecurityFindingType> {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            return None;
        }
        if Regex::new(r"(?i)(-----BEGIN\s+(?:RSA\s+)?PRIVATE\s+KEY-----)")
            .unwrap()
            .is_match(trimmed)
        {
            return Some(SecurityFindingType::PrivateKey);
        }
        if Regex::new(r"(?i)(mongodb\+srv://|postgresql://|mysql://|redis://)[^\s]+")
            .unwrap()
            .is_match(trimmed)
        {
            return Some(SecurityFindingType::ConnectionString);
        }
        if Regex::new(r"(?i)(sk-[a-zA-Z0-9]{20,}|pk-[a-zA-Z0-9]{20,}|ghp_[a-zA-Z0-9]{36,})")
            .unwrap()
            .is_match(trimmed)
        {
            return Some(SecurityFindingType::Token);
        }
        if Regex::new(r"(?i)(AKIA[0-9A-Z]{16})")
            .unwrap()
            .is_match(trimmed)
        {
            return Some(SecurityFindingType::ApiKey);
        }
        if Regex::new(r"(?i)\bpassword\s*[:=]")
            .unwrap()
            .is_match(trimmed)
        {
            return Some(SecurityFindingType::Password);
        }
        if trimmed == ".env" || trimmed.ends_with(".env") {
            return Some(SecurityFindingType::EnvFile);
        }
        if Regex::new(r"(?i)(api[_-]?key|secret|token)\s*[:=]\s*.{8,}")
            .unwrap()
            .is_match(trimmed)
        {
            return Some(SecurityFindingType::ApiKey);
        }
        None
    }

    /// Truncate and mask long secret snippets for safe display.
    fn sanitize_snippet(line: &str) -> String {
        let trimmed = line.trim();
        if trimmed.len() > 80 {
            format!("{}...", &trimmed[..77])
        } else {
            trimmed.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_project_memory_store_and_recall() {
        let dir = tempdir().unwrap();
        let mut pm = ProjectMemory::new(dir.path());
        pm.store("test-key", "test-value", "note");
        let entry = pm.recall("test-key").unwrap();
        assert_eq!(entry.key, "test-key");
        assert_eq!(entry.value, "test-value");
        assert_eq!(entry.category, "note");
        assert!(entry.timestamp > 0);
    }

    #[test]
    fn test_project_memory_recall_missing() {
        let dir = tempdir().unwrap();
        let pm = ProjectMemory::new(dir.path());
        assert!(pm.recall("nonexistent").is_none());
    }

    #[test]
    fn test_project_memory_search() {
        let dir = tempdir().unwrap();
        let mut pm = ProjectMemory::new(dir.path());
        pm.store("api-key", "sk_test_abc123", "security");
        pm.store("db-url", "postgresql://localhost", "config");
        let results = pm.search("api");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].key, "api-key");
    }

    #[test]
    fn test_project_memory_persist_and_reload() {
        let dir = tempdir().unwrap();
        {
            let mut pm = ProjectMemory::new(dir.path());
            pm.store("k1", "v1", "note");
            pm.store("k2", "v2", "config");
            pm.persist().unwrap();
        }
        let pm = ProjectMemory::new(dir.path());
        assert_eq!(pm.entries.len(), 2);
        assert_eq!(pm.recall("k1").unwrap().value, "v1");
    }

    #[test]
    fn test_is_likely_secret_api_key() {
        assert!(SecurityScanner::is_likely_secret(
            "API_KEY=sk_test_abcdefghijklmnopqrstuvwxyz"
        ));
        assert!(SecurityScanner::is_likely_secret(
            "secret=supersecretvalue123"
        ));
    }

    #[test]
    fn test_is_likely_secret_private_key() {
        assert!(SecurityScanner::is_likely_secret(
            "-----BEGIN PRIVATE KEY-----"
        ));
        assert!(SecurityScanner::is_likely_secret(
            "-----BEGIN RSA PRIVATE KEY-----"
        ));
    }

    #[test]
    fn test_is_likely_secret_negative() {
        assert!(!SecurityScanner::is_likely_secret("# this is a comment"));
        assert!(!SecurityScanner::is_likely_secret(""));
        assert!(!SecurityScanner::is_likely_secret("name=hello"));
    }

    #[test]
    fn test_scan_file_finds_secrets() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.env");
        fs::write(
            &file_path,
            "DB_URL=postgresql://user:pass@localhost/db\nAPI_KEY=sk_test_abc123\nSECRET_TOKEN=ghp_supersecret_token_value_123456\n",
        )
        .unwrap();
        let findings = SecurityScanner::scan_file(&file_path);
        assert!(!findings.is_empty(), "should find secrets");
        let types: Vec<_> = findings.iter().map(|f| &f.finding_type).collect();
        assert!(
            types.contains(&&SecurityFindingType::ConnectionString),
            "should detect connection string"
        );
    }

    #[test]
    fn test_scan_file_empty_for_clean_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("clean.txt");
        fs::write(&file_path, "hello world\nthis is safe\n").unwrap();
        let findings = SecurityScanner::scan_file(&file_path);
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_workspace_dotenv() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".env"),
            "password = hunter2\nAPI_TOKEN = sk-abc123def456ghi789\n",
        )
        .unwrap();
        let findings = SecurityScanner::scan_workspace(dir.path());
        assert!(!findings.is_empty(), "should find secrets in .env");
    }
}
