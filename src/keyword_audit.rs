//! @efficiency-role: audit
//!
//! Keyword gate audit — scans Rust source files for hardcoded keyword-based
//! routing patterns that violate Rule 1 (no keyword matcher). Provides
//! AnalyzerRule for automated fix suggestions.

use crate::*;
use regex::Regex;

/// Severity of a keyword match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum KeywordRisk {
    /// Routing decisions — keywords used to determine control flow (Rule 1 violation)
    High,
    /// Input parsing — keywords used to classify or tag user input
    Medium,
    /// UI formatting — cosmetic keyword matching
    Low,
}

impl KeywordRisk {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            KeywordRisk::High => "HIGH",
            KeywordRisk::Medium => "MEDIUM",
            KeywordRisk::Low => "LOW",
        }
    }
}

/// A single keyword match found during an audit scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct KeywordMatch {
    pub(crate) file: String,
    pub(crate) line: usize,
    pub(crate) pattern: String,
    pub(crate) risk: KeywordRisk,
}

/// An analyzer rule describing a specific anti-pattern to detect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AnalyzerRule {
    pub(crate) id: String,
    pub(crate) description: String,
    pub(crate) severity: KeywordRisk,
    pub(crate) applies_to: Vec<String>,
}

impl AnalyzerRule {
    /// Generate a fix suggestion for a given keyword match.
    pub(crate) fn fix_suggestion(m: &KeywordMatch) -> String {
        match m.risk {
            KeywordRisk::High => format!(
                "Line {} in {}: Replace keyword-based routing with confidence/entropy-based routing. Use model signals, not `.contains(\"{}\")`.",
                m.line, m.file, m.pattern
            ),
            KeywordRisk::Medium => format!(
                "Line {} in {}: Classify via model signals (probability, entropy) rather than keyword matching on \"{}\".",
                m.line, m.file, m.pattern
            ),
            KeywordRisk::Low => format!(
                "Line {} in {}: Consider using a structured mapping instead of string matching on \"{}\".",
                m.line, m.file, m.pattern
            ),
        }
    }
}

/// Scans codebases for hardcoded keyword-based routing patterns.
pub(crate) struct KeywordGateAudit;

impl KeywordGateAudit {
    /// Scan all Rust files under `codebase_path` for keyword matching patterns.
    pub(crate) fn audit(codebase_path: &Path) -> Vec<KeywordMatch> {
        let mut results = Vec::new();

        // Pattern list: (regex, human label, risk level)
        let patterns: Vec<(&str, &str, KeywordRisk)> = vec![
            (
                r#"if\s+\w+\s*\.\s*contains\s*\("#,
                "if x.contains(...)",
                KeywordRisk::High,
            ),
            (
                r#"(input|text|cmd|message)\s*\.\s*contains\s*\("#,
                "input.contains(...)",
                KeywordRisk::Medium,
            ),
            (
                r#"\.\s*contains\s*\(\s*"#,
                ".contains(\"...\")",
                KeywordRisk::Low,
            ),
            (
                r#"(?:input|text|msg)\s*==\s*"[^"]*"#,
                "string equality on input",
                KeywordRisk::Medium,
            ),
        ];

        let rs_files = collect_rs_files(codebase_path);

        for path in &rs_files {
            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let rel = path.strip_prefix(codebase_path).unwrap_or(path);
            let rel_str = rel.to_string_lossy().to_string();

            for (re_str, label, risk) in &patterns {
                let Ok(re) = Regex::new(re_str) else {
                    continue;
                };
                for m in re.find_iter(&content) {
                    let line_num = content[..m.start()].matches('\n').count() + 1;
                    results.push(KeywordMatch {
                        file: rel_str.clone(),
                        line: line_num,
                        pattern: label.to_string(),
                        risk: *risk,
                    });
                }
            }
        }

        results
    }
}

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return files;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap_or_default();
            if name == "target" {
                continue;
            }
            files.extend(collect_rs_files(&path));
        } else if path.extension().map_or(false, |e| e == "rs") {
            files.push(path);
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_keyword_risk_labels() {
        assert_eq!(KeywordRisk::High.label(), "HIGH");
        assert_eq!(KeywordRisk::Medium.label(), "MEDIUM");
        assert_eq!(KeywordRisk::Low.label(), "LOW");
    }

    #[test]
    fn test_risk_serde_roundtrip() {
        let json = serde_json::to_string(&KeywordRisk::High).unwrap();
        assert_eq!(json, "\"High\"");
        let back: KeywordRisk = serde_json::from_str(&json).unwrap();
        assert_eq!(back, KeywordRisk::High);
    }

    #[test]
    fn test_fix_suggestion_high_risk() {
        let m = KeywordMatch {
            file: "src/routing.rs".into(),
            line: 42,
            pattern: "if x.contains(".into(),
            risk: KeywordRisk::High,
        };
        let suggestion = AnalyzerRule::fix_suggestion(&m);
        assert!(suggestion.contains("keyword-based routing"));
        assert!(suggestion.contains("routing.rs"));
    }

    #[test]
    fn test_fix_suggestion_medium_risk() {
        let m = KeywordMatch {
            file: "src/parser.rs".into(),
            line: 10,
            pattern: "input.contains(".into(),
            risk: KeywordRisk::Medium,
        };
        let suggestion = AnalyzerRule::fix_suggestion(&m);
        assert!(suggestion.contains("model signals"));
    }

    #[test]
    fn test_fix_suggestion_low_risk() {
        let m = KeywordMatch {
            file: "src/ui.rs".into(),
            line: 5,
            pattern: ".contains(".into(),
            risk: KeywordRisk::Low,
        };
        let suggestion = AnalyzerRule::fix_suggestion(&m);
        assert!(suggestion.contains("structured mapping"));
    }

    #[test]
    fn test_audit_detects_contains_patterns() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("bad_routing.rs"),
            "fn route(input: &str) {\n    if input.contains(\"help\") {\n        do_help();\n    }\n}\n",
        )
        .unwrap();

        let results = KeywordGateAudit::audit(tmp.path());
        assert!(!results.is_empty(), "should detect patterns");
        assert!(results.iter().any(|m| m.file == "bad_routing.rs"));
    }

    #[test]
    fn test_audit_skips_target_dir() {
        let tmp = TempDir::new().unwrap();
        let td = tmp.path().join("target");
        std::fs::create_dir_all(&td).unwrap();
        std::fs::write(td.join("generated.rs"), "if x.contains(\"bad\") {}").unwrap();

        let results = KeywordGateAudit::audit(tmp.path());
        assert!(results.is_empty());
    }

    #[test]
    fn test_audit_empty_codebase() {
        let tmp = TempDir::new().unwrap();
        let results = KeywordGateAudit::audit(tmp.path());
        assert!(results.is_empty());
    }

    #[test]
    fn test_audit_no_matches() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("clean.rs"),
            "fn add(a: i32, b: i32) -> i32 { a + b }\n",
        )
        .unwrap();
        let results = KeywordGateAudit::audit(tmp.path());
        assert!(results.is_empty());
    }

    #[test]
    fn test_audit_non_rs_files_skipped() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("readme.md"), "if x.contains(\"bad\")").unwrap();
        let results = KeywordGateAudit::audit(tmp.path());
        assert!(results.is_empty());
    }
}
