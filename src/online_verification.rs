//! @efficiency-role: domain-logic
//!
//! Online Verification Policy And Tool Routing — Task 694.
//!
//! Represents online verification as a capability requirement in the
//! work graph. If network is disabled, the final answer must disclose
//! that verification was not performed. If enabled, the fetch/network
//! tool is selected and cited references are verified with evidence.

use crate::*;
use std::collections::HashSet;
use std::sync::{OnceLock, RwLock};

/// Set whether network is enabled for the current session.
pub(crate) fn set_network_enabled(enabled: bool) {
    let state = crate::session_state::get_session_state();
    let mut lock = match state.network_disabled.write() {
        Ok(l) => l,
        Err(_) => return,
    };
    *lock = !enabled;
}

/// Check if network verification is available.
pub(crate) fn is_online_verification_available() -> bool {
    let state = crate::session_state::get_session_state();
    let lock = match state.network_disabled.read() {
        Ok(l) => l,
        Err(_) => return false,
    };
    !*lock
}

/// Check if a user request contains online verification requirements.
/// Looks for patterns indicating the user wants to verify references,
/// check for updates, confirm API availability, etc.
pub(crate) fn request_requires_online_verification(user_request: &str) -> bool {
    let lower = user_request.to_lowercase();
    let online_patterns = [
        "verify",
        "check if",
        "confirm",
        "up-to-date",
        "up to date",
        "latest version",
        "current version",
        "is it still",
        "are they still",
        "api status",
        "online",
        "website",
        "url",
        "http",
        "https://",
        "fetch",
        "crates.io",
        "docs.rs",
        "github.com",
    ];
    online_patterns.iter().any(|p| lower.contains(p))
}

/// Patterns that indicate a final answer claims online verification happened.
pub(crate) static ONLINE_CLAIM_PATTERNS: &[&str] = &[
    "verified that",
    "confirmed that",
    "are current",
    "are up-to-date",
    "are up to date",
    "latest version",
    "check was performed",
    "online check",
    "api is available",
    "is still maintained",
    "is still active",
];

/// Check if a final answer contains claims that require online verification.
pub(crate) fn answer_claims_online_verification(answer: &str) -> Vec<&'static str> {
    let lower = answer.to_lowercase();
    ONLINE_CLAIM_PATTERNS
        .iter()
        .filter(|p| lower.contains(*p))
        .copied()
        .collect()
}

/// Check if there's evidence of a network/fetch tool call in the session.
pub(crate) fn has_network_evidence(messages: &[ChatMessage]) -> bool {
    messages.iter().any(|m| {
        if m.role == "tool" && m.name.as_deref() == Some("fetch") {
            return true;
        }
        if let Some(calls) = &m.tool_calls {
            if calls.iter().any(|c| c.function.name == "fetch") {
                return true;
            }
        }
        false
    })
}

/// Attempt to detect URLs in a user request that should be verified.
pub(crate) fn extract_urls_for_verification(user_request: &str) -> Vec<String> {
    let mut urls = Vec::new();
    for word in user_request.split_whitespace() {
        let clean = word.trim().trim_matches(|c: char| {
            c == '\'' || c == '"' || c == '`' || c == '(' || c == ')' || c == '[' || c == ']'
        });
        if clean.starts_with("http://") || clean.starts_with("https://") {
            let url_str = clean.to_string();
            if !urls.contains(&url_str) {
                urls.push(url_str);
            }
        }
    }
    urls
}

/// Validate a final answer for online verification honesty.
/// Returns corrections to append if unsupported claims are found.
pub(crate) fn validate_online_claims(
    final_answer: &str,
    messages: &[ChatMessage],
) -> Option<String> {
    let claims = answer_claims_online_verification(final_answer);
    if claims.is_empty() {
        return None;
    }

    let has_network = has_network_evidence(messages);
    let network_enabled = is_online_verification_available();

    if !has_network {
        let matched_claims: Vec<&str> = claims.iter().map(|c| *c).collect();
        let details = if !network_enabled {
            "Network access is disabled by policy. Online verification was not performed."
        } else {
            "No network/fetch tool calls were made during this session."
        };

        Some(format!(
            "\n\n**Online Verification Notice:**\n\
             The answer above makes claims suggesting online verification was performed:\n\
             {}\n\n\
             {}\n\
             These claims should be considered unverified.",
            matched_claims.join(", "),
            details
        ))
    } else {
        None
    }
}

