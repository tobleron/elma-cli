//! @efficiency-role: domain-logic
//!
//! Offline LSP diagnostics and code intelligence tool (Task 670).
//! Checks for available LSP servers and provides basic compile-check diagnostics
//! via rustc output parsing as a fallback when no LSP server is running.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum LspCapability {
    Diagnostics,
    Completions,
    Hover,
    GoToDefinition,
    References,
}

#[derive(Debug, Clone)]
pub(crate) struct LspDiagnostic {
    pub severity: String,
    pub message: String,
    pub line: usize,
    pub code: Option<String>,
}

pub(crate) struct OfflineLsp;

impl OfflineLsp {
    pub(crate) fn check_available() -> Vec<LspCapability> {
        let mut caps = Vec::new();
        if which::which("rust-analyzer").is_ok() {
            caps.extend_from_slice(&[
                LspCapability::Diagnostics,
                LspCapability::Completions,
                LspCapability::Hover,
                LspCapability::GoToDefinition,
                LspCapability::References,
            ]);
        }
        caps
    }

    pub(crate) fn has_lsp() -> bool {
        which::which("rust-analyzer").is_ok()
    }

    pub(crate) fn run_diagnostics(file_path: &Path) -> Vec<LspDiagnostic> {
        let mut diagnostics = Vec::new();
        if !file_path.exists() {
            diagnostics.push(LspDiagnostic {
                severity: "error".to_string(),
                message: format!("file not found: {}", file_path.display()),
                line: 0,
                code: None,
            });
            return diagnostics;
        }
        if let Some(ext) = file_path.extension() {
            if ext == "rs" {
                if let Ok(output) = std::process::Command::new("rustc")
                    .arg("--edition")
                    .arg("2021")
                    .arg("--crate-type")
                    .arg("lib")
                    .arg(file_path)
                    .output()
                {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    diagnostics = extract_diagnostics_from_rustc(&stderr);
                }
            }
        }
        diagnostics
    }
}

pub(crate) fn extract_diagnostics_from_rustc(output: &str) -> Vec<LspDiagnostic> {
    let mut diagnostics = Vec::new();
    for line in output.lines() {
        if line.contains("error[") || line.starts_with("error:") {
            let code = if line.contains('[') && line.contains(']') {
                Some(
                    line.split('[')
                        .nth(1)
                        .and_then(|s| s.split(']').next())
                        .unwrap_or("")
                        .to_string(),
                )
            } else {
                None
            };
            let message = line.to_string();
            let line_num = extract_line_number(line);
            diagnostics.push(LspDiagnostic {
                severity: "error".to_string(),
                message,
                line: line_num,
                code,
            });
        } else if line.contains("warning[") || line.starts_with("warning:") {
            let code = if line.contains('[') && line.contains(']') {
                Some(
                    line.split('[')
                        .nth(1)
                        .and_then(|s| s.split(']').next())
                        .unwrap_or("")
                        .to_string(),
                )
            } else {
                None
            };
            let line_num = extract_line_number(line);
            diagnostics.push(LspDiagnostic {
                severity: "warning".to_string(),
                message: line.to_string(),
                line: line_num,
                code,
            });
        }
    }
    diagnostics
}

fn extract_line_number(line: &str) -> usize {
    for part in line.split_whitespace() {
        if let Some(col_idx) = part.rfind(':') {
            if let Ok(n) = part[..col_idx]
                .rsplit(':')
                .next()
                .unwrap_or("0")
                .parse::<usize>()
            {
                return n;
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_available() {
        let caps = OfflineLsp::check_available();
        if which::which("rust-analyzer").is_ok() {
            assert!(caps.contains(&LspCapability::Diagnostics));
            assert!(caps.contains(&LspCapability::Hover));
        } else {
            assert!(caps.is_empty());
        }
    }

    #[test]
    fn test_has_lsp_returns_bool() {
        let _has = OfflineLsp::has_lsp();
    }

    #[test]
    fn test_run_diagnostics_file_not_found() {
        let diags = OfflineLsp::run_diagnostics(Path::new("/nonexistent/file.rs"));
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, "error");
        assert!(diags[0].message.contains("not found"));
    }

    #[test]
    fn test_extract_diagnostics_error_with_code() {
        let sample = "error[E0308]: mismatched types\n  --> src/main.rs:42:16\n";
        let diags = extract_diagnostics_from_rustc(sample);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, "error");
        assert_eq!(diags[0].code.as_deref(), Some("E0308"));
    }

    #[test]
    fn test_extract_diagnostics_error_without_code() {
        let sample = "error: aborting due to previous error\n";
        let diags = extract_diagnostics_from_rustc(sample);
        assert!(!diags.is_empty());
        assert!(diags[0].code.is_none());
    }

    #[test]
    fn test_extract_diagnostics_warning() {
        let sample = "warning[W0123]: unused variable\n  --> src/main.rs:10:5\n";
        let diags = extract_diagnostics_from_rustc(sample);
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, "warning");
        assert_eq!(diags[0].code.as_deref(), Some("W0123"));
    }

    #[test]
    fn test_extract_diagnostics_empty() {
        let diags = extract_diagnostics_from_rustc("");
        assert!(diags.is_empty());
    }

    #[test]
    fn test_extract_diagnostics_noise() {
        let sample = "   Compiling foo v0.1.0\n    Finished dev [unoptimized + debuginfo]\n";
        let diags = extract_diagnostics_from_rustc(sample);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_extract_multiple_diagnostics() {
        let sample = "\
error[E0308]: mismatched types
  --> src/main.rs:42:16
warning[W0123]: unused variable
  --> src/lib.rs:10:5
error: aborting due to previous error
";
        let diags = extract_diagnostics_from_rustc(sample);
        assert_eq!(diags.len(), 3);
        assert_eq!(diags[0].severity, "error");
        assert_eq!(diags[1].severity, "warning");
        assert_eq!(diags[2].severity, "error");
    }

    #[test]
    fn test_run_diagnostics_non_rs_file() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("test.txt");
        std::fs::write(&f, "hello").unwrap();
        let diags = OfflineLsp::run_diagnostics(&f);
        assert!(diags.is_empty());
    }
}
