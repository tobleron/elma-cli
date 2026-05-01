//! @efficiency-role: domain-logic
//!
//! Capability discovery intel unit: identifies what capability is needed
//! from a user request, then searches the tool registry for matching tools.
//! Output complies with Task 378 constraints (3 fields max, no nesting).

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

/// Capability request output — simple 3-field JSON per Task 378 constraints.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CapabilityRequest {
    /// One-sentence description of the capability needed
    pub(crate) capability: String,
    /// Confidence in the capability assessment
    pub(crate) confidence: String,
    /// Brief reason for this capability selection
    pub(crate) reason: String,
}

/// Capability Discovery Intel Unit
///
/// Given a user request, identifies the capability needed to fulfill it.
/// The output is used to search the tool registry for matching tools.
pub(crate) struct CapabilityDiscoveryUnit {
    profile: Profile,
}

impl CapabilityDiscoveryUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }

    pub async fn discover_capability(
        &self,
        client: &reqwest::Client,
        chat_url: &Url,
        user_message: &str,
    ) -> Result<CapabilityRequest> {
        let req = build_intel_system_user_request(
            &self.profile,
            format!(
                "User request: {}\n\nWhat capability is needed? Respond with JSON.",
                user_message
            ),
        );
        let chat_url = Url::parse(&self.profile.base_url)
            .map_err(|e| anyhow::anyhow!("Invalid base_url '{}': {}", self.profile.base_url, e))?
            .join("/v1/chat/completions")
            .map_err(|e| anyhow::anyhow!("Failed to build chat URL: {}", e))?;
        let response =
            chat_once_with_timeout(client, &chat_url, &req, self.profile.timeout_s).await?;
        let response_text = extract_response_text(&response);
        let json = crate::json_parser::extract_json_object(&response_text)
            .ok_or_else(|| anyhow::anyhow!("No JSON in capability response"))?;
        Ok(serde_json::from_value(json)?)
    }
}

impl IntelUnit for CapabilityDiscoveryUnit {
    fn name(&self) -> &'static str {
        "capability_discovery"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let result: CapabilityRequest = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            context.user_message.clone(),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.85,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get_str("capability").is_none_or(|s| s.is_empty()) {
            return Err(anyhow::anyhow!("Missing capability field"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "capability": "unknown",
                "confidence": "low",
                "reason": format!("fallback: {}", error),
            }),
            &format!("capability discovery failed: {}", error),
        ))
    }
}

/// Search the tool registry for tools matching a capability query,
/// ranked by implementation kind priority (rust-native first, shell last).
pub(crate) fn find_tools_for_capability(
    query: &str,
    max_results: usize,
) -> Vec<elma_tools::ToolDefinition> {
    let registry = crate::tool_registry::get_registry();
    let mut results: Vec<_> = registry
        .search(query)
        .into_iter()
        .filter(|t| t.is_available())
        .collect();

    // Sort by implementation kind priority (rust-native first, shell last for offline)
    results.sort_by(|a, b| {
        let pa = a.implementation_kind.selection_priority();
        let pb = b.implementation_kind.selection_priority();
        pb.cmp(&pa)
    });

    results.truncate(max_results);
    results.into_iter().map(|t| t.to_tool_definition()).collect()
}

/// Attempt to discover and load tools for a capability need.
/// Returns the list of loaded tool definitions.
pub(crate) async fn auto_discover_tools(
    client: &reqwest::Client,
    chat_url: &Url,
    profile: &Profile,
    user_message: &str,
) -> Vec<elma_tools::ToolDefinition> {
    let unit = CapabilityDiscoveryUnit::new(profile.clone());
    match unit.discover_capability(client, chat_url, user_message).await {
        Ok(cap) => {
            let tools = find_tools_for_capability(&cap.capability, 5);
            if !tools.is_empty() {
                let names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
                crate::tool_registry::mark_discovered(&names);
            }
            tools
        }
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_request_deserialization() {
        let json = serde_json::json!({
            "capability": "search file contents for a pattern",
            "confidence": "high",
            "reason": "user wants to find text in files"
        });
        let req: CapabilityRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.capability, "search file contents for a pattern");
        assert_eq!(req.confidence, "high");
        assert!(!req.reason.is_empty());
    }

    #[test]
    fn test_capability_request_requires_all_fields() {
        let json = serde_json::json!({
            "capability": "search",
            "confidence": "high"
        });
        let result: Result<CapabilityRequest, _> = serde_json::from_value(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_find_tools_for_capability_returns_ranked_results() {
        let tools = find_tools_for_capability("read file contents", 5);
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.function.name == "read"));
    }

    #[test]
    fn test_find_tools_for_capability_rust_native_preferred() {
        let tools = find_tools_for_capability("list directory files", 5);
        assert!(!tools.is_empty());
        // ls (rust-native, priority 100) should rank above shell (priority 40)
        let ls_pos = tools.iter().position(|t| t.function.name == "ls");
        let shell_pos = tools.iter().position(|t| t.function.name == "shell");
        if let (Some(ls_idx), Some(shell_idx)) = (ls_pos, shell_pos) {
            assert!(
                ls_idx < shell_idx,
                "Rust-native 'ls' should rank above 'shell'"
            );
        }
    }

    #[test]
    fn test_find_tools_for_capability_respects_max_results() {
        let tools = find_tools_for_capability("file", 3);
        assert!(tools.len() <= 3);
    }

    #[test]
    fn test_find_tools_for_capability_no_match() {
        let tools = find_tools_for_capability("xzylvmbrkwpt", 5);
        assert!(tools.is_empty());
    }

    #[test]
    fn test_capability_discovery_unit_creation() {
        let profile = Profile {
            version: 1,
            name: "test".to_string(),
            base_url: "http://localhost".to_string(),
            model: "test".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            repeat_penalty: 1.0,
            reasoning_format: "none".to_string(),
            max_tokens: 128,
            timeout_s: 30,
            system_prompt: "Identify the capability needed.".to_string(),
        };
        let unit = CapabilityDiscoveryUnit::new(profile);
        assert_eq!(unit.name(), "capability_discovery");
    }
}
