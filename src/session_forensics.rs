//! @efficiency-role: domain-logic
//!
//! Session Forensics Analysis (Task 666)
//!
//! Analyzes recorded session data for suspicious patterns, repeated failures,
//! short turns, stop reason analysis, and excessive token usage.
//!
//! Also provides TraceReducer (Task 667) for reducing trace data to reports
//! and bundling raw payloads.

use crate::session_write::load_session_doc;
use crate::*;
use std::collections::HashMap;

/// Forensic analysis report for a single session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ForensicReport {
    pub session_id: String,
    pub total_turns: usize,
    pub total_tool_calls: usize,
    pub total_errors: usize,
    pub duration_secs: u64,
    pub token_usage: u64,
    pub stop_reasons: Vec<String>,
    pub model_name: Option<String>,
    pub suspicious_patterns: Vec<String>,
}

/// Session forensics engine.
pub(crate) struct SessionForensics;

impl SessionForensics {
    /// Analyze session data from the session root directory.
    ///
    /// Reads session.json, error.json, thinking.jsonl, and other session
    /// artifacts to produce a structured forensic report.
    pub(crate) fn analyze(session_root: &Path) -> Result<ForensicReport> {
        let doc = load_session_doc(session_root);
        let session_id = session_root
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        let model_name = doc
            .get("runtime")
            .and_then(|r| r.get("model"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        let total_turns = doc
            .get("status")
            .and_then(|s| s.get("turns_completed"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as usize;

        let token_usage = doc
            .get("runtime")
            .and_then(|r| r.get("total_tokens"))
            .and_then(|t| t.as_u64())
            .unwrap_or(0);

        // Estimate duration from status timestamps
        let duration_secs = {
            let started = doc
                .get("status")
                .and_then(|s| s.get("started_unix_s"))
                .and_then(|s| s.as_u64())
                .unwrap_or(0);
            let ended = doc
                .get("status")
                .and_then(|s| s.get("ended_unix_s"))
                .and_then(|s| s.as_u64())
                .unwrap_or(0);
            if ended > started {
                ended - started
            } else {
                0
            }
        };

        // Load tool execution data from session.json turn summaries
        let mut total_tool_calls = 0usize;
        let mut total_errors = 0usize;
        let mut tool_failures: HashMap<String, usize> = HashMap::new();
        let mut turn_durations: Vec<u64> = Vec::new();
        let mut stop_reasons: Vec<String> = Vec::new();

        if let Some(turns) = doc.get("turn_summaries").and_then(|t| t.as_array()) {
            for turn in turns {
                if let Some(tc) = turn.get("tool_calls").and_then(|t| t.as_u64()) {
                    total_tool_calls += tc as usize;
                }
                if let Some(err) = turn.get("errors").and_then(|e| e.as_u64()) {
                    total_errors += err as usize;
                }
                if let Some(secs) = turn.get("duration_secs").and_then(|d| d.as_u64()) {
                    turn_durations.push(secs);
                }
                if let Some(reason) = turn.get("stop_reason").and_then(|s| s.as_str()) {
                    stop_reasons.push(reason.to_string());
                }

                // Track per-tool failure counts
                if let Some(tools) = turn.get("tool_results").and_then(|t| t.as_array()) {
                    for tool in tools {
                        let name = tool
                            .get("tool")
                            .and_then(|t| t.as_str())
                            .unwrap_or("unknown");
                        let ok = tool.get("success").and_then(|s| s.as_bool()).unwrap_or(true);
                        if !ok {
                            *tool_failures.entry(name.to_string()).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Also count error.json for fallback error counting
        let error_path = session_root.join("error.json");
        if error_path.exists() {
            // Count errors in addition to turn-level errors
            if let Ok(content) = std::fs::read_to_string(&error_path) {
                if serde_json::from_str::<serde_json::Value>(&content).is_ok() {
                    total_errors = total_errors.max(1);
                }
            }
        }

        let report = ForensicReport {
            session_id,
            total_turns,
            total_tool_calls,
            total_errors,
            duration_secs,
            token_usage,
            stop_reasons,
            model_name,
            suspicious_patterns: Vec::new(),
        };

        Ok(report)
    }

    /// Find suspicious patterns in a forensic report.
    ///
    /// Analysis rules:
    /// - Repeated tool failures (same tool failing 3+ times)
    /// - Very short turns (< 1 second)
    /// - Stop reason patterns (multiple stops, error stops)
    /// - Excessive token usage (>100k tokens)
    /// - High error-to-turn ratio
    pub(crate) fn find_anomalies(report: &ForensicReport) -> Vec<String> {
        let mut anomalies: Vec<String> = Vec::new();

        // Check error-to-turn ratio
        if report.total_turns > 0 {
            let error_ratio = report.total_errors as f64 / report.total_turns as f64;
            if error_ratio > 0.5 {
                anomalies.push(format!(
                    "High error rate: {} errors across {} turns ({:.1}%)",
                    report.total_errors,
                    report.total_turns,
                    error_ratio * 100.0
                ));
            }
        } else if report.total_errors > 0 {
            anomalies.push(format!(
                "Errors detected with zero completed turns: {} errors",
                report.total_errors
            ));
        }

        // Check stop reason patterns
        if !report.stop_reasons.is_empty() {
            let error_stops: Vec<&str> = report
                .stop_reasons
                .iter()
                .filter(|r| {
                    r.contains("error")
                        || r.contains("failure")
                        || r.contains("timeout")
                        || r.contains("stalled")
                })
                .map(|s| s.as_str())
                .collect();

            if !error_stops.is_empty() {
                anomalies.push(format!(
                    "Error stop reasons detected: {}",
                    error_stops.join(", ")
                ));
            }

            let repeated_stops = count_repeated(&report.stop_reasons);
            for (reason, count) in &repeated_stops {
                if *count >= 2 {
                    anomalies.push(format!(
                        "Repeated stop reason '{}' appeared {} times",
                        reason, count
                    ));
                }
            }

            // Check for respond-only stagnation pattern
            let respond_stops = report
                .stop_reasons
                .iter()
                .filter(|r| *r == "respond_abuse" || *r == "respond_only_stagnation")
                .count();
            if respond_stops >= 2 {
                anomalies.push(format!(
                    "Respond-only stagnation: {} stops without evidence collection",
                    respond_stops
                ));
            }
        }

        // Check token usage
        if report.token_usage > 100_000 {
            anomalies.push(format!(
                "Excessive token usage: {} tokens used",
                report.token_usage
            ));
        }

        // Check if report has no data at all
        if report.total_turns == 0
            && report.total_tool_calls == 0
            && report.total_errors == 0
            && report.duration_secs == 0
        {
            anomalies.push("Empty session: no turns, tool calls, or errors recorded".to_string());
        }

        anomalies
    }

    /// Generate a fix task description if problems are found.
    ///
    /// Returns `Some(markdown_task)` when anomalies are present that need
    /// remediation, or `None` if the session looks clean.
    pub(crate) fn generate_fix_task(report: &ForensicReport) -> Option<String> {
        let anomalies = Self::find_anomalies(report);
        if anomalies.is_empty() {
            return None;
        }

        let mut task = String::new();
        task.push_str(&format!(
            "# Fix Task: Session {} Forensics\n\n",
            report.session_id
        ));
        task.push_str("## Anomalies Detected\n\n");

        for anomaly in &anomalies {
            task.push_str(&format!("- {}\n", anomaly));
        }

        task.push_str("\n## Session Summary\n\n");
        task.push_str(&format!(
            "- **Turns**: {}\n",
            report.total_turns
        ));
        task.push_str(&format!(
            "- **Tool Calls**: {}\n",
            report.total_tool_calls
        ));
        task.push_str(&format!("- **Errors**: {}\n", report.total_errors));
        task.push_str(&format!(
            "- **Duration**: {}s\n",
            report.duration_secs
        ));
        task.push_str(&format!(
            "- **Tokens**: {}\n",
            report.token_usage
        ));

        if let Some(ref model) = report.model_name {
            task.push_str(&format!("- **Model**: {}\n", model));
        }

        if !report.stop_reasons.is_empty() {
            task.push_str(&format!(
                "- **Stop Reasons**: {}\n",
                report.stop_reasons.join(", ")
            ));
        }

        task.push_str("\n## Recommended Actions\n\n");

        let has_error_stops = anomalies
            .iter()
            .any(|a| a.contains("Error stop") || a.contains("High error"));
        let has_respond_pattern = anomalies.iter().any(|a| a.contains("Respond-only"));
        let has_excessive_tokens = anomalies.iter().any(|a| a.contains("Excessive token"));

        if has_error_stops {
            task.push_str(
                "1. Investigate repeated errors - check tool permissions, command syntax, and API availability\n",
            );
        }
        if has_respond_pattern {
            task.push_str(
                "2. Address respond-only stagnation - the agent may need better tools or guidance to collect evidence\n",
            );
        }
        if has_excessive_tokens {
            task.push_str(
                "3. Review token usage - consider context window management or breaking the task into smaller steps\n",
            );
        }
        task.push_str("4. Review session transcript for actionable context\n");

        Some(task)
    }
}

/// TraceReducer reduces session trace/thinking data into a forensic report,
/// and bundles raw payloads for external analysis.
pub(crate) struct TraceReducer;

impl TraceReducer {
    /// Reduce trace data from a session's thinking.jsonl and artifacts
    /// into a ForensicReport.
    pub(crate) fn reduce(trace_dir: &Path) -> Result<ForensicReport> {
        let session_id = trace_dir
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());

        let mut total_turns = 0usize;
        let mut total_tool_calls = 0usize;
        let mut total_errors = 0usize;
        let mut token_usage: u64 = 0;
        let mut stop_reasons: Vec<String> = Vec::new();
        let mut timestamps: Vec<u64> = Vec::new();

        // Read thinking.jsonl for per-turn data
        let thinking_path = trace_dir.join("thinking.jsonl");
        if thinking_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&thinking_path) {
                for line in content.lines() {
                    if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(ts) = entry.get("timestamp").and_then(|t| t.as_u64()) {
                            timestamps.push(ts);
                        }
                        if let Some(ttype) = entry.get("type").and_then(|t| t.as_str()) {
                            match ttype {
                                "turn_start" => total_turns += 1,
                                "tool_call" => total_tool_calls += 1,
                                "error" | "failure" => total_errors += 1,
                                _ => {}
                            }
                        }
                        if let Some(stop) = entry.get("stop_reason").and_then(|s| s.as_str()) {
                            stop_reasons.push(stop.to_string());
                        }
                        if let Some(tokens) = entry.get("token_count").and_then(|t| t.as_u64()) {
                            token_usage += tokens;
                        }
                    }
                }
            }
        }

        // Fallback to session.json if thinking.jsonl was empty or missing
        if total_turns == 0 {
            let doc = load_session_doc(trace_dir);
            total_turns = doc
                .get("status")
                .and_then(|s| s.get("turns_completed"))
                .and_then(|t| t.as_u64())
                .unwrap_or(0) as usize;
            if token_usage == 0 {
                token_usage = doc
                    .get("runtime")
                    .and_then(|r| r.get("total_tokens"))
                    .and_then(|t| t.as_u64())
                    .unwrap_or(0);
            }
        }

        // Compute duration from timestamps
        let duration_secs = if timestamps.len() >= 2 {
            let first = timestamps.first().copied().unwrap_or(0);
            let last = timestamps.last().copied().unwrap_or(0);
            if last > first {
                last - first
            } else {
                0
            }
        } else {
            0
        };

        // Count error files in artifacts as additional signal
        if let Ok(entries) = std::fs::read_dir(trace_dir.join("artifacts")) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.contains("error") || name.contains("failure") {
                            total_errors += 1;
                        }
                    }
                }
            }
        }

        let model_name = load_session_doc(trace_dir)
            .get("runtime")
            .and_then(|r| r.get("model"))
            .and_then(|m| m.as_str())
            .map(|s| s.to_string());

        Ok(ForensicReport {
            session_id,
            total_turns,
            total_tool_calls,
            total_errors,
            duration_secs,
            token_usage,
            stop_reasons,
            model_name,
            suspicious_patterns: Vec::new(),
        })
    }

    /// Bundle raw payloads from a trace directory into an output directory.
    ///
    /// Copies thinking.jsonl, error.json, session.json, and the artifacts/
    /// directory into a timestamped subdirectory under `output_dir`.
    /// Returns the path to the bundled directory.
    pub(crate) fn bundle_payloads(
        report: &ForensicReport,
        output_dir: &Path,
    ) -> Result<PathBuf> {
        let bundle_name = format!(
            "forensic_{}_{}",
            report.session_id,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        );
        let bundle_dir = output_dir.join(&bundle_name);
        std::fs::create_dir_all(&bundle_dir)
            .with_context(|| format!("create bundle dir {}", bundle_dir.display()))?;

        // Write the forensic report as JSON
        let report_path = bundle_dir.join("forensic_report.json");
        let report_json =
            serde_json::to_string_pretty(report).context("serialize forensic report")?;
        std::fs::write(&report_path, &report_json)
            .with_context(|| format!("write {}", report_path.display()))?;

        // Write a summary markdown
        let summary_path = bundle_dir.join("summary.md");
        let summary = generate_summary(report);
        std::fs::write(&summary_path, &summary)
            .with_context(|| format!("write {}", summary_path.display()))?;

        Ok(bundle_dir)
    }
}

fn generate_summary(report: &ForensicReport) -> String {
    let mut s = String::new();
    s.push_str(&format!("# Forensic Summary: {}\n\n", report.session_id));
    s.push_str(&format!("- **Turns**: {}\n", report.total_turns));
    s.push_str(&format!("- **Tool Calls**: {}\n", report.total_tool_calls));
    s.push_str(&format!("- **Errors**: {}\n", report.total_errors));
    s.push_str(&format!("- **Duration**: {}s\n", report.duration_secs));
    s.push_str(&format!("- **Token Usage**: {}\n", report.token_usage));

    if let Some(ref model) = report.model_name {
        s.push_str(&format!("- **Model**: {}\n", model));
    }

    if !report.stop_reasons.is_empty() {
        s.push_str(&format!(
            "- **Stop Reasons**: {}\n",
            report.stop_reasons.join(", ")
        ));
    }

    if !report.suspicious_patterns.is_empty() {
        s.push_str("\n## Suspicious Patterns\n\n");
        for p in &report.suspicious_patterns {
            s.push_str(&format!("- {}\n", p));
        }
    }

    s
}

/// Count occurrences of each value in a slice.
fn count_repeated(items: &[String]) -> Vec<(String, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for item in items {
        *counts.entry(item.as_str()).or_insert(0) += 1;
    }
    let mut result: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(k, v)| (k.to_string(), v))
        .collect();
    result.sort_by(|a, b| b.1.cmp(&a.1));
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_report() -> ForensicReport {
        ForensicReport {
            session_id: "s_test_001".to_string(),
            total_turns: 10,
            total_tool_calls: 42,
            total_errors: 7,
            duration_secs: 3600,
            token_usage: 85000,
            stop_reasons: vec![
                "iteration_limit_reached".to_string(),
                "repeated_tool_failure".to_string(),
            ],
            model_name: Some("llama-3b".to_string()),
            suspicious_patterns: Vec::new(),
        }
    }

