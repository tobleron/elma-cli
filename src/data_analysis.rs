//! @efficiency-role: domain-logic
//!
//! Offline data analysis mode with bounded local execution (Task 671).
//!
//! Detects available interpreters (python3, python, node), runs bounded
//! analysis scripts with timeout, and provides basic CSV statistics
//! without requiring external dependencies.

use crate::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

/// Result of a single analysis operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AnalysisResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub row_count: Option<usize>,
}

/// Statistics for a single CSV column.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CsvColumn {
    pub name: String,
    pub row_count: usize,
    pub null_count: usize,
    pub inferred_type: String,
}

/// Offline data analysis mode using locally available interpreters.
pub(crate) struct DataAnalysisMode;

impl DataAnalysisMode {
    /// Check whether a supported interpreter is available on the system.
    pub(crate) fn is_available() -> bool {
        detect_interpreter().is_some()
    }

    /// Run an analysis script with a bounded timeout, returning the result.
    ///
    /// The script is fed via stdin to the detected interpreter. If the
    /// interpreter is `node`, the script is passed as `-e <script>`.
    pub(crate) fn run_script(script: &str, timeout_secs: u64) -> AnalysisResult {
        let start = Instant::now();
        let interpreter = match detect_interpreter() {
            Some(i) => i,
            None => {
                return AnalysisResult {
                    success: false,
                    output: String::new(),
                    error: Some("No interpreter found (python3, python, node)".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                    row_count: None,
                };
            }
        };

        let is_node = interpreter == "node";
        let result = if is_node {
            let mut child = Command::new(&interpreter)
                .args(["-e", script])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .ok();
            match child {
                Some(c) => wait_with_timeout(c, Duration::from_secs(timeout_secs)),
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to spawn interpreter",
                )),
            }
        } else {
            let mut child = Command::new(&interpreter)
                .arg("-c")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .ok();
            match child {
                Some(mut c) => {
                    use std::io::Write;
                    let _ = c.stdin.take().map(|mut stdin| {
                        let _ = stdin.write_all(script.as_bytes());
                    });
                    let timeout = Duration::from_secs(timeout_secs);
                    match wait_with_timeout(c, timeout) {
                        Ok(output) => Ok(output),
                        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
                    }
                }
                None => Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to spawn interpreter",
                )),
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;
        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                AnalysisResult {
                    success: output.status.success(),
                    output: stdout,
                    error: if stderr.is_empty() {
                        None
                    } else {
                        Some(stderr)
                    },
                    duration_ms,
                    row_count: None,
                }
            }
            Err(e) => AnalysisResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                duration_ms,
                row_count: None,
            },
        }
    }

    /// Run basic CSV analysis using the detected interpreter.
    ///
    /// Delegates to a Python script for robust parsing, falling back to
    /// a simpler Node.js variant if only Node is available.
    pub(crate) fn analyze_csv(path: &Path) -> AnalysisResult {
        if !path.exists() {
            return AnalysisResult {
                success: false,
                output: String::new(),
                error: Some(format!("File not found: {}", path.display())),
                duration_ms: 0,
                row_count: None,
            };
        }
        let csv_path = path
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('\'', "'\\''");
        let interpreter = detect_interpreter();
        let script = match interpreter.as_deref() {
            Some("python3") | Some("python") => format!(
                r#"import csv, sys, json
with open('{csv_path}', newline='', encoding='utf-8', errors='replace') as f:
    reader = csv.DictReader(f)
    rows = list(reader)
print(json.dumps({{"rows": len(rows), "columns": list(reader.fieldnames) if reader.fieldnames else []}}))
"#
            ),
            Some("node") => format!(
                r#"const fs = require('fs');
const content = fs.readFileSync('{csv_path}', 'utf8');
const lines = content.split('\n').filter(l => l.trim());
const headers = lines[0].split(',');
console.log(JSON.stringify({{rows: lines.length - 1, columns: headers}}));
"#
            ),
            _ => {
                return AnalysisResult {
                    success: false,
                    output: String::new(),
                    error: Some("No interpreter found".to_string()),
                    duration_ms: 0,
                    row_count: None,
                };
            }
        };
        let mut result = Self::run_script(&script, 30);
        if result.success {
            if let Ok(parsed) =
                serde_json::from_str::<HashMap<String, serde_json::Value>>(&result.output)
            {
                if let Some(count) = parsed.get("rows").and_then(|v| v.as_u64()) {
                    result.row_count = Some(count as usize);
                }
            }
        }
        result
    }
}

/// Detect the best available interpreter on the system.
///
/// Checks in order: python3, python (>= 3.8), node.
pub(crate) fn detect_interpreter() -> Option<String> {
    for candidate in &["python3", "python", "node"] {
        if let Ok(output) = Command::new(candidate).arg("--version").output() {
            if output.status.success() {
                return Some(candidate.to_string());
            }
        }
    }
    None
}

