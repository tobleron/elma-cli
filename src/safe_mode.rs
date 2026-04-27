//! @efficiency-role: domain-logic
//!
//! Safe Mode Toggle System (Task 272)
//!
//! Provides clear permission levels for shell command execution:
//! - `ask`: Prompt user for permission on destructive commands (default)
//! - `on`: Always require explicit permission, never auto-approve
//! - `off`: Auto-approve all commands (unsafe, trusted environments only)

use crate::*;
use std::sync::OnceLock;

/// Safe mode permission level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum SafeMode {
    /// Prompt user for permission on destructive commands (default)
    Ask,
    /// Always require explicit permission, never auto-approve
    On,
    /// Auto-approve all commands (unsafe)
    Off,
}

impl SafeMode {
    /// Get display string for the mode
    pub fn display(&self) -> &'static str {
        match self {
            SafeMode::Ask => "ask",
            SafeMode::On => "on",
            SafeMode::Off => "off",
        }
    }

    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "ask" => Some(SafeMode::Ask),
            "on" | "strict" => Some(SafeMode::On),
            "off" | "unsafe" => Some(SafeMode::Off),
            _ => None,
        }
    }

    /// Check if this mode allows auto-approval of safe commands
    pub fn allows_safe_auto_approve(&self) -> bool {
        match self {
            SafeMode::Ask | SafeMode::Off => true,
            SafeMode::On => false,
        }
    }

    /// Check if this mode allows auto-approval of caution-level commands
    pub fn allows_caution_auto_approve(&self) -> bool {
        match self {
            SafeMode::Off => true,
            SafeMode::Ask | SafeMode::On => false,
        }
    }

    /// Check if this mode allows auto-approval in non-interactive mode
    pub fn allows_non_interactive_auto(&self) -> bool {
        match self {
            SafeMode::Off | SafeMode::Ask => true,
            SafeMode::On => false,
        }
    }

    /// Get description for user display
    pub fn description(&self) -> &'static str {
        match self {
            SafeMode::Ask => "Prompt for destructive commands (default)",
            SafeMode::On => "Always require explicit permission",
            SafeMode::Off => "Auto-approve all commands (unsafe)",
        }
    }
}

impl Default for SafeMode {
    fn default() -> Self {
        SafeMode::Ask
    }
}

impl std::fmt::Display for SafeMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Global safe mode state
static SAFE_MODE_STATE: OnceLock<Mutex<SafeMode>> = OnceLock::new();

fn safe_mode_state() -> &'static Mutex<SafeMode> {
    SAFE_MODE_STATE.get_or_init(|| Mutex::new(SafeMode::default()))
}

/// Get the current safe mode
pub(crate) fn get_safe_mode() -> SafeMode {
    *safe_mode_state().lock().unwrap_or_else(|e| e.into_inner())
}

/// Set the safe mode
pub(crate) fn set_safe_mode(mode: SafeMode) {
    let mut state = safe_mode_state().lock().unwrap_or_else(|e| e.into_inner());
    *state = mode;
}

/// Parse and set safe mode from string
pub(crate) fn set_safe_mode_from_str(s: &str) -> Result<(), String> {
    match SafeMode::from_str(s) {
        Some(mode) => {
            set_safe_mode(mode);
            Ok(())
        }
        None => Err(format!(
            "Invalid safe mode: '{}'. Use 'ask', 'on', or 'off'",
            s
        )),
    }
}

/// Check if a command should be auto-approved based on safe mode and risk level
pub(crate) fn should_auto_approve(
    command: &str,
    risk_level: &str,
    is_non_interactive: bool,
) -> bool {
    let mode = get_safe_mode();

    let risk_allows = match risk_level {
        "safe" => mode.allows_safe_auto_approve(),
        "caution" => mode.allows_caution_auto_approve(),
        "dangerous" => false,
        _ => false,
    };

    risk_allows && !(is_non_interactive && !mode.allows_non_interactive_auto())
}

