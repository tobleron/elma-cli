//! @efficiency-role: domain-logic
//!
//! Release risk security audit gate (Task 677).
//! Scans the codebase for common security anti-patterns.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub(crate) struct UnsafeBlock {
    pub(crate) file: std::path::PathBuf,
    pub(crate) line: usize,
    pub(crate) context: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SecurityFinding {
    pub(crate) finding_type: String,
    pub(crate) file: std::path::PathBuf,
    pub(crate) line: usize,
    pub(crate) description: String,
    pub(crate) risk: RiskLevel,
}

#[derive(Debug, Clone)]
pub(crate) struct SecurityAuditReport {
    pub(crate) findings: Vec<SecurityFinding>,
    pub(crate) risk_level: RiskLevel,
    pub(crate) summary: String,
}

pub(crate) struct SecurityAudit;

impl SecurityAudit {
    pub(crate) fn run(root: &Path) -> SecurityAuditReport {
        let mut findings = Vec::new();

        let blocks = scan_unsafe_blocks(root);
        for block in &blocks {
            findings.push(SecurityFinding {
                finding_type: "unsafe_block".into(),
                file: block.file.clone(),
                line: block.line,
                description: format!("unsafe block: {}", block.context),
                risk: RiskLevel::Medium,
            });
        }

        findings.extend(scan_secrets(root));
        findings.extend(scan_command_injection(root));
        findings.extend(scan_world_writable(root));

        let risk_level = if findings.is_empty() {
            RiskLevel::Low
        } else if findings.iter().any(|f| f.risk == RiskLevel::Critical) {
            RiskLevel::Critical
        } else if findings.iter().any(|f| f.risk == RiskLevel::High) {
            RiskLevel::High
        } else if findings.iter().any(|f| f.risk == RiskLevel::Medium) {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let summary = format!(
            "Security audit complete: {} finding(s) ({} unsafe, highest risk: {:?})",
            findings.len(),
            blocks.len(),
            risk_level,
        );

        SecurityAuditReport {
            findings,
            risk_level,
            summary,
        }
    }
}

pub(crate) fn scan_unsafe_blocks(path: &Path) -> Vec<UnsafeBlock> {
    let mut blocks = Vec::new();
    let mut entries = Vec::new();
    collect_rs_files(path, &mut entries);

    for file in &entries {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            if line.contains("unsafe ") && (line.contains('{') || line.trim() == "unsafe") {
                let ctx = line.trim().chars().take(60).collect::<String>();
                blocks.push(UnsafeBlock {
                    file: file.clone(),
                    line: i + 1,
                    context: ctx,
                });
            }
        }
    }
    blocks
}

fn scan_secrets(root: &Path) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut entries = Vec::new();
    collect_rs_files(root, &mut entries);

    let secret_patterns = [
        "api_key",
        "api.secret",
        "password",
        "passwd",
        "secret_key",
        "access.token",
        "auth.token",
        "credentials",
    ];

    for file in &entries {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            let lower = line.to_lowercase();
            for pat in &secret_patterns {
                if lower.contains(pat) && (line.contains('=') || line.contains(':')) {
                    let val = line.split('=').nth(1).unwrap_or(line).trim();
                    let is_literal =
                        val.starts_with('"') || val.starts_with('\'') || val.starts_with("`");
                    if is_literal && val.len() > 4 {
                        findings.push(SecurityFinding {
                            finding_type: "potential_secret".into(),
                            file: file.clone(),
                            line: i + 1,
                            description: format!("Possible hardcoded secret ({})", pat),
                            risk: RiskLevel::High,
                        });
                    }
                    break;
                }
            }
        }
    }
    findings
}

fn scan_command_injection(root: &Path) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut entries = Vec::new();
    collect_rs_files(root, &mut entries);

    for file in &entries {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.contains("Command::new") && trimmed.contains("format!") {
                findings.push(SecurityFinding {
                    finding_type: "command_injection".into(),
                    file: file.clone(),
                    line: i + 1,
                    description: "Command::new with formatted input may allow injection".into(),
                    risk: RiskLevel::High,
                });
            }
            if trimmed.contains("eval(") || trimmed.contains("std::process::Command::new") {
                if trimmed.contains("&format") || trimmed.contains("+ &") {
                    findings.push(SecurityFinding {
                        finding_type: "eval_pattern".into(),
                        file: file.clone(),
                        line: i + 1,
                        description: "eval-like pattern with string concatenation".into(),
                        risk: RiskLevel::Critical,
                    });
                }
            }
        }
    }
    findings
}

fn scan_world_writable(root: &Path) -> Vec<SecurityFinding> {
    let mut findings = Vec::new();
    let mut entries = Vec::new();
    collect_rs_files(root, &mut entries);

    for file in &entries {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            if line.contains("0o777") || line.contains("0o666") {
                findings.push(SecurityFinding {
                    finding_type: "world_writable_permissions".into(),
                    file: file.clone(),
                    line: i + 1,
                    description: "World-writable permission mode detected".into(),
                    risk: RiskLevel::Medium,
                });
            }
        }
    }
    findings
}

fn collect_rs_files(path: &Path, entries: &mut Vec<std::path::PathBuf>) {
    if path.is_dir() {
        if let Ok(dir) = std::fs::read_dir(path) {
            for entry in dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if name != "target" && !name.starts_with('.') {
                        collect_rs_files(&path, entries);
                    }
                } else if path.extension().map_or(false, |e| e == "rs") {
                    entries.push(path);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_scan_unsafe_blocks_detects_unsafe() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "fn foo() { unsafe { /* raw */ } }\n").unwrap();
        let blocks = scan_unsafe_blocks(dir.path());
        assert!(!blocks.is_empty());
        assert!(blocks[0].context.contains("unsafe"));
    }

    #[test]
    fn test_scan_unsafe_blocks_clean_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("safe.rs");
        fs::write(&file, "fn foo() { let x = 1; }\n").unwrap();
        let blocks = scan_unsafe_blocks(dir.path());
        assert!(blocks.is_empty());
    }

    #[test]
    fn test_security_audit_run_produces_report() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "fn foo() { unsafe { let p = std::ptr::null(); } }\n").unwrap();
        let report = SecurityAudit::run(dir.path());
        assert!(!report.findings.is_empty());
        assert_eq!(report.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_scan_secrets_detects_hardcoded() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("config.rs");
        fs::write(&file, "let api_key = \"sk-123456\";\n").unwrap();
        let findings = scan_secrets(dir.path());
        assert!(!findings.is_empty());
        assert!(findings
            .iter()
            .any(|f| f.finding_type == "potential_secret"));
    }

    #[test]
    fn test_scan_secrets_ignores_variable_only() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("config.rs");
        fs::write(&file, "let api_key = get_env(\"API_KEY\");\n").unwrap();
        let findings = scan_secrets(dir.path());
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_world_writable_detects_mode() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("perms.rs");
        fs::write(&file, "let mode = 0o777;\n").unwrap();
        let findings = scan_world_writable(dir.path());
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_risk_level_escalation() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("inject.rs");
        fs::write(
            &file,
            "fn run(cmd: &str) { std::process::Command::new(&format!(\"bash {}\", cmd)); }\n",
        )
        .unwrap();
        let report = SecurityAudit::run(dir.path());
        assert_eq!(report.risk_level, RiskLevel::Critical);
    }

    #[test]
    fn test_clean_codebase_low_risk() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("clean.rs");
        fs::write(&file, "fn main() { println!(\"hi\"); }\n").unwrap();
        let report = SecurityAudit::run(dir.path());
        assert_eq!(report.risk_level, RiskLevel::Low);
    }
}
