//! @efficiency-role: data-model
//!
//! Session - Error Reporting (Task 018)

use crate::*;

/// Session error types for structured error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    let path = session_root.join("error.json");
    let json = serde_json::to_string_pretty(error).context("serialize session error")?;
    std::fs::write(&path, json)
        .with_context(|| format!("write error report {}", path.display()))?;
    Ok(path)
}

/// Write session status marker
pub(crate) fn write_session_status(
    session_root: &PathBuf,
    status: &str,
    turns_completed: u32,
    last_turn: Option<&str>,
    error_summary: Option<&str>,
) -> Result<PathBuf> {
    let path = session_root.join("session_status.json");
    let status_obj = serde_json::json!({
        "status": status,
        "turns_completed": turns_completed,
        "last_turn": last_turn,
        "error_summary": error_summary,
        "ended_unix_s": SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    });
    let json = serde_json::to_string_pretty(&status_obj).context("serialize session status")?;
    std::fs::write(&path, json)
        .with_context(|| format!("write session status {}", path.display()))?;
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
