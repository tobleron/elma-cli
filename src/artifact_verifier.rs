//! @efficiency-role: domain-logic
//!
//! Artifact Deliverable Verifier — Task 688 / Task 697.
//!
//! Tracks required output artifacts requested by the user and verifies
//! they exist before finalization. Prevents the model from claiming
//! completion without creating the requested files.
//!
//! Task 697: Task-specific artifact naming, artifact manifest, collision-safe paths.

use crate::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// Quality state of a required artifact file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArtifactState {
    /// Model-authored content written successfully.
    CompleteModelAuthored,
    /// Deterministic structured fallback (not model-authored, but properly formatted).
    CompleteDeterministicStructured,
    /// Recovered evidence dump — file exists but content is raw tool output.
    PartialEvidenceRecovery,
    /// File does not exist on disk.
    Failed,
}

impl std::fmt::Display for ArtifactState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactState::CompleteModelAuthored => write!(f, "complete_model_authored"),
            ArtifactState::CompleteDeterministicStructured => write!(f, "complete_deterministic_structured"),
            ArtifactState::PartialEvidenceRecovery => write!(f, "partial_evidence_recovery"),
            ArtifactState::Failed => write!(f, "failed"),
        }
    }
}

/// Check whether a file contains a recovered-evidence-dump header.
pub(crate) fn is_evidence_recovery_file(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    if let Ok(content) = std::fs::read_to_string(path) {
        content.lines().next().map_or(false, |first_line| {
            first_line.trim().starts_with("# Recovered Artifact:")
        })
    } else {
        false
    }
}

/// Check whether a file is empty or contains only whitespace.
pub(crate) fn is_empty_file(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    if let Ok(content) = std::fs::read_to_string(path) {
        content.trim().is_empty()
    } else {
        false
    }
}

/// Determine the artifact state for a given required artifact.
pub(crate) fn get_artifact_state(workspace_root: &Path, artifact_relative_path: &str) -> ArtifactState {
    let full_path = workspace_root.join(artifact_relative_path);
    if !full_path.exists() {
        return ArtifactState::Failed;
    }
    if is_evidence_recovery_file(&full_path) {
        return ArtifactState::PartialEvidenceRecovery;
    }
    if is_empty_file(&full_path) {
        return ArtifactState::PartialEvidenceRecovery;
    }
    // File exists, is not empty, and is not an evidence-recovery dump
    ArtifactState::CompleteModelAuthored
}

/// Check if all required artifacts are in a complete state (not partial/failed).
pub(crate) fn are_all_artifacts_complete(workspace_root: &Path) -> bool {
    let artifacts = get_required_artifacts();
    if artifacts.is_empty() {
        return true;
    }
    for artifact in &artifacts {
        let state = get_artifact_state(workspace_root, artifact);
        if state == ArtifactState::Failed || state == ArtifactState::PartialEvidenceRecovery {
            return false;
        }
    }
    true
}

/// Find artifacts that are incomplete (partial or failed), returning state per artifact.
pub(crate) fn find_incomplete_artifacts(workspace_root: &Path) -> Vec<(String, PathBuf, ArtifactState)> {
    let artifacts = get_required_artifacts();
    if artifacts.is_empty() {
        return Vec::new();
    }
    let mut incomplete = Vec::new();
    for artifact in &artifacts {
        let state = get_artifact_state(workspace_root, artifact);
        if state == ArtifactState::Failed || state == ArtifactState::PartialEvidenceRecovery {
            let full_path = workspace_root.join(artifact);
            incomplete.push((artifact.clone(), full_path, state));
        }
    }
    incomplete
}

// ============================================================================
// Global state (session-scoped)
// ============================================================================

static REQUIRED_ARTIFACTS: OnceLock<RwLock<HashSet<String>>> = OnceLock::new();

fn required_artifacts() -> &'static RwLock<HashSet<String>> {
    REQUIRED_ARTIFACTS.get_or_init(|| RwLock::new(HashSet::new()))
}

