//! @efficiency-role: domain-logic
//!
//! Replayable trace reducer and raw payload bundle (Task 667).
//! Reads trace/thinking events from a trace directory and produces
//! a structured summary. Also bundles raw payloads for external analysis.

use crate::*;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TraceEvent {
    pub timestamp: u64,
    pub turn_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub payload: String,
    pub payload_ref: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ReducedSummary {
    pub turn_count: usize,
    pub tool_call_count: usize,
    pub total_tokens: u64,
    pub duration_secs: u64,
    pub key_events: Vec<String>,
}

pub(crate) struct TraceReducer;

impl TraceReducer {
    pub(crate) fn reduce(trace_dir: &Path) -> Vec<TraceEvent> {
        let mut events = Vec::new();
        let thinking_path = trace_dir.join("thinking.jsonl");
        if !thinking_path.exists() {
            return events;
        }
        let content = match std::fs::read_to_string(&thinking_path) {
            Ok(c) => c,
            Err(_) => return events,
        };
        for line in content.lines() {
            if let Ok(entry) = serde_json::from_str::<serde_json::Value>(line) {
                let timestamp = entry.get("timestamp").and_then(|t| t.as_u64()).unwrap_or(0);
                let turn_id = entry
                    .get("turn_id")
                    .and_then(|t| t.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let event_type = entry
                    .get("type")
                    .and_then(|t| t.as_str())
                    .unwrap_or("notice")
                    .to_string();
                let payload = entry
                    .get("payload")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string();
                let payload = if payload.len() > 1000 {
                    format!("{}...", &payload[..1000])
                } else {
                    payload
                };
                let payload_ref = entry
                    .get("payload_ref")
                    .and_then(|r| r.as_str())
                    .map(PathBuf::from);
                events.push(TraceEvent {
                    timestamp,
                    turn_id,
                    event_type,
                    payload,
                    payload_ref,
                });
            }
        }
        events
    }

    pub(crate) fn summarize(events: &[TraceEvent]) -> ReducedSummary {
        let turn_count = events
            .iter()
            .filter(|e| e.event_type == "turn_start" || e.event_type == "turn_end")
            .count();
        let tool_call_count = events
            .iter()
            .filter(|e| e.event_type == "tool_call" || e.event_type == "tool_result")
            .count();
        let total_tokens = 0;
        let mut key_events: Vec<String> = Vec::new();
        for event in events.iter().take(10) {
            key_events.push(format!(
                "[{}] {}: {}",
                event.timestamp, event.event_type, event.turn_id
            ));
        }
        let timestamps: Vec<u64> = events.iter().map(|e| e.timestamp).collect();
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
        ReducedSummary {
            turn_count,
            tool_call_count,
            total_tokens,
            duration_secs,
            key_events,
        }
    }
}

pub(crate) struct PayloadBundle;

impl PayloadBundle {
    pub(crate) fn bundle(trace_dir: &Path, output_path: &Path) -> Result<PathBuf> {
        let bundle_dir = output_path.join(format!(
            "trace_bundle_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        ));
        std::fs::create_dir_all(&bundle_dir)
            .with_context(|| format!("create bundle dir {}", bundle_dir.display()))?;

        let thinking_path = trace_dir.join("thinking.jsonl");
        if thinking_path.exists() {
            let dest = bundle_dir.join("thinking.jsonl");
            std::fs::copy(&thinking_path, &dest)
                .with_context(|| format!("copy thinking.jsonl to {}", dest.display()))?;
        }

        let artifacts_dir = trace_dir.join("artifacts");
        if artifacts_dir.exists() {
            let dest_artifacts = bundle_dir.join("artifacts");
            std::fs::create_dir_all(&dest_artifacts)?;
            for entry in std::fs::read_dir(&artifacts_dir).into_iter().flatten() {
                if let Ok(e) = entry {
                    let file_path = e.path();
                    if file_path.is_file() {
                        let dest_file =
                            dest_artifacts.join(file_path.file_name().unwrap_or_default());
                        let _ = std::fs::copy(&file_path, &dest_file);
                    }
                }
            }
        }

        let session_path = trace_dir.join("session.json");
        if session_path.exists() {
            let dest = bundle_dir.join("session.json");
            let _ = std::fs::copy(&session_path, &dest);
        }

        Ok(bundle_dir)
    }

    pub(crate) fn bundle_size(path: &Path) -> u64 {
        let mut total = 0u64;
        if path.is_file() {
            if let Ok(meta) = path.metadata() {
                total += meta.len();
            }
        } else if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.flatten() {
                    total += Self::bundle_size(&entry.path());
                }
            }
        }
        total
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_events() -> Vec<TraceEvent> {
        vec![
            TraceEvent {
                timestamp: 1000,
                turn_id: "turn_1".into(),
                event_type: "turn_start".into(),
                payload: String::new(),
                payload_ref: None,
            },
            TraceEvent {
                timestamp: 1001,
                turn_id: "turn_1".into(),
                event_type: "tool_call".into(),
                payload: "read file".into(),
                payload_ref: None,
            },
            TraceEvent {
                timestamp: 1002,
                turn_id: "turn_1".into(),
                event_type: "tool_result".into(),
                payload: "ok".into(),
                payload_ref: None,
            },
            TraceEvent {
                timestamp: 2000,
                turn_id: "turn_2".into(),
                event_type: "turn_start".into(),
                payload: String::new(),
                payload_ref: None,
            },
        ]
    }

    #[test]
    fn test_reduce_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let events = TraceReducer::reduce(tmp.path());
        assert!(events.is_empty());
    }

    #[test]
    fn test_reduce_with_thinking_jsonl() {
        let tmp = tempfile::tempdir().unwrap();
        let lines = vec![
            r#"{"type":"turn_start","turn_id":"t1","timestamp":1000,"payload":"hello"}"#,
            r#"{"type":"tool_call","turn_id":"t1","timestamp":1001,"payload":"ls"}"#,
        ];
        std::fs::write(tmp.path().join("thinking.jsonl"), lines.join("\n")).unwrap();
        let events = TraceReducer::reduce(tmp.path());
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "turn_start");
        assert_eq!(events[0].turn_id, "t1");
    }

    #[test]
    fn test_summarize_counts() {
        let events = sample_events();
        let summary = TraceReducer::summarize(&events);
        assert_eq!(summary.turn_count, 2);
        assert_eq!(summary.tool_call_count, 2);
        assert!(summary.duration_secs > 0);
    }

    #[test]
    fn test_summarize_empty() {
        let events = Vec::new();
        let summary = TraceReducer::summarize(&events);
        assert_eq!(summary.turn_count, 0);
        assert_eq!(summary.tool_call_count, 0);
        assert_eq!(summary.duration_secs, 0);
    }

    #[test]
    fn test_bundle_creates_dir() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();
        let path = PayloadBundle::bundle(src.path(), dst.path()).unwrap();
        assert!(path.exists());
        assert!(path.is_dir());
    }

    #[test]
    fn test_bundle_size_file() {
        let tmp = tempfile::tempdir().unwrap();
        let f = tmp.path().join("test.txt");
        std::fs::write(&f, "hello world").unwrap();
        let size = PayloadBundle::bundle_size(&f);
        assert_eq!(size, 11);
    }

    #[test]
    fn test_bundle_size_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "abc").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "defgh").unwrap();
        let size = PayloadBundle::bundle_size(tmp.path());
        assert_eq!(size, 8);
    }

    #[test]
    fn test_payload_truncation() {
        let tmp = tempfile::tempdir().unwrap();
        let long_payload = "x".repeat(2000);
        let line = format!(
            r#"{{"type":"tool_call","turn_id":"t1","timestamp":0,"payload":"{}"}}"#,
            long_payload
        );
        std::fs::write(tmp.path().join("thinking.jsonl"), &line).unwrap();
        let events = TraceReducer::reduce(tmp.path());
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload.len(), 1003); // 1000 + "..."
    }
}
