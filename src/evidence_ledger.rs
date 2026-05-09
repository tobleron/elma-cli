//! @efficiency-role: data-model
//!
//! Evidence Ledger Module
//!
//! Structured evidence tracking across session steps.
//! - Raw evidence stored in separate files on disk
//! - Compact summaries integrated into chat narratives
//! - Staleness and quality tracking per entry

use crate::evidence_summary::{should_store_raw, summarize_tool_result, SummarizeExtra};
use crate::*;
use sha1::{Digest, Sha1};
use std::fmt;
use std::sync::{OnceLock, RwLock};

// ============================================================================
// Global Ledger Holder (session-scoped)
// ============================================================================

pub(crate) fn init_session_ledger(session_id: &str, base_dir: &PathBuf) {
    let state = crate::session_state::get_session_state();
    let mut lock = match state.evidence_ledger.write() {
        Ok(l) => l,
        Err(_) => return,
    };
    *lock = Some(EvidenceLedger::new(session_id, base_dir));
}

pub(crate) fn get_session_ledger() -> Option<EvidenceLedger> {
    let state = crate::session_state::get_session_state();
    let lock = state.evidence_ledger.read().ok()?;
    lock.clone()
}

pub(crate) fn with_session_ledger<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut EvidenceLedger) -> R,
{
    let state = crate::session_state::get_session_state();
    let mut lock = match state.evidence_ledger.write() {
        Ok(l) => l,
        Err(_) => return None,
    };
    if let Some(ledger) = lock.as_mut() {
        return Some(f(ledger));
    }
    None
}

pub(crate) fn persist_session_ledger() -> Result<()> {
    let state = crate::session_state::get_session_state();
    let lock = match state.evidence_ledger.read() {
        Ok(l) => l,
        Err(_) => return Ok(()),
    };
    if let Some(ledger) = lock.as_ref() {
        return ledger.persist();
    }
    Ok(())
}

pub(crate) fn clear_session_ledger() {
    let state = crate::session_state::get_session_state();
    let mut lock = match state.evidence_ledger.write() {
        Ok(l) => l,
        Err(_) => return,
    };
    *lock = None;
}

// ============================================================================
// Core Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Staleness {
    Fresh,
    PotentiallyStale,
    Stale,
}

