//! @efficiency-role: util-pure
//! Config validation at startup with clear error messages.

/// Validate that all configuration requirements are met at startup.
/// Returns a list of validation messages: errors (must fix) and warnings (advisory).
pub fn validate_config() -> Vec<String> {
    use std::path::Path;
    let mut messages = Vec::new();

    // Check sessions root exists
    let sessions_root = match crate::paths::sessions_root_path("") {
        Ok(p) => p,
        Err(e) => {
            messages.push(format!("Warning: could not resolve sessions root: {}", e));
            return messages;
        }
    };
    if !sessions_root.exists() {
        messages.push(format!(
            "Info: sessions root '{}' will be created on first use.",
            sessions_root.display()
        ));
    }

    messages
}