/// Get a summary of the current safe mode state for display
pub(crate) fn safe_mode_summary() -> String {
    let mode = get_safe_mode();
    format!("Safe mode: {} ({})", mode.display(), mode.description())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_mode_from_str() {
        assert_eq!(SafeMode::from_str("ask"), Some(SafeMode::Ask));
        assert_eq!(SafeMode::from_str("ASK"), Some(SafeMode::Ask));
        assert_eq!(SafeMode::from_str("on"), Some(SafeMode::On));
        assert_eq!(SafeMode::from_str("strict"), Some(SafeMode::On));
        assert_eq!(SafeMode::from_str("off"), Some(SafeMode::Off));
        assert_eq!(SafeMode::from_str("unsafe"), Some(SafeMode::Off));
        assert_eq!(SafeMode::from_str("invalid"), None);
    }

    #[test]
    fn test_safe_mode_display() {
        assert_eq!(SafeMode::Ask.display(), "ask");
        assert_eq!(SafeMode::On.display(), "on");
        assert_eq!(SafeMode::Off.display(), "off");
    }

    #[test]
    fn test_safe_mode_allows_safe_auto_approve() {
        assert!(SafeMode::Ask.allows_safe_auto_approve());
        assert!(!SafeMode::On.allows_safe_auto_approve());
        assert!(SafeMode::Off.allows_safe_auto_approve());
    }

    #[test]
    fn test_safe_mode_allows_caution_auto_approve() {
        assert!(!SafeMode::Ask.allows_caution_auto_approve());
        assert!(!SafeMode::On.allows_caution_auto_approve());
        assert!(SafeMode::Off.allows_caution_auto_approve());
    }

    #[test]
    fn test_safe_mode_allows_non_interactive_auto() {
        assert!(SafeMode::Ask.allows_non_interactive_auto());
        assert!(!SafeMode::On.allows_non_interactive_auto());
        assert!(SafeMode::Off.allows_non_interactive_auto());
    }

    #[test]
    fn test_should_auto_approve_safe_command() {
        // Reset to default
        set_safe_mode(SafeMode::Ask);
        assert!(should_auto_approve("ls", "safe", false));

        set_safe_mode(SafeMode::On);
        assert!(!should_auto_approve("ls", "safe", false));

        set_safe_mode(SafeMode::Off);
        assert!(should_auto_approve("ls", "safe", false));
    }

    #[test]
    fn test_should_auto_approve_caution_command() {
        set_safe_mode(SafeMode::Ask);
        assert!(!should_auto_approve("rm file.txt", "caution", false));

        set_safe_mode(SafeMode::Off);
        assert!(should_auto_approve("rm file.txt", "caution", false));
    }

    #[test]
    fn test_should_auto_approve_dangerous_never_auto() {
        set_safe_mode(SafeMode::Ask);
        assert!(!should_auto_approve("rm -rf /", "dangerous", false));

        set_safe_mode(SafeMode::Off);
        assert!(!should_auto_approve("rm -rf /", "dangerous", false));
    }

    #[test]
    fn test_set_safe_mode_from_str() {
        assert!(set_safe_mode_from_str("ask").is_ok());
        assert_eq!(get_safe_mode(), SafeMode::Ask);

        assert!(set_safe_mode_from_str("on").is_ok());
        assert_eq!(get_safe_mode(), SafeMode::On);

        assert!(set_safe_mode_from_str("invalid").is_err());
    }

    #[test]
    fn test_safe_mode_summary() {
        set_safe_mode(SafeMode::Ask);
        let summary = safe_mode_summary();
        assert!(summary.contains("ask"));
        assert!(summary.contains("Prompt"));
    }

    #[test]
    fn test_safe_mode_default() {
        set_safe_mode(SafeMode::default());
        assert_eq!(get_safe_mode(), SafeMode::Ask);
    }

    #[test]
    fn test_safe_mode_display_trait() {
        assert_eq!(format!("{}", SafeMode::Ask), "ask");
        assert_eq!(format!("{}", SafeMode::On), "on");
        assert_eq!(format!("{}", SafeMode::Off), "off");
    }
}