/// Initialize the required artifacts set for a new session.
pub(crate) fn init_artifact_tracking() {
    if let Ok(mut lock) = required_artifacts().write() {
        lock.clear();
    }
}

/// Register a file path as a required deliverable (Task 688).
/// Call this when the user explicitly requests that a file be created.
pub(crate) fn require_artifact(path: &str) {
    if let Ok(mut lock) = required_artifacts().write() {
        lock.insert(normalize_path(path));
    }
}

/// Register multiple file paths as required deliverables.
pub(crate) fn require_artifacts(paths: &[String]) {
    if let Ok(mut lock) = required_artifacts().write() {
        for p in paths {
            lock.insert(normalize_path(p));
        }
    }
}

/// Check if a specific path is a required artifact.
pub(crate) fn is_required_artifact(path: &str) -> bool {
    if let Ok(lock) = required_artifacts().read() {
        lock.contains(&normalize_path(path))
    } else {
        false
    }
}

/// Normalize a path for consistent comparison.
fn normalize_path(path: &str) -> String {
    path.trim()
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

/// Get all currently registered required artifacts.
pub(crate) fn get_required_artifacts() -> Vec<String> {
    if let Ok(lock) = required_artifacts().read() {
        let mut artifacts: Vec<String> = lock.iter().cloned().collect();
        artifacts.sort();
        artifacts
    } else {
        Vec::new()
    }
}

/// Verify that all required artifacts exist on disk.
/// Returns a list of missing artifacts.
pub(crate) fn find_missing_artifacts(workspace_root: &Path) -> Vec<(String, PathBuf)> {
    let artifacts = get_required_artifacts();
    if artifacts.is_empty() {
        return Vec::new();
    }

    let mut missing = Vec::new();
    for artifact in &artifacts {
        let full_path = workspace_root.join(artifact);
        if !full_path.exists() {
            missing.push((artifact.clone(), full_path));
        }
    }
    missing
}

/// Verify that file claims in a final answer are backed by real file writes.
/// Returns a list of unsubstantiated file claims from the answer text.
pub(crate) fn find_unsubstantiated_file_claims(
    final_answer: &str,
    workspace_root: &Path,
) -> Vec<String> {
    let mut claims = Vec::new();

    // Look for patterns like "Created: path/to/file", "Wrote path/to/file"
    for line in final_answer.lines() {
        let lower = line.to_lowercase();
        for prefix in &["created", "wrote", "wrote to", "saved to"] {
            if let Some(pos) = lower.find(prefix) {
                let after = &line[pos + prefix.len()..];
                // Try to extract a path from the remainder
                let path_candidate = after
                    .trim_start_matches(|c: char| c == ':' || c == ' ' || c == '\t')
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(|c: char| c == '.' || c == ',' || c == '!');
                if !path_candidate.is_empty()
                    && (path_candidate.contains('/') || path_candidate.contains('.'))
                {
                    let full_path = workspace_root.join(path_candidate);
                    if !full_path.exists() {
                        claims.push(path_candidate.to_string());
                    }
                }
            }
        }
    }

    claims
}

/// Check if a final answer claims file creation that is registered as required.
pub(crate) fn required_artifacts_addressed(final_answer: &str) -> Vec<String> {
    let artifacts = get_required_artifacts();
    if artifacts.is_empty() {
        return Vec::new();
    }

    let lower_answer = final_answer.to_lowercase();
    artifacts
        .into_iter()
        .filter(|a| !lower_answer.contains(&a.to_lowercase()))
        .collect()
}

/// Artifact manifest entry (Task 697).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArtifactManifestEntry {
    pub requested_path: String,
    pub inferred_path: Option<String>,
    pub artifact_type: String,
    pub created: bool,
    pub verified: bool,
    pub session_id: String,
}

/// Artifact manifest for the current session (Task 697).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ArtifactManifest {
    pub entries: Vec<ArtifactManifestEntry>,
    pub session_id: String,
}

static ARTIFACT_MANIFEST: OnceLock<RwLock<ArtifactManifest>> = OnceLock::new();