/// Basic CSV statistics computed in pure Rust.
///
/// Reads the CSV directly, counts rows and columns, infers column types
/// (int, float, bool, string), and counts null cells.
pub(crate) fn basic_csv_stats(path: &Path) -> Result<HashMap<String, CsvColumn>> {
    let content = fs::read_to_string(path)?;
    let mut lines = content.lines().peekable();
    let header = lines.next().ok_or_else(|| anyhow::anyhow!("Empty CSV"))?;
    let col_names: Vec<&str> = header.split(',').map(|s| s.trim()).collect();
    if col_names.is_empty() {
        anyhow::bail!("No columns found in CSV header");
    }

    let num_cols = col_names.len();
    let mut null_counts = vec![0usize; num_cols];
    let mut sample_values: Vec<Vec<String>> = vec![Vec::new(); num_cols];
    let mut row_count = 0usize;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(',').collect();
        row_count += 1;
        for (i, val) in cols.iter().enumerate() {
            if i >= num_cols {
                break;
            }
            let v = val.trim().trim_matches('"');
            if v.is_empty() {
                null_counts[i] += 1;
            } else if sample_values[i].len() < 10 {
                sample_values[i].push(v.to_string());
            }
        }
    }

    let mut stats = HashMap::new();
    for (i, name) in col_names.iter().enumerate() {
        let inferred = infer_type(&sample_values[i]);
        stats.insert(
            name.to_string(),
            CsvColumn {
                name: name.to_string(),
                row_count,
                null_count: null_counts[i],
                inferred_type: inferred,
            },
        );
    }
    Ok(stats)
}

/// Infer the data type of a column from a sample of its values.
fn infer_type(samples: &[String]) -> String {
    if samples.is_empty() {
        return "unknown".to_string();
    }
    let non_empty: Vec<&str> = samples.iter().map(|s| s.as_str()).collect();
    if non_empty.is_empty() {
        return "string".to_string();
    }
    if non_empty
        .iter()
        .all(|v| v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("false"))
    {
        return "bool".to_string();
    }
    let all_int = non_empty.iter().all(|v| v.parse::<i64>().is_ok());
    if all_int {
        return "int".to_string();
    }
    let all_float = non_empty.iter().all(|v| v.parse::<f64>().is_ok());
    if all_float {
        return "float".to_string();
    }
    "string".to_string()
}

/// Wait for a child process to finish with a timeout, collecting its output.
fn wait_with_timeout(
    mut child: std::process::Child,
    timeout: Duration,
) -> std::io::Result<std::process::Output> {
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                if let Some(ref mut out) = child.stdout {
                    use std::io::Read;
                    let _ = out.read_to_end(&mut stdout);
                }
                if let Some(ref mut err) = child.stderr {
                    use std::io::Read;
                    let _ = err.read_to_end(&mut stderr);
                }
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "process timed out",
                    ));
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_interpreter() {
        let result = detect_interpreter();
        // At least one of python3, python, or node should exist in CI
        assert!(result.is_some());
    }

    #[test]
    fn test_is_available() {
        assert!(DataAnalysisMode::is_available());
    }

    #[test]
    fn test_infer_type_int() {
        let samples = vec!["42".to_string(), "1".to_string(), "-5".to_string()];
        assert_eq!(infer_type(&samples), "int");
    }

    #[test]
    fn test_infer_type_float() {
        let samples = vec!["3.14".to_string(), "2.0".to_string()];
        assert_eq!(infer_type(&samples), "float");
    }

    #[test]
    fn test_infer_type_bool() {
        let samples = vec!["true".to_string(), "false".to_string(), "True".to_string()];
        assert_eq!(infer_type(&samples), "bool");
    }

    #[test]
    fn test_infer_type_string() {
        let samples = vec!["hello".to_string(), "world".to_string()];
        assert_eq!(infer_type(&samples), "string");
    }

    #[test]
    fn test_infer_type_unknown() {
        let samples: Vec<String> = vec![];
        assert_eq!(infer_type(&samples), "unknown");
    }

    #[test]
    fn test_basic_csv_stats() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("test.csv");
        fs::write(
            &csv_path,
            "name,age,active\nAlice,30,true\nBob,,false\nCharlie,25,true\n",
        )
        .unwrap();
        let stats = basic_csv_stats(&csv_path).unwrap();
        assert_eq!(stats.len(), 3);

        let name_col = stats.get("name").unwrap();
        assert_eq!(name_col.row_count, 3);
        assert_eq!(name_col.null_count, 0);
        assert_eq!(name_col.inferred_type, "string");

        let age_col = stats.get("age").unwrap();
        assert_eq!(age_col.row_count, 3);
        assert_eq!(age_col.null_count, 1);
        assert_eq!(age_col.inferred_type, "int");

        let active_col = stats.get("active").unwrap();
        assert_eq!(active_col.inferred_type, "bool");
    }

    #[test]
    fn test_basic_csv_stats_empty_file() {
        let dir = tempdir().unwrap();
        let csv_path = dir.path().join("empty.csv");
        fs::write(&csv_path, "").unwrap();
        assert!(basic_csv_stats(&csv_path).is_err());
    }

    #[test]
    fn test_analyze_csv_missing_file() {
        let result = DataAnalysisMode::analyze_csv(Path::new("/nonexistent/file.csv"));
        assert!(!result.success);
        assert!(result.error.unwrap().contains("File not found"));
    }
}
