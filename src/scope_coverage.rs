//! @efficiency-role: domain-logic
//!
//! Scope Coverage Ledger — Task 764.
//!
//! Tracks required coverage items discovered from the user's request
//! (files, directories, documents) and marks them as covered/skipped/failed
//! as tools execute. Finalization checks the ledger before claiming completion.

use crate::*;
use std::path::Path;

/// Status of a single coverage item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum CoverageStatus {
    Pending,
    Covered,
    Skipped,
    Failed,
}

impl std::fmt::Display for CoverageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverageStatus::Pending => write!(f, "pending"),
            CoverageStatus::Covered => write!(f, "covered"),
            CoverageStatus::Skipped => write!(f, "skipped"),
            CoverageStatus::Failed => write!(f, "failed"),
        }
    }
}

/// A single item that needs coverage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CoverageItem {
    /// The path or identifier of the item to cover.
    pub item: String,
    /// What kind of item this is: "file", "directory", "document", "search"
    pub kind: String,
    /// Current coverage status.
    pub status: CoverageStatus,
}

/// Task 764: Per-session scope coverage ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScopeCoverageLedger {
    pub items: Vec<CoverageItem>,
}

impl ScopeCoverageLedger {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Register items from a discovery result (e.g., glob, ls, search).
    pub fn register_items(&mut self, items: &[String], kind: &str) {
        for item in items {
            if !self.items.iter().any(|i| i.item == *item) {
                self.items.push(CoverageItem {
                    item: item.clone(),
                    kind: kind.to_string(),
                    status: CoverageStatus::Pending,
                });
            }
        }
    }

    /// Register a bounded scope from a request (e.g., all files under a directory).
    pub fn register_bounded_set(&mut self, paths: &[String], kind: &str) {
        for path in paths {
            if !self.items.iter().any(|i| i.item == *path) {
                self.items.push(CoverageItem {
                    item: path.clone(),
                    kind: kind.to_string(),
                    status: CoverageStatus::Pending,
                });
            }
        }
    }

    /// Mark an item as covered (read, searched, processed).
    pub fn mark_covered(&mut self, item: &str) {
        for entry in &mut self.items {
            if entry.item == item && entry.status == CoverageStatus::Pending {
                entry.status = CoverageStatus::Covered;
            }
        }
    }

    /// Mark an item as skipped (user scope, not relevant).
    pub fn mark_skipped(&mut self, item: &str) {
        for entry in &mut self.items {
            if entry.item == item && entry.status == CoverageStatus::Pending {
                entry.status = CoverageStatus::Skipped;
            }
        }
    }

    /// Mark an item as failed (could not be read/processed).
    pub fn mark_failed(&mut self, item: &str) {
        for entry in &mut self.items {
            if entry.item == item {
                entry.status = CoverageStatus::Failed;
            }
        }
    }

    /// Whether all items are in a terminal state (not pending).
    pub fn all_terminal(&self) -> bool {
        !self.items.is_empty()
            && self.items.iter().all(|i| i.status != CoverageStatus::Pending)
    }

    /// Whether there are any pending items.
    pub fn has_pending(&self) -> bool {
        self.items.iter().any(|i| i.status == CoverageStatus::Pending)
    }

    /// Count of items by status.
    pub fn count_by_status(&self, status: CoverageStatus) -> usize {
        self.items.iter().filter(|i| i.status == status).count()
    }

    /// Total items registered.
    pub fn total(&self) -> usize {
        self.items.len()
    }

    /// Render a coverage summary for the transcript.
    pub fn render_summary(&self) -> String {
        if self.items.is_empty() {
            return "No coverage items registered.".to_string();
        }
        let covered = self.count_by_status(CoverageStatus::Covered);
        let pending = self.count_by_status(CoverageStatus::Pending);
        let skipped = self.count_by_status(CoverageStatus::Skipped);
        let failed = self.count_by_status(CoverageStatus::Failed);
        let total = self.items.len();
        format!(
            "Coverage: {}/{} covered ({} pending, {} skipped, {} failed)",
            covered, total, pending, skipped, failed
        )
    }

    /// Render detailed item status.
    pub fn render_detailed(&self) -> String {
        if self.items.is_empty() {
            return "No coverage items.".to_string();
        }
        let mut lines: Vec<String> = Vec::new();
        lines.push("Coverage items:".to_string());
        for item in &self.items {
            lines.push(format!("  [{}] {} ({})", item.status, item.item, item.kind));
        }
        lines.join("\n")
    }

    /// Persist the ledger to session storage.
    pub fn persist(&self, session_root: &Path) {
        let dir = session_root.join("coverage");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("coverage.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, &json);
        }
    }
}

impl Default for ScopeCoverageLedger {
    fn default() -> Self {
        Self::new()
    }
}

// ── tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_ledger() {
        let ledger = ScopeCoverageLedger::new();
        assert_eq!(ledger.total(), 0);
        assert!(!ledger.has_pending());
        assert!(!ledger.all_terminal());
    }

    #[test]
    fn test_register_items() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string(), "b.md".to_string()], "file");
        assert_eq!(ledger.total(), 2);
        assert!(ledger.has_pending());
    }

    #[test]
    fn test_register_items_dedup() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string()], "file");
        ledger.register_items(&["a.md".to_string()], "file");
        assert_eq!(ledger.total(), 1);
    }

    #[test]
    fn test_mark_covered() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string()], "file");
        ledger.mark_covered("a.md");
        assert_eq!(ledger.count_by_status(CoverageStatus::Covered), 1);
        assert!(!ledger.has_pending());
    }

    #[test]
    fn test_mark_skipped() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string()], "file");
        ledger.mark_skipped("a.md");
        assert_eq!(ledger.count_by_status(CoverageStatus::Skipped), 1);
    }

    #[test]
    fn test_mark_failed() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string()], "file");
        ledger.mark_failed("a.md");
        assert_eq!(ledger.count_by_status(CoverageStatus::Failed), 1);
    }

    #[test]
    fn test_all_terminal() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string(), "b.md".to_string()], "file");
        assert!(!ledger.all_terminal());
        ledger.mark_covered("a.md");
        ledger.mark_skipped("b.md");
        assert!(ledger.all_terminal());
    }

    #[test]
    fn test_render_summary() {
        let mut ledger = ScopeCoverageLedger::new();
        let summary = ledger.render_summary();
        assert!(summary.contains("No coverage items"));
        ledger.register_items(&["a.md".to_string()], "file");
        let summary = ledger.render_summary();
        assert!(summary.contains("0/1 covered"));
        assert!(summary.contains("1 pending"));
    }

    #[test]
    fn test_render_detailed() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string()], "file");
        ledger.mark_covered("a.md");
        let detail = ledger.render_detailed();
        assert!(detail.contains("covered"));
        assert!(detail.contains("a.md"));
    }

    #[test]
    fn test_persist() {
        let tmp = std::env::temp_dir().join("coverage_test");
        let _ = std::fs::create_dir_all(&tmp);
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_items(&["a.md".to_string()], "file");
        ledger.persist(&tmp);
        assert!(tmp.join("coverage").join("coverage.json").exists());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_register_bounded_set() {
        let mut ledger = ScopeCoverageLedger::new();
        ledger.register_bounded_set(
            &[
                "doc1.md".to_string(),
                "doc2.md".to_string(),
                "doc3.md".to_string(),
            ],
            "document",
        );
        assert_eq!(ledger.total(), 3);
    }
}