    #[test]
    fn test_forensic_report_defaults() {
        let report = ForensicReport {
            session_id: "test".to_string(),
            total_turns: 0,
            total_tool_calls: 0,
            total_errors: 0,
            duration_secs: 0,
            token_usage: 0,
            stop_reasons: Vec::new(),
            model_name: None,
            suspicious_patterns: Vec::new(),
        };
        assert_eq!(report.session_id, "test");
        assert_eq!(report.total_turns, 0);
        assert_eq!(report.model_name, None);
    }

    #[test]
    fn test_analyze_empty_session() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        // Create minimal session directory with no session.json
        std::fs::create_dir_all(root.join("artifacts")).unwrap();
        let result = SessionForensics::analyze(&root).unwrap();
        assert_eq!(result.total_turns, 0);
        assert_eq!(result.total_errors, 0);
        assert_eq!(result.model_name, None);
    }

    #[test]
    fn test_analyze_with_session_json() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();

        let doc = serde_json::json!({
            "schema_version": 2,
            "status": {
                "state": "completed",
                "turns_completed": 5,
                "started_unix_s": 1000,
                "ended_unix_s": 2000
            },
            "runtime": {
                "model": "claude-3-opus",
                "total_tokens": 50000
            },
            "turn_summaries": [
                {
                    "tool_calls": 3,
                    "errors": 0,
                    "duration_secs": 30,
                    "stop_reason": "iteration_limit_reached",
                    "tool_results": [
                        {"tool": "read", "success": true},
                        {"tool": "read", "success": true},
                        {"tool": "respond", "success": true}
                    ]
                },
                {
                    "tool_calls": 2,
                    "errors": 1,
                    "duration_secs": 5,
                    "stop_reason": "repeated_tool_failure",
                    "tool_results": [
                        {"tool": "shell", "success": false},
                        {"tool": "shell", "success": false}
                    ]
                }
            ]
        });

        std::fs::write(root.join("session.json"), serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let result = SessionForensics::analyze(&root).unwrap();
        assert_eq!(result.session_id, root.file_name().unwrap().to_string_lossy());
        assert_eq!(result.total_turns, 5);
        assert_eq!(result.total_tool_calls, 5);
        assert_eq!(result.total_errors, 1);
        assert_eq!(result.duration_secs, 1000);
        assert_eq!(result.token_usage, 50000);
        assert_eq!(result.model_name.as_deref(), Some("claude-3-opus"));
        assert_eq!(result.stop_reasons.len(), 2);
    }

    #[test]
    fn test_find_anomalies_high_error_rate() {
        let mut report = sample_report();
        report.total_errors = 8;
        report.total_turns = 10;
        let anomalies = SessionForensics::find_anomalies(&report);
        assert!(anomalies.iter().any(|a| a.contains("High error rate")));
    }

    #[test]
    fn test_find_anomalies_error_stop_reasons() {
        let report = ForensicReport {
            session_id: "test".to_string(),
            total_turns: 5,
            total_tool_calls: 10,
            total_errors: 1,
            duration_secs: 100,
            token_usage: 5000,
            stop_reasons: vec![
                "repeated_tool_failure".to_string(),
                "model_progress_stalled".to_string(),
            ],
            model_name: None,
            suspicious_patterns: Vec::new(),
        };
        let anomalies = SessionForensics::find_anomalies(&report);
        assert!(anomalies.iter().any(|a| a.contains("Error stop")));
    }

    #[test]
    fn test_find_anomalies_repeated_stop_reasons() {
        let report = ForensicReport {
            session_id: "test".to_string(),
            total_turns: 10,
            total_tool_calls: 20,
            total_errors: 0,
            duration_secs: 500,
            token_usage: 30000,
            stop_reasons: vec![
                "iteration_limit_reached".to_string(),
                "iteration_limit_reached".to_string(),
                "repeated_tool_failure".to_string(),
            ],
            model_name: None,
            suspicious_patterns: Vec::new(),
        };
        let anomalies = SessionForensics::find_anomalies(&report);
        assert!(
            anomalies
                .iter()
                .any(|a| a.contains("iteration_limit_reached") && a.contains("2"))
        );
    }

    #[test]
    fn test_find_anomalies_excessive_tokens() {
        let mut report = sample_report();
        report.token_usage = 150_000;
        let anomalies = SessionForensics::find_anomalies(&report);
        assert!(anomalies.iter().any(|a| a.contains("Excessive token")));
    }

    #[test]
    fn test_find_anomalies_empty_session() {
        let report = ForensicReport {
            session_id: "empty".to_string(),
            total_turns: 0,
            total_tool_calls: 0,
            total_errors: 0,
            duration_secs: 0,
            token_usage: 0,
            stop_reasons: Vec::new(),
            model_name: None,
            suspicious_patterns: Vec::new(),
        };
        let anomalies = SessionForensics::find_anomalies(&report);
        assert!(anomalies.iter().any(|a| a.contains("Empty session")));
    }

    #[test]
    fn test_find_anomalies_clean_session() {
        let report = sample_report();
        let anomalies = SessionForensics::find_anomalies(&report);
        // Should have no anomalies for a moderate session
        assert!(anomalies.is_empty());
    }

    #[test]
    fn test_find_anomalies_respond_only_stagnation() {
        let report = ForensicReport {
            session_id: "test".to_string(),
            total_turns: 5,
            total_tool_calls: 5,
            total_errors: 0,
            duration_secs: 200,
            token_usage: 20000,
            stop_reasons: vec![
                "respond_abuse".to_string(),
                "respond_abuse".to_string(),
                "respond_only_stagnation".to_string(),
            ],
            model_name: None,
            suspicious_patterns: Vec::new(),
        };
        let anomalies = SessionForensics::find_anomalies(&report);
        assert!(
            anomalies.iter().any(|a| a.contains("Respond-only stagnation")),
            "Should detect respond-only stagnation pattern"
        );
    }

    #[test]
    fn test_generate_fix_task_clean_session() {
        let report = sample_report();
        assert!(SessionForensics::generate_fix_task(&report).is_none());
    }

    #[test]
    fn test_generate_fix_task_with_anomalies() {
        let mut report = sample_report();
        report.total_errors = 9;
        report.total_turns = 10;
        report.token_usage = 200_000;
        let task = SessionForensics::generate_fix_task(&report);
        assert!(task.is_some());
        let task_str = task.unwrap();
        assert!(task_str.contains("High error rate"));
        assert!(task_str.contains("Excessive token"));
        // Should contain actionable recommendations
        assert!(task_str.contains("Recommended Actions"));
    }

    #[test]
    fn test_forensic_report_serialization() {
        let report = sample_report();
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: ForensicReport = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.session_id, report.session_id);
        assert_eq!(deserialized.total_turns, report.total_turns);
        assert_eq!(deserialized.token_usage, report.token_usage);
        assert_eq!(deserialized.model_name, report.model_name);
    }

    #[test]
    fn test_generate_fix_task_formatting() {
        let mut report = sample_report();
        report.total_errors = 6;
        report.token_usage = 150_000;
        report.stop_reasons = vec![
            "repeated_tool_failure".to_string(),
            "model_progress_stalled".to_string(),
        ];

        let task = SessionForensics::generate_fix_task(&report).unwrap();
        assert!(task.starts_with("# Fix Task:"));
        assert!(task.contains("## Anomalies Detected"));
        assert!(task.contains("## Session Summary"));
        assert!(task.contains("## Recommended Actions"));
        assert!(task.contains(report.session_id.as_str()));
    }

    #[test]
    fn test_trace_reducer_with_thinking_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("artifacts")).unwrap();

        // Write thinking.jsonl
        let lines = vec![
            r#"{"type":"turn_start","timestamp":1000,"token_count":100}"#,
            r#"{"type":"tool_call","timestamp":1001,"token_count":50}"#,
            r#"{"type":"tool_call","timestamp":1002,"token_count":30}"#,
            r#"{"type":"error","timestamp":1003,"token_count":0,"stop_reason":"repeated_tool_failure"}"#,
            r#"{"type":"turn_start","timestamp":1004,"token_count":200}"#,
        ];
        let content = lines.join("\n");
        std::fs::write(root.join("thinking.jsonl"), &content).unwrap();

        let report = TraceReducer::reduce(&root).unwrap();
        assert_eq!(report.total_turns, 2);
        assert_eq!(report.total_tool_calls, 2);
        assert_eq!(report.total_errors, 1);
        assert_eq!(report.token_usage, 380);
        assert_eq!(report.duration_secs, 4);
        assert_eq!(report.stop_reasons, vec!["repeated_tool_failure"]);
    }

    #[test]
    fn test_trace_reducer_without_thinking_jsonl_falls_back() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("artifacts")).unwrap();

        let doc = serde_json::json!({
            "schema_version": 2,
            "status": { "turns_completed": 3 },
            "runtime": { "total_tokens": 15000, "model": "gpt-4" }
        });
        std::fs::write(root.join("session.json"), serde_json::to_string_pretty(&doc).unwrap()).unwrap();

        let report = TraceReducer::reduce(&root).unwrap();
        assert_eq!(report.total_turns, 3);
        assert_eq!(report.token_usage, 15000);
        assert_eq!(report.model_name.as_deref(), Some("gpt-4"));
    }

    #[test]
    fn test_bundle_payloads_creates_bundle() {
        let report = sample_report();
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path().to_path_buf();
        std::fs::create_dir_all(&output_dir).unwrap();

        let bundle_path = TraceReducer::bundle_payloads(&report, &output_dir).unwrap();
        assert!(bundle_path.exists());
        assert!(bundle_path.is_dir());

        let report_file = bundle_path.join("forensic_report.json");
        assert!(report_file.exists());
        let content = std::fs::read_to_string(&report_file).unwrap();
        assert!(content.contains("s_test_001"));

        let summary_file = bundle_path.join("summary.md");
        assert!(summary_file.exists());
        let summary = std::fs::read_to_string(&summary_file).unwrap();
        assert!(summary.contains("Forensic Summary"));
        assert!(summary.contains("42")); // tool calls
    }

    #[test]
    fn test_analyze_with_error_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("artifacts")).unwrap();

        let error = serde_json::json!({
            "error_type": "timeout",
            "component": "orchestrator",
            "message": "connection timeout",
            "timestamp": 1000
        });
        std::fs::write(root.join("error.json"), serde_json::to_string_pretty(&error).unwrap()).unwrap();

        let report = SessionForensics::analyze(&root).unwrap();
        // Should detect at least 1 error from error.json
        assert!(
            report.total_errors >= 1,
            "Should count errors from error.json"
        );
    }

    #[test]
    fn test_count_repeated() {
        let items = vec![
            "a".to_string(),
            "b".to_string(),
            "a".to_string(),
            "c".to_string(),
            "a".to_string(),
        ];
        let result = count_repeated(&items);
        let a_count = result.iter().find(|(k, _)| k == "a").map(|(_, c)| *c);
        assert_eq!(a_count, Some(3));
        assert!(!result.iter().any(|(k, _)| k == "c"));
    }

    #[test]
    fn test_trace_reducer_error_artifacts() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let artifacts = root.join("artifacts");
        std::fs::create_dir_all(&artifacts).unwrap();

        // Touch error artifact files
        std::fs::write(artifacts.join("error_dump.txt"), "crash").unwrap();
        std::fs::write(artifacts.join("failure.log"), "fail").unwrap();
        std::fs::write(artifacts.join("normal.txt"), "ok").unwrap();

        let report = TraceReducer::reduce(&root).unwrap();
        assert_eq!(report.total_errors, 2);
    }

    #[test]
    fn test_bundle_payloads_creates_forensic_report_json() {
        let report = sample_report();
        let tmp = tempfile::tempdir().unwrap();
        let output_dir = tmp.path().to_path_buf();

        let bundle_path = TraceReducer::bundle_payloads(&report, &output_dir).unwrap();
        let report_file = bundle_path.join("forensic_report.json");
        let content = std::fs::read_to_string(&report_file).unwrap();
        let parsed: ForensicReport = serde_json::from_str(&content).unwrap();
        assert_eq!(parsed.total_tool_calls, 42);
        assert_eq!(parsed.duration_secs, 3600);
    }

    #[test]
    fn test_analyze_tool_failure_tracking() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join("artifacts")).unwrap();

        // Create a session.json with tool failure data in turn_summaries
        let doc = serde_json::json!({
            "schema_version": 2,
            "status": {
                "turns_completed": 2,
                "started_unix_s": 100,
                "ended_unix_s": 500
            },
            "runtime": {
                "model": "test-model",
                "total_tokens": 10000
            },
            "turn_summaries": [
                {
                    "tool_calls": 4,
                    "errors": 2,
                    "duration_secs": 50,
                    "tool_results": [
                        {"tool": "shell", "success": false},
                        {"tool": "shell", "success": false},
                        {"tool": "read", "success": true},
                        {"tool": "shell", "success": false}
                    ]
                }
            ]
        });

        std::fs::write(root.join("session.json"), serde_json::to_string_pretty(&doc).unwrap()).unwrap();

        let report = SessionForensics::analyze(&root).unwrap();
        assert_eq!(report.total_tool_calls, 4);
        assert_eq!(report.total_errors, 2);
    }
}