fn artifact_manifest() -> &'static RwLock<ArtifactManifest> {
    ARTIFACT_MANIFEST.get_or_init(|| RwLock::new(ArtifactManifest::default()))
}

/// Initialize the artifact manifest for a session.
pub(crate) fn init_artifact_manifest(session_id: &str) {
    if let Ok(mut lock) = artifact_manifest().write() {
        lock.session_id = session_id.to_string();
        lock.entries.clear();
    }
}

/// Register an artifact in the manifest (Task 697).
pub(crate) fn register_artifact_in_manifest(
    requested_path: &str,
    inferred_path: Option<String>,
    artifact_type: &str,
) {
    if let Ok(mut lock) = artifact_manifest().write() {
        let sid = lock.session_id.clone();
        lock.entries.push(ArtifactManifestEntry {
            requested_path: requested_path.to_string(),
            inferred_path,
            artifact_type: artifact_type.to_string(),
            created: false,
            verified: false,
            session_id: sid,
        });
    }
}

/// Mark an artifact as created in the manifest.
pub(crate) fn mark_artifact_created(requested_path: &str) {
    if let Ok(mut lock) = artifact_manifest().write() {
        for entry in &mut lock.entries {
            if entry.requested_path == requested_path {
                entry.created = true;
            }
        }
    }
}

/// Mark an artifact as verified.
pub(crate) fn mark_artifact_verified(requested_path: &str) {
    if let Ok(mut lock) = artifact_manifest().write() {
        for entry in &mut lock.entries {
            if entry.requested_path == requested_path {
                entry.verified = true;
            }
        }
    }
}

/// Get the current artifact manifest.
pub(crate) fn get_artifact_manifest() -> ArtifactManifest {
    if let Ok(lock) = artifact_manifest().read() {
        lock.clone()
    } else {
        ArtifactManifest::default()
    }
}

/// Derive a task-specific filename from user intent and artifact type (Task 697).
/// Produces collision-safe, descriptive filenames instead of generic "report.md".
pub(crate) fn derive_task_filename(user_request: &str, artifact_type: &str, extension: &str) -> String {
    let lower = user_request.to_lowercase();

    // Extract key terms from the request
    let terms = extract_key_terms(&lower);

    // Build slug from type and key terms
    let mut parts: Vec<String> = Vec::new();
    parts.push(artifact_type.to_string());

    // Add distinguishing terms
    for term in &terms {
        if !parts.contains(term) {
            parts.push(term.clone());
        }
        if parts.len() >= 4 {
            break;
        }
    }

    let slug = parts.join("_");
    let slug = slug.replace(' ', "_");
    let slug = slug
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>();

    // Add timestamp for collision safety
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let short_ts = format!("{:x}", nanos);
    let ts = &short_ts[short_ts.len().saturating_sub(8)..];

    format!("{}_{}.{}", slug, ts, extension.trim_start_matches('.'))
}

/// Extract key distinguishing terms from a user request for filename generation.
fn extract_key_terms(text: &str) -> Vec<String> {
    let stop_words = [
        "the", "a", "an", "in", "to", "for", "of", "and", "or", "is", "are",
        "was", "were", "be", "been", "being", "have", "has", "had", "do", "does",
        "did", "will", "would", "could", "should", "may", "might", "shall",
        "can", "need", "must", "please", "let", "me", "my", "this", "that",
        "these", "those", "with", "from", "by", "on", "at", "as", "but",
        "not", "all", "any", "each", "every", "some", "no", "its", "it's",
        "i'd", "i'll", "i'm", "i've", "you", "your", "we", "our", "they",
        "their", "create", "write", "save", "output", "file", "report", "check",
        "find", "search", "list", "show", "get", "make", "new",
    ];

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut terms: Vec<String> = Vec::new();
    let mut seen = HashSet::new();

    for w in words {
        let clean = w.trim_matches(|c: char| !c.is_alphanumeric());
        if clean.len() < 3 {
            continue;
        }
        if stop_words.contains(&clean) {
            continue;
        }
        if seen.insert(clean.to_string()) {
            terms.push(clean.to_string());
        }
    }

    terms
}

