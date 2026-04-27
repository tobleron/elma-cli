//! @efficiency-role: ui-component
//!
//! Claude Code-style Status Line
//!
//! Conditional status display:
//! - Model name
//! - Token usage
//! - Workspace
//! - Approval policy

use crate::ui_theme::*;

// ============================================================================
// Status Line
// ============================================================================

#[derive(Clone, Debug, Default)]
pub(crate) struct StatusLine {
    pub model: Option<String>,
    pub tokens_used: Option<u64>,
    pub tokens_max: Option<u64>,
    pub workspace: Option<String>,
    pub policy: Option<String>,
    pub visible: bool,
}

impl StatusLine {
    pub(crate) fn new() -> Self {
        Self {
            model: None,
            tokens_used: None,
            tokens_max: None,
            workspace: None,
            policy: None,
            visible: false,
        }
    }

    pub(crate) fn set_model(&mut self, model: String) {
        self.model = Some(model);
        self.visible = true;
    }

    pub(crate) fn set_tokens(&mut self, used: u64, max: u64) {
        self.tokens_used = Some(used);
        self.tokens_max = Some(max);
        self.visible = true;
    }

    pub(crate) fn set_workspace(&mut self, workspace: String) {
        self.workspace = Some(workspace);
    }

    pub(crate) fn set_policy(&mut self, policy: String) {
        self.policy = Some(policy);
    }

    pub(crate) fn show(&mut self) {
        self.visible = true;
    }

    pub(crate) fn hide(&mut self) {
        self.visible = false;
    }

    pub(crate) fn render(&self) -> String {
        if !self.visible {
            return String::new();
        }

        let mut parts = Vec::new();

        if let Some(ref model) = self.model {
            parts.push(info_cyan(model));
        }

        if let (Some(used), Some(max)) = (self.tokens_used, self.tokens_max) {
            let pct = if max > 0 {
                (used as f64 / max as f64 * 100.0).round() as u64
            } else {
                0
            };
            parts.push(meta_comment(&format!("{}/{} ({}%)", used, max, pct)));
        }

        if let Some(ref workspace) = self.workspace {
            let ws = if workspace.len() > 30 {
                let start = if workspace.len() >= 30 {
                    let pos = workspace.len() - 30;
                    if workspace.is_char_boundary(pos) {
                        pos
                    } else {
                        let mut p = pos;
                        while !workspace.is_char_boundary(p) {
                            p += 1;
                        }
                        p
                    }
                } else {
                    0
                };
                format!("…{}", &workspace[start..])
            } else {
                workspace.clone()
            };
            parts.push(meta_comment(&ws));
        }

        if let Some(ref policy) = self.policy {
            if policy == "auto" {
                parts.push(dim("(auto)"));
            } else if policy == "confirm" {
                parts.push(elma_accent("(confirm)"));
            }
        }

        if parts.is_empty() {
            return String::new();
        }

        parts.join("  ")
    }

    pub(crate) fn token_pct(&self) -> Option<u64> {
        match (self.tokens_used, self.tokens_max) {
            (Some(used), Some(max)) if max > 0 => {
                Some((used as f64 / max as f64 * 100.0).round() as u64)
            }
            _ => None,
        }
    }
}

// ============================================================================
// Notification
// ============================================================================

#[derive(Clone, Debug)]
pub(crate) enum NotificationKind {
    Info,
    Warning,
    Error,
    Success,
}

#[derive(Clone, Debug)]
pub(crate) struct Notification {
    pub message: String,
    pub kind: NotificationKind,
    pub timestamp: u64,
}

impl Notification {
    pub(crate) fn info(message: String) -> Self {
        Self {
            message,
            kind: NotificationKind::Info,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub(crate) fn warn(message: String) -> Self {
        Self {
            message,
            kind: NotificationKind::Warning,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }

    pub(crate) fn render(&self) -> String {
        match self.kind {
            NotificationKind::Info => format!("ℹ {}", meta_comment(&self.message)),
            NotificationKind::Warning => format!("⚠ {}", warn_yellow(&self.message)),
            NotificationKind::Error => format!("✗ {}", error_red(&self.message)),
            NotificationKind::Success => format!("✓ {}", success_green(&self.message)),
        }
    }

    pub(crate) fn is_expired(&self, ttl_ms: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        now.saturating_sub(self.timestamp) > ttl_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_line() {
        let mut s = StatusLine::new();
        s.set_model("llama-3b".to_string());
        assert!(s.visible);
        let line = s.render();
        assert!(!line.is_empty());
    }

    #[test]
    fn test_token_percentage() {
        let mut s = StatusLine::new();
        s.set_tokens(1000, 4000);
        assert_eq!(s.token_pct(), Some(25));
    }

    #[test]
    fn test_notification() {
        let n = Notification::info("Test".to_string());
        let line = n.render();
        assert!(line.starts_with("ℹ"));
    }
}