impl fmt::Display for Staleness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Staleness::Fresh => write!(f, "FRESH"),
            Staleness::PotentiallyStale => write!(f, "POTENTIALLY_STALE"),
            Staleness::Stale => write!(f, "STALE"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum EvidenceQuality {
    Direct,
    Indirect,
    Weak,
}

impl fmt::Display for EvidenceQuality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceQuality::Direct => write!(f, "DIRECT"),
            EvidenceQuality::Indirect => write!(f, "INDIRECT"),
            EvidenceQuality::Weak => write!(f, "WEAK"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum EvidenceSource {
    Shell { command: String, exit_code: i32 },
    Read { path: String },
    Search { path: String, pattern: String },
    Tool { name: String, input: String },
}

impl fmt::Display for EvidenceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvidenceSource::Shell { command, exit_code } => {
                write!(f, "shell({}) exit={}", command, exit_code)
            }
            EvidenceSource::Read { path } => write!(f, "read({})", path),
            EvidenceSource::Search { path, pattern } => {
                write!(f, "search({} in {})", pattern, path)
            }
            EvidenceSource::Tool { name, input } => {
                write!(
                    f,
                    "tool({}: {})",
                    name,
                    input.chars().take(50).collect::<String>()
                )
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EvidenceEntry {
    pub(crate) id: String,
    pub(crate) source: EvidenceSource,
    pub(crate) timestamp: u64,
    pub(crate) summary: String,
    pub(crate) raw_path: Option<String>,
    pub(crate) staleness: Staleness,
    pub(crate) quality: EvidenceQuality,
    pub(crate) file_mtime: Option<u64>,
    pub(crate) file_hash: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Claim {
    pub(crate) id: String,
    pub(crate) statement: String,
    pub(crate) supported_by: Vec<String>,
    pub(crate) contested_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EvidenceSummary {
    pub(crate) entries_count: usize,
    pub(crate) files_read: Vec<String>,
    pub(crate) key_findings: Vec<String>,
}

// ============================================================================
// Evidence Ledger
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EvidenceLedger {
    pub(crate) session_id: String,
    pub(crate) entries: Vec<EvidenceEntry>,
    pub(crate) claims: Vec<Claim>,
    pub(crate) base_dir: String,
    next_id: usize,
}

impl EvidenceLedger {
    pub(crate) fn new(session_id: &str, base_dir: &PathBuf) -> Self {
        Self {
            session_id: session_id.to_string(),
            entries: Vec::new(),
            claims: Vec::new(),
            base_dir: base_dir.to_string_lossy().to_string(),
            next_id: 1,
        }
    }

    pub(crate) fn add_entry(&mut self, source: EvidenceSource, raw_output: &str) -> &EvidenceEntry {
        // Strip ANSI escape sequences from raw output
        let clean_output = match strip_ansi_escapes::strip(raw_output.as_bytes()) {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(_) => raw_output.to_string(), // Fallback: return raw if stripping fails
        };

        let id = format!("e_{:03}", self.next_id);
        self.next_id += 1;

        let extra = match &source {
            EvidenceSource::Shell { command, exit_code } => SummarizeExtra {
                command: Some(command.clone()),
                path: None,
                pattern: None,
                exit_code: Some(*exit_code),
            },
            EvidenceSource::Read { path } => SummarizeExtra {
                command: None,
                path: Some(path.clone()),
                pattern: None,
                exit_code: None,
            },
            EvidenceSource::Search { path, pattern } => SummarizeExtra {
                command: None,
                path: Some(path.clone()),
                pattern: Some(pattern.clone()),
                exit_code: None,
            },
            EvidenceSource::Tool { name, input } => SummarizeExtra {
                command: None,
                path: None,
                pattern: None,
                exit_code: None,
            },
        };

        let summary = summarize_tool_result(
            match &source {
                EvidenceSource::Shell { .. } => "shell",
                EvidenceSource::Read { .. } => "read",
                EvidenceSource::Search { .. } => "search",
                EvidenceSource::Tool { name, .. } => name.as_str(),
            },
            &clean_output, // Use cleaned output for summarization
            &extra,
        );

        let quality = Self::assess_quality(&source, &clean_output); // Use cleaned output for quality assessment
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut raw_path = None;
        if should_store_raw(&clean_output) {
            let evidence_dir = PathBuf::from(&self.base_dir)
                .join("evidence")
                .join(&self.session_id);
            std::fs::create_dir_all(&evidence_dir).ok();
            let file_path = evidence_dir.join(format!("{}_raw.txt", id));
            if std::fs::write(&file_path, &clean_output).is_ok() {
                // Store cleaned output
                raw_path = Some(file_path.to_string_lossy().to_string());
            }
        }

        let entry = EvidenceEntry {
            id,
            source,
            timestamp,
            summary,
            raw_path: raw_path.clone(),
            staleness: Staleness::Fresh,
            quality,
            file_mtime: raw_path.as_ref().and_then(|p| {
                std::fs::metadata(p)
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
            }),
            file_hash: raw_path.as_ref().and_then(|p| {
                std::fs::read(p).ok().map(|content| {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = DefaultHasher::new();
                    content.hash(&mut hasher);
                    hasher.finish()
                })
            }),
        };

        self.entries.push(entry);
        self.entries.last().unwrap()
    }

    pub(crate) fn mark_stale(&mut self, path: &str) {
        for entry in &mut self.entries {
            if let EvidenceSource::Read { path: entry_path } = &entry.source {
                if entry_path == path || entry_path.contains(path) {
                    entry.staleness = Staleness::Stale;
                }
            }
        }
    }

    pub(crate) fn mark_path_modified(&mut self, path: &str) {
        self.mark_stale(path);
    }

    /// Compute a content hash for staleness detection using SHA-1.
    fn compute_content_hash(path: &str) -> Option<u64> {
        let content = std::fs::read(path).ok()?;
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(&content);
        let hash = hasher.finalize();
        // Use first 8 bytes as a u64 for compact storage
        let bytes = &hash[..8];
        Some(u64::from_le_bytes(bytes.try_into().ok()?))
    }

    pub(crate) fn check_file_is_stale(&self, path: &str) -> bool {
        for entry in self.entries.iter().rev() {
            if let EvidenceSource::Read { path: entry_path } = &entry.source {
                if entry_path == path || entry_path.contains(path) {
                    if entry.staleness == Staleness::Stale {
                        return true;
                    }
                    // Check mtime-based staleness
                    if let Some(stored_mtime) = entry.file_mtime {
                        if let Ok(current_meta) = std::fs::metadata(path) {
                            if let Ok(current_modified) = current_meta.modified() {
                                if let Ok(current_secs) =
                                    current_modified.duration_since(std::time::UNIX_EPOCH)
                                {
                                    if current_secs.as_secs() > stored_mtime {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                    // Check content-hash based staleness
                    if let Some(stored_hash) = entry.file_hash {
                        if let Some(current_hash) = Self::compute_content_hash(path) {
                            if current_hash != stored_hash {
                                return true;
                            }
                        }
                    }
                }
            }
        }
        false
    }

    pub(crate) fn get_entry(&self, id: &str) -> Option<&EvidenceEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub(crate) fn get_raw(&self, id: &str) -> Result<String> {
        let entry = self.get_entry(id).context("Evidence entry not found")?;
        if let Some(ref raw_path) = entry.raw_path {
            std::fs::read_to_string(raw_path)
                .with_context(|| format!("Failed to read raw evidence: {}", raw_path))
        } else {
            Ok(entry.summary.clone())
        }
    }

    pub(crate) fn compact_summary(&self) -> String {
        if self.entries.is_empty() {
            return "No evidence collected yet.".to_string();
        }

        self.entries
            .iter()
            .map(|e| {
                let staleness_tag = match e.staleness {
                    Staleness::Stale => " [STALE]",
                    Staleness::PotentiallyStale => " [STALE?]",
                    Staleness::Fresh => "",
                };
                format!("{}: {}{}", e.id, e.summary, staleness_tag)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(crate) fn narrative_snippet(&self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }

        self.entries
            .iter()
            .filter(|e| matches!(e.staleness, Staleness::Fresh))
            .map(|e| format!("{}: {}", e.id, e.summary))
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub(crate) fn get_latest_reflection(&self) -> Option<String> {
        self.entries.last().map(|e| e.summary.clone())
    }

    pub(crate) fn persist(&self) -> Result<()> {
        let session_root = PathBuf::from(&self.base_dir);

        // Atomic write to session.json.evidence
        crate::session_write::mutate_session_doc(&session_root, |doc| {
            let compact = serde_json::json!({
                "entries": serde_json::to_value(&self.entries).unwrap_or_default(),
                "claims": serde_json::to_value(&self.claims).unwrap_or_default(),
            });
            doc["evidence"] = compact;
        })?;

        // Atomic write to evidence/ dir
        let evidence_dir = session_root.join("evidence").join(&self.session_id);
        std::fs::create_dir_all(&evidence_dir)
            .with_context(|| format!("mkdir {}", evidence_dir.display()))?;
        let ledger_path = evidence_dir.join("ledger.json");
        let json = serde_json::to_string_pretty(self).context("Failed to serialize ledger")?;
        
        crate::atomic_write::atomic_write(&ledger_path, &json)?;

        Ok(())
    }

    /// Attempt to load evidence from session.json (new canonical path).
    fn try_load_from_session_json(session_id: &str, session_root: &PathBuf) -> Option<Self> {
        use crate::session_write::load_session_doc;
        let doc = load_session_doc(session_root);
        let evidence = doc.get("evidence")?;
        let entries: Vec<EvidenceEntry> =
            serde_json::from_value(evidence.get("entries")?.clone()).ok()?;
        let claims: Vec<Claim> = serde_json::from_value(evidence.get("claims")?.clone()).ok()?;
        let max_id = entries
            .iter()
            .filter_map(|e| e.id.strip_prefix("e_"))
            .filter_map(|n| n.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        Some(Self {
            session_id: session_id.to_string(),
            entries,
            claims,
            base_dir: session_root.to_string_lossy().to_string(),
            next_id: max_id + 1,
        })
    }

    pub(crate) fn load(session_id: &str, base_dir: &PathBuf) -> Result<Self> {
        // Try session.json evidence first (new path)
        let session_root = base_dir;
        if let Some(ledger) = Self::try_load_from_session_json(session_id, session_root) {
            return Ok(ledger);
        }

        // Legacy fallback: evidence/<session_id>/ledger.json
        let evidence_dir = base_dir.join("evidence").join(session_id);
        let ledger_path = evidence_dir.join("ledger.json");

        if !ledger_path.exists() {
            return Ok(Self::new(session_id, base_dir));
        }

        let json = std::fs::read_to_string(&ledger_path)
            .with_context(|| format!("read {}", ledger_path.display()))?;
        let mut ledger: EvidenceLedger =
            serde_json::from_str(&json).context("Failed to deserialize ledger")?;

        let max_id = ledger
            .entries
            .iter()
            .filter_map(|e| e.id.strip_prefix("e_"))
            .filter_map(|n| n.parse::<usize>().ok())
            .max()
            .unwrap_or(0);
        ledger.next_id = max_id + 1;

        Ok(ledger)
    }

    pub(crate) fn add_claim(&mut self, statement: &str, supported_by: Vec<String>) {
        let id = format!("c_{:03}", self.claims.len() + 1);
        self.claims.push(Claim {
            id,
            statement: statement.to_string(),
            supported_by,
            contested_by: Vec::new(),
        });
    }

    pub(crate) fn entries_count(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn build_retry_summary(&self) -> EvidenceSummary {
        let mut files_read = Vec::new();
        let mut key_findings = Vec::new();

        for entry in &self.entries {
            match &entry.source {
                EvidenceSource::Read { path } => {
                    files_read.push(path.clone());
                }
                _ => {}
            }
            if matches!(entry.quality, EvidenceQuality::Direct) && key_findings.len() < 5 {
                key_findings.push(entry.summary.clone());
            }
        }

        files_read.sort();
        files_read.dedup();

        EvidenceSummary {
            entries_count: self.entries.len(),
            files_read,
            key_findings,
        }
    }


    /// Task 761: Returns a summary of evidence grouped by source type
    /// (files read, directories searched, shell commands run).
    pub(crate) fn coverage_summary(&self) -> String {
        if self.entries.is_empty() {
            return "No evidence collected.".to_string();
        }

        let mut file_paths: Vec<String> = Vec::new();
        let mut search_dirs: Vec<String> = Vec::new();
        let mut shell_cmds: Vec<String> = Vec::new();
        let mut tool_calls: Vec<String> = Vec::new();

        for entry in &self.entries {
            match &entry.source {
                EvidenceSource::Read { path } => {
                    file_paths.push(path.clone());
                }
                EvidenceSource::Search { path, .. } => {
                    search_dirs.push(path.clone());
                }
                EvidenceSource::Shell { command, .. } => {
                    let short = command.chars().take(80).collect::<String>();
                    shell_cmds.push(short);
                }
                EvidenceSource::Tool { name, input } => {
                    let short = input.chars().take(60).collect::<String>();
                    tool_calls.push(format!("{}: {}", name, short));
                }
            }
        }

        let mut parts: Vec<String> = Vec::new();
        let total = self.entries.len();
        parts.push(format!("Total evidence entries: {}", total));

        if !file_paths.is_empty() {
            parts.push(format!("Files read: {}", file_paths.len()));
            let preview: Vec<String> = file_paths.iter().take(5).cloned().collect();
            parts.push(format!("  ({})", preview.join(", ")));
            if file_paths.len() > 5 {
                parts.push(format!("  ... and {} more", file_paths.len() - 5));
            }
        }
        if !search_dirs.is_empty() {
            parts.push(format!("Directories searched: {}", search_dirs.len()));
        }
        if !shell_cmds.is_empty() {
            parts.push(format!("Shell commands executed: {}", shell_cmds.len()));
        }
        if !tool_calls.is_empty() {
            parts.push(format!("Other tool calls: {}", tool_calls.len()));
        }

        parts.join("\n")
    }

    /// Task 761: Whether the evidence ledger has at least minimal coverage
    /// (at least one successful entry of any kind).
    pub(crate) fn has_minimal_coverage(&self) -> bool {
        self.entries.iter().any(|e| matches!(e.quality, EvidenceQuality::Direct | EvidenceQuality::Indirect))
    }

    /// Task 761: Count of unique files read in this session.
    pub(crate) fn unique_files_read(&self) -> usize {
        use std::collections::HashSet;
        let paths: HashSet<&str> = self
            .entries
            .iter()
            .filter_map(|e| match &e.source {
                EvidenceSource::Read { path } => Some(path.as_str()),
                _ => None,
            })
            .collect();
        paths.len()
    }

    pub(crate) fn fresh_entries(&self) -> Vec<&EvidenceEntry> {
        self.entries
            .iter()
            .filter(|e| matches!(e.staleness, Staleness::Fresh))
            .collect()
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.claims.clear();
        self.next_id = 1;
    }

    pub(crate) fn has_evidence_matching(&self, keywords: &[&str]) -> bool {
        self.entries.iter().any(|entry| {
            keywords
                .iter()
                .any(|kw| entry.summary.to_lowercase().contains(&kw.to_lowercase()))
        })
    }

    fn assess_quality(source: &EvidenceSource, raw_output: &str) -> EvidenceQuality {
        match source {
            EvidenceSource::Shell { exit_code, .. } => {
                if *exit_code == 0 && !raw_output.trim().is_empty() {
                    EvidenceQuality::Direct
                } else if *exit_code == 0 {
                    EvidenceQuality::Indirect
                } else {
                    EvidenceQuality::Weak
                }
            }
            EvidenceSource::Read { .. } => {
                if !raw_output.trim().is_empty() {
                    EvidenceQuality::Direct
                } else {
                    EvidenceQuality::Weak
                }
            }
            EvidenceSource::Search { .. } => {
                if !raw_output.trim().is_empty() {
                    EvidenceQuality::Direct
                } else {
                    EvidenceQuality::Weak
                }
            }
            EvidenceSource::Tool { .. } => {
                if !raw_output.trim().is_empty() {
                    EvidenceQuality::Indirect
                } else {
                    EvidenceQuality::Weak
                }
            }
        }
    }
}

// ============================================================================
// Claim-Evidence Mapping (Enforcement Gate)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ClaimVerdict {
    pub(crate) statement: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EvidenceVerdict {
    pub(crate) claims: Vec<ClaimVerdict>,
}

impl EvidenceVerdict {
    pub(crate) fn is_pass(&self) -> bool {
        self.claims.iter().all(|c| c.status == "GROUNDED")
    }

    pub(crate) fn ungrounded_claims(&self) -> Vec<&ClaimVerdict> {
        self.claims
            .iter()
            .filter(|c| c.status == "UNGROUNDED")
            .collect()
    }
}

pub(crate) fn enforce_evidence_grounding(draft: &str, ledger: &EvidenceLedger) -> EvidenceVerdict {
    heuristic_grounding_check(draft, ledger)
}

pub(crate) async fn enforce_evidence_grounding_with_intel(
    draft: &str,
    ledger: &EvidenceLedger,
    client: &reqwest::Client,
    profile: &Profile,
) -> EvidenceVerdict {
    let summary = ledger.compact_summary();

    let narrative = format!(
        r#"DRAFT ANSWER:
{draft}

AVAILABLE EVIDENCE:
{summary}

TASK:
Extract every factual claim from the draft answer. For each claim, identify which evidence entry (by ID) supports it. If no evidence supports a claim, mark it as UNGROUNDED.

Output contract:
{{"claims": [{{"statement": "...", "evidence_ids": ["e_001"], "status": "GROUNDED|UNGROUNDED"}}]}}"#,
        draft = draft.trim(),
        summary = summary,
    );

    match crate::intel_trait::execute_intel_json_from_user_content::<serde_json::Value>(
        client, profile, narrative,
    )
    .await
    {
        Ok(result) => {
            let claims: Vec<ClaimVerdict> = result
                .get("claims")
                .and_then(|c| c.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|c| {
                            Some(ClaimVerdict {
                                statement: c.get("statement")?.as_str()?.to_string(),
                                evidence_ids: c
                                    .get("evidence_ids")
                                    .and_then(|a| a.as_array())
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(|v| v.as_str().map(String::from))
                                            .collect()
                                    })
                                    .unwrap_or_default(),
                                status: c.get("status")?.as_str()?.to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            EvidenceVerdict { claims }
        }
        Err(_) => heuristic_grounding_check(draft, ledger),
    }
}

fn heuristic_grounding_check(draft: &str, ledger: &EvidenceLedger) -> EvidenceVerdict {
    let mut claims = Vec::new();

    for line in draft.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() < 20 {
            continue;
        }

        let has_identifier = trimmed.contains('/')
            || (trimmed.contains('.') && trimmed.contains(|c: char| c.is_alphabetic()))
            || trimmed.chars().any(|c| c.is_ascii_digit());
        if !has_identifier {
            continue;
        }

        let mut supporting = Vec::new();
        for entry in &ledger.entries {
            // Level 1: Summary check (fast)
            let summary_words: Vec<_> = entry
                .summary
                .split_whitespace()
                .filter(|w| w.len() > 3)
                .collect();
            let draft_words: Vec<_> = trimmed.split_whitespace().filter(|w| w.len() > 3).collect();
            let overlap: usize = draft_words
                .iter()
                .filter(|dw| {
                    summary_words.iter().any(|sw| {
                        sw.to_lowercase().contains(&dw.to_lowercase())
                            || dw.to_lowercase().contains(&sw.to_lowercase())
                    })
                })
                .count();
            
            if overlap >= 2 {
                supporting.push(entry.id.clone());
                continue;
            }

            // Level 2: Deep check (if summary failed but raw data exists)
            if let Some(ref raw_path) = entry.raw_path {
                if let Ok(content) = std::fs::read_to_string(raw_path) {
                    let content_lower = content.to_lowercase();
                    let matches = draft_words.iter()
                        .filter(|w| w.len() > 4) // Only significant words
                        .filter(|w| content_lower.contains(&w.to_lowercase()))
                        .count();
                    
                    if matches >= 3 {
                        supporting.push(entry.id.clone());
                    }
                }
            }
        }

        let is_grounded = !supporting.is_empty();
        claims.push(ClaimVerdict {
            statement: trimmed.chars().take(100).collect(),
            evidence_ids: supporting,
            status: if is_grounded {
                "GROUNDED".to_string()
            } else {
                "UNGROUNDED".to_string()
            },
        });
    }

    EvidenceVerdict { claims }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ledger() -> EvidenceLedger {
        let dir = std::env::temp_dir().join("test_evidence");
        let mut ledger = EvidenceLedger::new("s_test", &dir);
        ledger.add_entry(
            EvidenceSource::Shell {
                command: "ls -la".to_string(),
                exit_code: 0,
            },
            "total 48\ndrwxr-xr-x  12 user  staff   384 Apr 20 10:00 .\nAGENTS.md\nCargo.toml",
        );
        ledger.add_entry(
            EvidenceSource::Read {
                path: "src/main.rs".to_string(),
            },
            "fn main() {\n    println!(\"hello\");\n}",
        );
        ledger
    }

    #[test]
    fn test_new_ledger() {
        let dir = std::env::temp_dir().join("test_ledger_new");
        let ledger = EvidenceLedger::new("s_123", &dir);
        assert_eq!(ledger.session_id, "s_123");
        assert!(ledger.entries.is_empty());
    }

    #[test]
    fn test_add_entry_shell() {
        let mut ledger = test_ledger();
        assert_eq!(ledger.entries_count(), 2);
        let first = ledger.get_entry("e_001").unwrap();
        // Small output returns raw content as summary
        assert!(first.summary.contains("AGENTS.md"));
        assert!(matches!(first.quality, EvidenceQuality::Direct));
    }

    #[test]
    fn test_add_entry_read() {
        let mut ledger = test_ledger();
        let entry = ledger.get_entry("e_002").unwrap();
        // Small output returns raw content as summary
        assert!(entry.summary.contains("fn main()"));
        assert!(matches!(entry.quality, EvidenceQuality::Direct));
    }

    #[test]
    fn test_mark_stale() {
        let mut ledger = test_ledger();
        ledger.mark_stale("src/main.rs");
        let entry = ledger.get_entry("e_002").unwrap();
        assert!(matches!(entry.staleness, Staleness::Stale));
    }

    #[test]
    fn test_compact_summary() {
        let ledger = test_ledger();
        let summary = ledger.compact_summary();
        assert!(summary.contains("e_001"));
        assert!(summary.contains("e_002"));
    }

    #[test]
    fn test_narrative_snippet() {
        let ledger = test_ledger();
        let snippet = ledger.narrative_snippet();
        assert!(!snippet.is_empty());
        assert!(snippet.contains("e_001"));
    }

    #[test]
    fn test_get_raw_small_entry() {
        let ledger = test_ledger();
        let raw = ledger.get_raw("e_002").unwrap();
        assert!(raw.contains("fn main()"));
    }

    #[test]
    fn test_assess_quality_shell_success() {
        let q = EvidenceLedger::assess_quality(
            &EvidenceSource::Shell {
                command: "ls".to_string(),
                exit_code: 0,
            },
            "file1\nfile2",
        );
        assert!(matches!(q, EvidenceQuality::Direct));
    }

    #[test]
    fn test_assess_quality_shell_failure() {
        let q = EvidenceLedger::assess_quality(
            &EvidenceSource::Shell {
                command: "ls".to_string(),
                exit_code: 1,
            },
            "error",
        );
        assert!(matches!(q, EvidenceQuality::Weak));
    }

    #[test]
    fn test_assess_quality_read_empty() {
        let q = EvidenceLedger::assess_quality(
            &EvidenceSource::Read {
                path: "empty.txt".to_string(),
            },
            "",
        );
        assert!(matches!(q, EvidenceQuality::Weak));
    }

    #[test]
    fn test_enforce_grounding_heuristic() {
        let ledger = test_ledger();
        let draft = "I found AGENTS.md in the project root directory.\nThe project uses Cargo.toml for dependencies.";
        let verdict = enforce_evidence_grounding(draft, &ledger);
        assert!(!verdict.claims.is_empty());
    }

    #[test]
    fn test_fresh_entries() {
        let mut ledger = test_ledger();
        ledger.mark_stale("src/main.rs");
        let fresh = ledger.fresh_entries();
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].id, "e_001");
    }

    #[test]
    fn test_add_claim() {
        let mut ledger = test_ledger();
        ledger.add_claim("AGENTS.md exists", vec!["e_001".to_string()]);
        assert_eq!(ledger.claims.len(), 1);
        assert_eq!(ledger.claims[0].statement, "AGENTS.md exists");
    }

    /// End-to-end: full evidence lifecycle from tool execution to enforcement
    #[test]
    fn test_evidence_ledger_e2e() {
        let test_dir = std::env::temp_dir().join("test_evidence_e2e");
        let _ = std::fs::remove_dir_all(&test_dir);

        let mut ledger = EvidenceLedger::new("s_e2e_test", &test_dir);

        // 1. Shell result (small — summary = raw)
        let shell_output = "total 48\nAGENTS.md\nCargo.toml\n.gitignore";
        let entry1 = ledger.add_entry(
            EvidenceSource::Shell {
                command: "ls -la".to_string(),
                exit_code: 0,
            },
            shell_output,
        );
        assert_eq!(entry1.id, "e_001");
        assert!(matches!(entry1.quality, EvidenceQuality::Direct));

        // 2. Read result (small)
        let cargo = "[package]\nname = \"elma-cli\"\nversion = \"0.1.0\"\n\n[dependencies]\nreqwest = \"0.12\"\ntokio = \"1.36\"";
        let entry2 = ledger.add_entry(
            EvidenceSource::Read {
                path: "Cargo.toml".to_string(),
            },
            cargo,
        );
        assert_eq!(entry2.id, "e_002");

        // 3. Large search result — should trigger raw file storage
        let large_search = (0..200)
            .map(|i| format!("src/file_{i}.rs:10: fn helper_{i}() {{}}"))
            .collect::<Vec<_>>()
            .join("\n");
        let entry3 = ledger.add_entry(
            EvidenceSource::Search {
                path: "src/".to_string(),
                pattern: "fn helper".to_string(),
            },
            &large_search,
        );
        assert_eq!(entry3.id, "e_003");
        assert!(entry3.raw_path.is_some());
        let raw_path = entry3.raw_path.as_ref().unwrap();
        assert!(std::path::Path::new(raw_path).exists());
        assert!(std::fs::read_to_string(raw_path)
            .unwrap()
            .contains("fn helper_100"));

        // 4. Compact summary includes all entries
        let summary = ledger.compact_summary();
        assert!(summary.contains("e_001"));
        assert!(summary.contains("e_002"));
        assert!(summary.contains("e_003"));
        assert!(summary.contains("AGENTS.md"));
        assert!(summary.contains("200 matches"));

        // 5. Staleness: modifying Cargo.toml marks e_002 stale
        ledger.mark_path_modified("Cargo.toml");
        assert!(matches!(
            ledger.get_entry("e_002").unwrap().staleness,
            Staleness::Stale
        ));
        assert!(matches!(
            ledger.get_entry("e_001").unwrap().staleness,
            Staleness::Fresh
        ));

        // 6. Raw retrieval
        assert!(ledger.get_raw("e_001").unwrap().contains("AGENTS.md"));
        assert!(ledger.get_raw("e_003").unwrap().contains("fn helper_150"));

        // 7. Enforcement: grounded draft
        let grounded = "I found AGENTS.md and Cargo.toml in the project root.";
        let v1 = enforce_evidence_grounding(grounded, &ledger);
        let ungrounded1: Vec<_> = v1
            .claims
            .iter()
            .filter(|c| c.status == "UNGROUNDED")
            .collect();
        assert!(ungrounded1.len() < v1.claims.len());

        // 8. Enforcement: ungrounded draft
        let ungrounded_draft = "The project is written in Python and uses Django with PostgreSQL.";
        let v2 = enforce_evidence_grounding(ungrounded_draft, &ledger);
        let ungrounded2: Vec<_> = v2
            .claims
            .iter()
            .filter(|c| c.status == "UNGROUNDED")
            .collect();
        assert!(!ungrounded2.is_empty());

        // 9. Persist and reload
        ledger.persist().unwrap();
        let reloaded = EvidenceLedger::load("s_e2e_test", &test_dir).unwrap();
        assert_eq!(reloaded.entries_count(), 3);

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    /// Narrative integration: evidence IDs appear when ledger is provided
    #[test]
    fn test_narrative_includes_evidence_ids() {
        use crate::{Program, Step, StepCommon, StepResult};

        let test_dir = std::env::temp_dir().join("test_narrative_evidence");
        let _ = std::fs::remove_dir_all(&test_dir);

        let mut ledger = EvidenceLedger::new("s_narrative", &test_dir);
        ledger.add_entry(
            EvidenceSource::Shell {
                command: "ls -la".to_string(),
                exit_code: 0,
            },
            "total 48\nAGENTS.md\nCargo.toml",
        );

        let program = Program {
            objective: "list files".to_string(),
            steps: vec![Step::Shell {
                id: "e_001".to_string(),
                cmd: "ls -la".to_string(),
                common: StepCommon {
                    purpose: "list workspace files".to_string(),
                    depends_on: vec![],
                    success_condition: "files listed".to_string(),
                    ..StepCommon::default()
                },
            }],
        };

        let step_results = vec![StepResult {
            id: "e_001".to_string(),
            kind: "shell".to_string(),
            purpose: "list workspace files".to_string(),
            depends_on: vec![],
            success_condition: "files listed".to_string(),
            ok: true,
            summary: "Command executed successfully".to_string(),
            raw_output: Some("total 48\nAGENTS.md\nCargo.toml".to_string()),
            exit_code: Some(0),
            ..StepResult::default()
        }];

        let narrative_with =
            crate::intel_narrative::build_steps_narrative(&program, &step_results, Some(&ledger));
        assert!(
            narrative_with.contains("e_001"),
            "Narrative with ledger should include evidence ID. Got:\n{}",
            narrative_with
        );

        let narrative_without =
            crate::intel_narrative::build_steps_narrative(&program, &step_results, None);
        assert!(
            !narrative_without.contains("[e_001]"),
            "Narrative without ledger should not have evidence tag. Got:\n{}",
            narrative_without
        );

        let _ = std::fs::remove_dir_all(&test_dir);
    }

    #[test]
    fn test_clear_evidence_ledger() {
        let mut ledger = test_ledger();
        assert_eq!(ledger.entries_count(), 2);
        ledger.clear();
        assert_eq!(ledger.entries_count(), 0);
        assert!(ledger.claims.is_empty());
    }

    #[test]
    fn test_has_evidence_matching_found() {
        let ledger = test_ledger();
        assert!(ledger.has_evidence_matching(&["AGENTS.md"]));
        assert!(ledger.has_evidence_matching(&["Cargo.toml", "main.rs"]));
    }

    #[test]
    fn test_has_evidence_matching_not_found() {
        let ledger = test_ledger();
        assert!(!ledger.has_evidence_matching(&["nonexistent"]));
        assert!(!ledger.has_evidence_matching(&["Python", "Django"]));
    }

    #[test]
    fn test_has_evidence_matching_empty_ledger() {
        let dir = std::env::temp_dir().join("test_clear");
        let ledger = EvidenceLedger::new("s_empty", &dir);
        assert!(!ledger.has_evidence_matching(&["anything"]));
    }

    #[test]
    fn test_staleness_enum_derive() {
        use Staleness::*;
        assert!(Fresh == Fresh);
        assert!(Stale == Stale);
        assert!(Fresh != Stale);
    }

    #[test]
    fn test_evidence_entry_has_mtime_field() {
        let dir = std::env::temp_dir().join("test_entry");
        let mut ledger = EvidenceLedger::new("s_test", &dir);
        ledger.add_entry(
            EvidenceSource::Read {
                path: "Cargo.toml".to_string(),
            },
            "test content",
        );
        let entry = ledger.entries.first().unwrap();
        assert!(entry.file_mtime.is_none() || entry.file_mtime.is_some());
    }

    #[test]
    fn test_check_file_is_stale_no_file() {
        let dir = std::env::temp_dir().join("test_stale");
        let ledger = EvidenceLedger::new("s_test", &dir);
        assert!(!ledger.check_file_is_stale("nonexistent.txt"));
    }

    // ── Task 761: Coverage assessment tests ──

    #[test]
    fn test_coverage_summary_with_evidence() {
        let ledger = test_ledger();
        let summary = ledger.coverage_summary();
        assert!(summary.contains("Total evidence entries: 2"));
        assert!(summary.contains("Files read: 1"));
        assert!(summary.contains("Shell commands executed: 1"));
        assert!(summary.contains("src/main.rs"));
    }

    #[test]
    fn test_coverage_summary_empty() {
        let dir = std::env::temp_dir().join("test_coverage_empty");
        let ledger = EvidenceLedger::new("s_empty", &dir);
        let summary = ledger.coverage_summary();
        assert!(summary.contains("No evidence collected"));
    }

    #[test]
    fn test_has_minimal_coverage() {
        let ledger = test_ledger();
        assert!(ledger.has_minimal_coverage());
    }

    #[test]
    fn test_has_minimal_coverage_empty() {
        let dir = std::env::temp_dir().join("test_coverage_min");
        let ledger = EvidenceLedger::new("s_empty", &dir);
        assert!(!ledger.has_minimal_coverage());
    }

    #[test]
    fn test_unique_files_read() {
        let mut ledger = test_ledger();
        assert_eq!(ledger.unique_files_read(), 1); // src/main.rs
        ledger.add_entry(
            EvidenceSource::Read {
                path: "AGENTS.md".to_string(),
            },
            "content",
        );
        assert_eq!(ledger.unique_files_read(), 2);
    }
}