/// Persist the artifact manifest to the session.
pub(crate) fn persist_artifact_manifest(session_root: &Path) {
    let manifest = get_artifact_manifest();
    let manifest_path = session_root.join("artifact_manifest.json");
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        let _ = std::fs::write(&manifest_path, &json);
    }
}

/// Generate a finalization notice about missing deliverables.
pub(crate) fn build_missing_artifact_notice(missing: &[(String, PathBuf)]) -> String {
    if missing.is_empty() {
        return String::new();
    }

    let items: Vec<String> = missing
        .iter()
        .map(|(path, _)| format!("  - {} (requested file was not created)", path))
        .collect();

    format!(
        "\n\n**Incomplete Deliverables:**\n{}\n\
         The following requested files were not created. \
         The task may need to be continued to produce these artifacts.",
        items.join("\n")
    )
}

/// Clear all tracked artifacts (for new sessions).
pub(crate) fn clear_artifact_tracking() {
    if let Ok(mut lock) = required_artifacts().write() {
        lock.clear();
    }
}

/// Extract file paths from a user request that look like required output artifacts.
/// Uses heuristics: looks for paths containing `/` followed by report/doc/md patterns
/// near words like "create", "write", "save", "output", "report".
pub(crate) fn extract_required_artifacts_from_request(user_request: &str) -> Vec<String> {
    let mut artifacts = Vec::new();
    let lower = user_request.to_lowercase();

    for line in user_request.lines() {
        let trimmed = line.trim();

        // Check for file creation patterns
        let is_output_request = {
            let lower_line = trimmed.to_lowercase();
            lower_line.contains("create")
                || lower_line.contains("write")
                || lower_line.contains("save ")
                || lower_line.contains("output ")
                || lower_line.contains("generate")
                || lower_line.contains("produce")
                || lower_line.ends_with(".md")
                || lower_line.ends_with(".txt")
                || lower_line.ends_with(".json")
                || lower_line.ends_with(".rs")
                || lower_line.ends_with(".toml")
        };

        if !is_output_request {
            continue;
        }

        // Extract path-like strings
        for word in trimmed.split_whitespace() {
            let clean = word
                .trim_start_matches('`')
                .trim_end_matches('`')
                .trim_end_matches(|c: char| c == '.' || c == ',' || c == ')' || c == ']')
                .trim_start_matches(|c: char| c == '(' || c == '[');
            if (clean.contains('/') || clean.contains('.'))
                && !clean.starts_with('-')
                && !clean.starts_with("--")
                && clean.len() > 3
                && clean.len() < 200
                && clean.contains(|c: char| c.is_alphanumeric())
            {
                let cleaned = clean.to_string();
                if !artifacts.contains(&cleaned) {
                    artifacts.push(cleaned);
                }
            }
        }
    }

    if artifacts.is_empty() {
        if let Some(inferred) = infer_project_tmp_report_path(&lower) {
            artifacts.push(inferred);
        }
    }

    artifacts
}

