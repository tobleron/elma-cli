//! @efficiency-role: domain-logic
//!
//! Cross-platform portability gate (Task 673).
//! Scans the codebase for platform-specific patterns that may break
//! on non-Unix systems.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PortabilityCheck {
    PathSeparator,
    LineEnding,
    CaseSensitive,
    ShellEncoding,
    TempDir,
    HomeDir,
}

#[derive(Debug, Clone)]
pub(crate) struct PortabilityIssue {
    pub(crate) check: PortabilityCheck,
    pub(crate) file: std::path::PathBuf,
    pub(crate) line: usize,
    pub(crate) finding: String,
    pub(crate) fix: Option<String>,
}

pub(crate) struct PortabilityGate;

impl PortabilityGate {
    pub(crate) fn check() -> Vec<PortabilityIssue> {
        Vec::new()
    }
}

pub(crate) fn scan_for_platform_issues(root: &Path) -> Vec<PortabilityIssue> {
    let mut issues = Vec::new();
    let entries = match walk_dir(root) {
        Ok(e) => e,
        Err(_) => return issues,
    };
    for entry in entries {
        let path = entry.path();
        if path
            .extension()
            .map_or(true, |e| e != "rs" && e != "sh" && e != "toml")
        {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (i, line) in content.lines().enumerate() {
            let line_num = i + 1;

            if line.contains("sh -c")
                && !line.trim().starts_with("//")
                && !line.trim().starts_with('#')
            {
                issues.push(PortabilityIssue {
                    check: PortabilityCheck::ShellEncoding,
                    file: path.clone(),
                    line: line_num,
                    finding: format!("'sh -c' usage may fail on Windows: {}", line.trim()),
                    fix: Some(
                        "Use Command::new(\"cmd\") / std::process::Command on Windows".into(),
                    ),
                });
            }

            if line.contains("#!/bin/bash") || line.contains("#!/bin/sh") {
                issues.push(PortabilityIssue {
                    check: PortabilityCheck::ShellEncoding,
                    file: path.clone(),
                    line: line_num,
                    finding: format!("Unix shebang not portable: {}", line.trim()),
                    fix: Some(
                        "Consider a cross-platform approach or document Windows equivalent".into(),
                    ),
                });
            }

            if !line.trim().starts_with("//") {
                let unix_paths = [
                    "/tmp/", "/usr/", "/etc/", "/var/", "/home/", "/opt/", "/bin/",
                ];
                for up in &unix_paths {
                    if line.contains(up) && (line.contains('"') || line.contains('\'')) {
                        issues.push(PortabilityIssue {
                            check: PortabilityCheck::PathSeparator,
                            file: path.clone(),
                            line: line_num,
                            finding: format!("Possible hardcoded Unix path: {}", line.trim()),
                            fix: Some(
                                "Use std::path::PathBuf / dirs crate for platform-aware paths"
                                    .into(),
                            ),
                        });
                        break;
                    }
                }
            }
        }
    }
    issues
}

fn walk_dir(root: &Path) -> Result<Vec<std::fs::DirEntry>, std::io::Error> {
    let mut entries = Vec::new();
    if root.is_dir() {
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                if let Ok(sub) = walk_dir(&path) {
                    entries.extend(sub);
                }
            } else {
                entries.push(entry);
            }
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_hardcoded_slash_path_detected() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.rs");
        fs::write(&file, "let p = \"/tmp/foo\";\n").unwrap();
        let issues = scan_for_platform_issues(dir.path());
        assert!(issues
            .iter()
            .any(|i| i.check == PortabilityCheck::PathSeparator));
    }

    #[test]
    fn test_shellcheck_issue_detected() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("build.rs");
        fs::write(&file, "fn run() { Command::new(\"sh -c\"); }\n").unwrap();
        let issues = scan_for_platform_issues(dir.path());
        assert!(issues
            .iter()
            .any(|i| i.check == PortabilityCheck::ShellEncoding));
    }

    #[test]
    fn test_bash_shebang_detected() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("script.sh");
        fs::write(&file, "#!/bin/bash\necho hi\n").unwrap();
        let issues = scan_for_platform_issues(dir.path());
        assert!(issues
            .iter()
            .any(|i| i.check == PortabilityCheck::ShellEncoding));
    }

    #[test]
    fn test_clean_file_no_issues() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("clean.rs");
        fs::write(&file, "fn main() { let x = 1; }\n").unwrap();
        let issues = scan_for_platform_issues(dir.path());
        let relevant: Vec<_> = issues.into_iter().filter(|i| i.file == file).collect();
        assert!(relevant.is_empty());
    }

    #[test]
    fn test_portability_gate_check_returns_empty() {
        let issues = PortabilityGate::check();
        assert!(issues.is_empty());
    }
}