/// Build a tool-routing hint for including the fetch tool when online verification is needed.
pub(crate) fn online_verification_tool_hint() -> Option<String> {
    if !is_online_verification_available() {
        return Some(
            "Note: Online verification is not available (network disabled by policy). \
             Local analysis will be performed instead."
                .to_string(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    static NET_TEST_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_network_disabled_by_default() {
        let _guard = NET_TEST_MUTEX.lock().unwrap();
        set_network_enabled(false);
        assert!(!is_online_verification_available());
    }

    #[test]
    fn test_network_enabled() {
        let _guard = NET_TEST_MUTEX.lock().unwrap();
        set_network_enabled(true);
        assert!(is_online_verification_available());
        set_network_enabled(false);
    }

    #[test]
    fn test_request_requires_online() {
        assert!(request_requires_online_verification(
            "Verify that the API documentation is up-to-date"
        ));
        assert!(!request_requires_online_verification(
            "List the files in the src directory"
        ));
    }

    #[test]
    fn test_answer_claims_online() {
        let answer = "I verified that all dependencies are current.";
        let claims = answer_claims_online_verification(answer);
        assert!(!claims.is_empty());
        assert!(claims.contains(&"verified that"));

        let clean = "The project has 42 source files.";
        assert!(answer_claims_online_verification(clean).is_empty());
    }

    #[test]
    fn test_has_network_evidence_with_fetch() {
        let msg = ChatMessage {
            role: "tool".to_string(),
            content: "200 OK".to_string(),
            name: Some("fetch".to_string()),
            tool_calls: None,
            tool_call_id: Some("t1".to_string()),
            reasoning_content: None,
            summarized: false,
        };
        assert!(has_network_evidence(&[msg]));
    }

    #[test]
    fn test_has_network_evidence_without_fetch() {
        let msg = ChatMessage {
            role: "tool".to_string(),
            content: "src/main.rs".to_string(),
            name: Some("read".to_string()),
            tool_calls: None,
            tool_call_id: Some("t1".to_string()),
            reasoning_content: None,
            summarized: false,
        };
        assert!(!has_network_evidence(&[msg]));
    }

    #[test]
    fn test_extract_urls_from_request() {
        let request = "Check https://crates.io/api/v1/crates/serde and https://docs.rs/serde";
        let urls = extract_urls_for_verification(request);
        assert_eq!(urls.len(), 2);
        assert!(urls.contains(&"https://crates.io/api/v1/crates/serde".to_string()));
        assert!(urls.contains(&"https://docs.rs/serde".to_string()));
    }

    #[test]
    fn test_no_urls_in_normal_request() {
        let request = "List files in src directory";
        let urls = extract_urls_for_verification(request);
        assert!(urls.is_empty());
    }

    #[test]
    fn test_validate_online_claims_no_claims() {
        let answer = "Found 3 TODO comments.";
        let result = validate_online_claims(answer, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_validate_online_claims_unsupported() {
        let answer = "Verified that all API references are current and secure.";
        let result = validate_online_claims(answer, &[]);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.contains("Online Verification Notice"));
        assert!(result.contains("verified that"));
    }

    #[test]
    fn test_validate_online_claims_supported() {
        let answer = "Verified that the API is current.";
        let msg = ChatMessage {
            role: "tool".to_string(),
            content: "200 OK".to_string(),
            name: Some("fetch".to_string()),
            tool_calls: None,
            tool_call_id: Some("t1".to_string()),
            reasoning_content: None,
            summarized: false,
        };
        let result = validate_online_claims(answer, &[msg]);
        assert!(result.is_none());
    }

    #[test]
    fn test_tool_hint_when_disabled() {
        let _guard = NET_TEST_MUTEX.lock().unwrap();
        set_network_enabled(false);
        assert!(online_verification_tool_hint().is_some());
    }

    #[test]
    fn test_tool_hint_when_enabled() {
        let _guard = NET_TEST_MUTEX.lock().unwrap();
        set_network_enabled(true);
        assert!(online_verification_tool_hint().is_none());
        set_network_enabled(false);
    }

    #[test]
    fn test_request_pattern_covers_common_cases() {
        let cases = [
            ("verify that the docs are correct", true),
            ("check if the API is still maintained", true),
            ("confirm the latest version of tokio", true),
            ("list all rust files", false),
            ("what does this function do", false),
        ];
        for (request, expected) in &cases {
            assert_eq!(
                request_requires_online_verification(request),
                *expected,
                "Failed for: {}",
                request
            );
        }
    }
}