fn infer_project_tmp_report_path(lower_request: &str) -> Option<String> {
    let asks_for_written_deliverable = lower_request.contains("create")
        || lower_request.contains("write")
        || lower_request.contains("save")
        || lower_request.contains("produce")
        || lower_request.contains("generate");
    let asks_for_report_like_file = lower_request.contains("report")
        || lower_request.contains("findings")
        || lower_request.contains("summary")
        || lower_request.contains("audit")
        || lower_request.contains("document");

    if !asks_for_written_deliverable
        || !asks_for_report_like_file
        || !lower_request.contains("project_tmp")
    {
        return None;
    }

    let stem = if lower_request.contains("security") {
        "security_report"
    } else if lower_request.contains("architecture") {
        "architecture_report"
    } else if lower_request.contains("audit") {
        "audit_report"
    } else if lower_request.contains("cleanup") {
        "cleanup_report"
    } else if lower_request.contains("test") || lower_request.contains("testing") {
        "testing_report"
    } else if lower_request.contains("summary") {
        "summary"
    } else {
        "report"
    };

    Some(format!("project_tmp/{stem}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_init_and_clear() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        require_artifact("project_tmp/report.md");
        assert_eq!(get_required_artifacts().len(), 1);
        clear_artifact_tracking();
        assert!(get_required_artifacts().is_empty());
    }

    #[test]
    fn test_require_and_check() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        require_artifact("project_tmp/report.md");
        assert!(is_required_artifact("project_tmp/report.md"));
        assert!(is_required_artifact("  project_tmp/report.md  "));
        assert!(!is_required_artifact("other.md"));
    }

    #[test]
    fn test_require_multiple() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        require_artifacts(&[
            "a.md".to_string(),
            "b.txt".to_string(),
        ]);
        assert_eq!(get_required_artifacts().len(), 2);
    }

    #[test]
    fn test_find_missing_artifacts() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        require_artifact("nonexistent_file_12345.md");
        let missing = find_missing_artifacts(Path::new("/tmp"));
        assert_eq!(missing.len(), 1);
        assert_eq!(missing[0].0, "nonexistent_file_12345.md");
    }

    #[test]
    fn test_no_missing_when_empty() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        let missing = find_missing_artifacts(Path::new("/tmp"));
        assert!(missing.is_empty());
    }

    #[test]
    fn test_extract_artifacts_from_request() {
        let request = "Create a report in project_tmp/security_report.md and save the output to project_tmp/summary.txt";
        let artifacts = extract_required_artifacts_from_request(request);
        assert!(artifacts.contains(&"project_tmp/security_report.md".to_string()));
        assert!(artifacts.contains(&"project_tmp/summary.txt".to_string()));
    }

    #[test]
    fn test_extract_artifacts_no_false_positives() {
        let request = "Check the docs/README.md file";
        let artifacts = extract_required_artifacts_from_request(request);
        // This should not extract docs/README.md since it starts with "Check"
        // and doesn't contain create/write/save/output keywords
        assert!(!artifacts.contains(&"docs/README.md".to_string()));
    }

    #[test]
    fn test_unsubstantiated_claims_detected() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        let answer = "I created project_tmp/report.md with the findings.";
        let tmp = std::env::temp_dir();
        let claims = find_unsubstantiated_file_claims(answer, &tmp);
        assert!(!claims.is_empty());
        assert!(claims.contains(&"project_tmp/report.md".to_string()));
    }

    #[test]
    fn test_required_artifacts_addressed() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        require_artifact("project_tmp/report.md");
        require_artifact("project_tmp/audit.json");
        let answer = "I created project_tmp/report.md with the results.";
        let unaddressed = required_artifacts_addressed(answer);
        assert!(unaddressed.contains(&"project_tmp/audit.json".to_string()));
        assert!(!unaddressed.contains(&"project_tmp/report.md".to_string()));
    }

    #[test]
    fn test_no_required_no_unaddressed() {
        let _guard = TEST_MUTEX.lock().unwrap();
        init_artifact_tracking();
        let unaddressed = required_artifacts_addressed("Nothing to see here.");
        assert!(unaddressed.is_empty());
    }

    #[test]
    fn test_build_missing_notice() {
        let missing = vec![("report.md".to_string(), PathBuf::from("report.md"))];
        let notice = build_missing_artifact_notice(&missing);
        assert!(notice.contains("report.md"));
        assert!(notice.contains("Incomplete Deliverables"));
    }

    #[test]
    fn test_empty_missing_notice() {
        let notice = build_missing_artifact_notice(&[]);
        assert!(notice.is_empty());
    }

    #[test]
    fn test_extract_artifacts_markdown_code_ticks() {
        let request = "Write a report to `project_tmp/audit.md`";
        let artifacts = extract_required_artifacts_from_request(request);
        assert!(artifacts.contains(&"project_tmp/audit.md".to_string()));
    }

    #[test]
    fn test_infer_project_tmp_security_report_without_filename() {
        let request = "Search src and create a detailed security report in the project_tmp directory.";
        let artifacts = extract_required_artifacts_from_request(request);
        assert!(artifacts.contains(&"project_tmp/security_report.md".to_string()));
    }
}
