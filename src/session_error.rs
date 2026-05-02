//! @efficiency-role: data-model
//!
//! Session - Error Reporting (Task 018)
//!
//! Also handles session index updates on status changes (Task 282)

use crate::session_index::{self, build_index_entry};
use crate::session_write::mutate_session_doc;
use crate::*;

/// Session error types for structured error reporting
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionErrorType {
    Timeout,
    ParseError,
    ApiError,
    Panic,
    IoError,
    Unknown,
}

/// Structured session error for crash reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionError {
    pub error_type: SessionErrorType,
    pub component: String,
    pub message: String,
    pub timestamp: u64,
    pub last_action: Option<String>,
    pub context: serde_json::Value,
}

impl SessionError {
    pub(crate) fn timeout(component: &str, message: &str, last_action: Option<String>) -> Self {
        SessionError {
            error_type: SessionErrorType::Timeout,
            component: component.to_string(),
            message: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_action,
            context: serde_json::json!({}),
        }
    }

    pub(crate) fn api_error(component: &str, message: &str, last_action: Option<String>) -> Self {
        SessionError {
            error_type: SessionErrorType::ApiError,
            component: component.to_string(),
            message: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_action,
            context: serde_json::json!({}),
        }
    }

    pub(crate) fn parse_error(component: &str, message: &str, last_action: Option<String>) -> Self {
        SessionError {
            error_type: SessionErrorType::ParseError,
            component: component.to_string(),
            message: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_action,
            context: serde_json::json!({}),
        }
    }

    pub(crate) fn panic(component: &str, message: &str, last_action: Option<String>) -> Self {
        SessionError {
            error_type: SessionErrorType::Panic,
            component: component.to_string(),
            message: message.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            last_action,
            context: serde_json::json!({}),
        }
    }
}

/// Write error.json to session directory for crash reporting
pub(crate) fn write_session_error(session_root: &PathBuf, error: &SessionError) -> Result<PathBuf> {
    // Primary: store in session.json.status.error
    let _ = mutate_session_doc(session_root, |doc| {
        if doc.get("status").is_none() {
            doc["status"] = serde_json::json!({});
        }
        doc["status"]["error"] = serde_json::to_value(error).expect("serialize error");
    });

    // Legacy: also write error.json for crash survivability
    let path = session_root.join("error.json");
    let json = serde_json::to_string_pretty(error).context("serialize session error")?;
    std::fs::write(&path, json)
        .with_context(|| format!("write error report {}", path.display()))?;
    Ok(path)
}

/// Write session status marker and update index
pub(crate) fn write_session_status(
    session_root: &PathBuf,
    status: &str,
    turns_completed: u32,
    last_turn: Option<&str>,
    error_summary: Option<&str>,
) -> Result<PathBuf> {
    let ended_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Primary: store in session.json.status
    let _ = mutate_session_doc(session_root, |doc| {
        let state = serde_json::json!({
            "state": status,
            "turns_completed": turns_completed,
            "last_turn": last_turn,
            "error_summary": error_summary,
            "ended_unix_s": ended_unix_s,
        });
        if doc.get("status").is_none() {
            doc["status"] = serde_json::json!({});
        }
        doc["status"]["state"] = serde_json::json!(status);
        doc["status"]["turns_completed"] = serde_json::json!(turns_completed);
        doc["status"]["last_turn"] = serde_json::json!(last_turn);
        doc["status"]["error_summary"] = serde_json::json!(error_summary);
        doc["status"]["ended_unix_s"] = serde_json::json!(ended_unix_s);
    });

    // Legacy: also write session_status.json
    let path = session_root.join("session_status.json");
    let status_obj = serde_json::json!({
        "status": status,
        "turns_completed": turns_completed,
        "last_turn": last_turn,
        "error_summary": error_summary,
        "ended_unix_s": ended_unix_s,
    });
    let json = serde_json::to_string_pretty(&status_obj).context("serialize session status")?;
    std::fs::write(&path, json)
        .with_context(|| format!("write session status {}", path.display()))?;

    // Update session index (fire-and-forget, don't fail the write if index update fails)
    if let Some(entry) = session_index::build_index_entry(
        session_root,
        &session_root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
    ) {
        let sessions_root = session_root.parent().unwrap_or(session_root);
        if let Ok(mut index) = session_index::SessionIndex::load(sessions_root) {
            index.upsert_entry(entry);
            let _ = index.save(sessions_root);
        }
    }

    Ok(path)
}

/// Install panic hook for crash reporting
pub(crate) fn install_panic_hook(session_root: Option<PathBuf>) {
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = panic_info
            .location()
            .map(|loc| format!("{}:{}:{}", loc.file(), loc.line(), loc.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let full_message = format!("Panic at {}: {}", location, message);

        eprintln!("\n❌ FATAL: {}", full_message);

        if let Some(ref root) = session_root {
            let error = SessionError::panic("runtime", &full_message, None);
            if let Ok(path) = write_session_error(root, &error) {
                eprintln!("   Error report: {}", path.display());
            }
            let _ = write_session_status(root, "error", 0, None, Some(&message));
        }

        if let Some(ref root) = session_root {
            let trace_path = root.join("trace_debug.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&trace_path)
            {
                use std::io::Write;
                let _ = writeln!(file, "[PANIC] {}", full_message);
            }
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn session_error_factory_timeout() {
        let err = SessionError::timeout(
            "orchestrator",
            "connection timeout",
            Some("calling model".into()),
        );
        assert_eq!(err.error_type, SessionErrorType::Timeout);
        assert_eq!(err.component, "orchestrator");
        assert!(err.message.contains("timeout"));
    }

    #[test]
    fn session_error_factory_api_error() {
        let err =
            SessionError::api_error("api_client", "rate limited", Some("chat completion".into()));
        assert_eq!(err.error_type, SessionErrorType::ApiError);
        assert_eq!(err.component, "api_client");
    }

    #[test]
    fn session_error_factory_parse_error() {
        let err = SessionError::parse_error("json_handler", "invalid JSON", None);
        assert_eq!(err.error_type, SessionErrorType::ParseError);
    }

    #[test]
    fn session_error_factory_panic() {
        let err = SessionError::panic(
            "runtime",
            "index out of bounds",
            Some("reading artifact".into()),
        );
        assert_eq!(err.error_type, SessionErrorType::Panic);
        assert!(err.message.contains("bounds"));
    }

    #[test]
    fn write_error_json_creates_file() {
        let root = std::env::temp_dir().join(format!(
            "elma_test_err_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&root).ok();
        let err = SessionError::api_error("test", "test error", None);
        let path = write_session_error(&root, &err).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("api_error"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn write_session_status_creates_file() {
        let root = std::env::temp_dir().join(format!(
            "elma_test_stat_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        std::fs::create_dir_all(&root).ok();
        let path =
            write_session_status(&root, "error", 3, Some("test prompt"), Some("timeout")).unwrap();
        assert!(path.exists());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("status"));
        assert!(content.contains("error"));
        let _ = std::fs::remove_dir_all(&root);
    }
}
