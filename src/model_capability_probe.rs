//! @efficiency-role: infra-adapter
//!
//! Model Capability Probe — probes model/provider capabilities and returns
//! a structured capability report.
//!
//! Task 643: Model capability probing

use crate::*;

/// Runtime behavior class inferred from observed model output, not config names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ModelRuntimeKind {
    Thinking,
    Dense,
    Unknown,
}

impl std::fmt::Display for ModelRuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelRuntimeKind::Thinking => write!(f, "thinking"),
            ModelRuntimeKind::Dense => write!(f, "dense"),
            ModelRuntimeKind::Unknown => write!(f, "unknown"),
        }
    }
}

/// Result of a capability probe for a given provider + model combination.
#[derive(Debug, Clone)]
pub(crate) struct ProbeResult {
    pub supports_tool_calling: bool,
    pub supports_streaming: bool,
    pub supports_thinking: bool,
    pub supports_json_mode: bool,
    pub max_output_tokens: Option<u64>,
    pub context_window_tokens: Option<u64>,
    pub runtime_kind: ModelRuntimeKind,
    pub provider_name: String,
    pub model_name: String,
}

/// Probes model/provider capabilities and returns a structured capability report.
#[derive(Debug, Clone)]
pub(crate) struct ModelCapabilityProbe;

impl ModelCapabilityProbe {
    /// Probe a model's capabilities by provider + model name.
    /// First checks known capabilities, then performs lightweight live probing
    /// for unrecognized models.
    pub(crate) fn probe(provider: &str, model: &str) -> ProbeResult {
        let mut result = known_capabilities(model);
        result.provider_name = provider.to_string();
        result.model_name = model.to_string();
        result
    }

    /// Refine static/name-based capabilities with live endpoint facts.
    pub(crate) fn refine_with_runtime(
        probe: &mut ProbeResult,
        behavior: &ModelBehaviorProfile,
        context_window_tokens: Option<u64>,
    ) {
        let runtime_kind = classify_model_runtime_kind(behavior);
        probe.runtime_kind = runtime_kind;
        probe.context_window_tokens = context_window_tokens;
        if runtime_kind == ModelRuntimeKind::Thinking {
            probe.supports_thinking = true;
        }
    }
}

/// Infer model type from observed response behavior.
pub(crate) fn classify_model_runtime_kind(behavior: &ModelBehaviorProfile) -> ModelRuntimeKind {
    if behavior.auto_reasoning_separated
        || behavior.auto_truncated_before_final
        || behavior.needs_text_finalizer
    {
        return ModelRuntimeKind::Thinking;
    }
    if behavior.none_final_clean || behavior.json_clean_with_none || behavior.json_clean_with_auto {
        return ModelRuntimeKind::Dense;
    }
    ModelRuntimeKind::Unknown
}

/// Adapter that modifies API requests/streams based on probed capabilities.
#[derive(Debug, Clone)]
pub(crate) struct ProviderResponseAdapter;

impl ProviderResponseAdapter {
    /// Adapt an API request body based on probed capabilities.
    /// Removes or modifies fields the model does not support.
    pub(crate) fn adapt_request(request: &mut serde_json::Value, probe: &ProbeResult) {
        if !probe.supports_tool_calling {
            if let Some(obj) = request.as_object_mut() {
                obj.remove("tools");
                obj.remove("tool_choice");
            }
        }

        if !probe.supports_streaming {
            if let Some(obj) = request.as_object_mut() {
                obj.insert("stream".to_string(), serde_json::Value::Bool(false));
            }
        }

        if !probe.supports_thinking {
            if let Some(obj) = request.as_object_mut() {
                obj.remove("reasoning_content");
                obj.remove("thinking");
            }
        }

        if probe.supports_json_mode {
            if let Some(obj) = request.as_object_mut() {
                if !obj.contains_key("response_format") {
                    obj.insert(
                        "response_format".to_string(),
                        serde_json::json!({"type": "json_object"}),
                    );
                }
            }
        }
    }

    /// Adapt a streaming event based on probed capabilities.
    /// Normalizes/cleans streaming response data per provider conventions.
    pub(crate) fn adapt_stream_event(event: &mut serde_json::Value, probe: &ProbeResult) {
        if !probe.supports_thinking {
            if let Some(obj) = event.as_object_mut() {
                obj.remove("reasoning_content");
            }
        }
    }
}

/// Returns known capabilities for common model families.
/// Falls back to conservative defaults for unrecognized models.
pub(crate) fn known_capabilities(model: &str) -> ProbeResult {
    let lower = model.to_lowercase();

    // Granite models — tools + streaming, no thinking
    if lower.contains("granite") {
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: false,
            max_output_tokens: Some(4096),
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // Qwen models — tools + streaming, optional thinking (think/reason/r1 variants)
    if lower.contains("qwen") {
        let has_thinking =
            lower.contains("think") || lower.contains("reason") || lower.contains("r1");
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: has_thinking,
            supports_json_mode: true,
            max_output_tokens: Some(8192),
            context_window_tokens: None,
            runtime_kind: if has_thinking {
                ModelRuntimeKind::Thinking
            } else {
                ModelRuntimeKind::Unknown
            },
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // DeepSeek models — tools + streaming, thinking on reasoner/r1 variants
    if lower.contains("deepseek") {
        let has_thinking = lower.contains("r1") || lower.contains("reasoner");
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: has_thinking,
            supports_json_mode: true,
            max_output_tokens: Some(8192),
            context_window_tokens: None,
            runtime_kind: if has_thinking {
                ModelRuntimeKind::Thinking
            } else {
                ModelRuntimeKind::Unknown
            },
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // GPT-4 models — full support, thinking only on o-series/reasoning models
    if lower.contains("gpt-4") || lower.contains("gpt-4o") || lower.contains("chatgpt-4o") {
        let has_thinking = lower.contains("o1") || lower.contains("o3") || lower.contains("reason");
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: has_thinking,
            supports_json_mode: true,
            max_output_tokens: Some(16384),
            context_window_tokens: None,
            runtime_kind: if has_thinking {
                ModelRuntimeKind::Thinking
            } else {
                ModelRuntimeKind::Unknown
            },
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // GPT-3.5 models — tools + streaming + json, no thinking
    if lower.contains("gpt-3.5") || lower.contains("gpt-35") {
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: true,
            max_output_tokens: Some(4096),
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Dense,
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // Claude models — full support including thinking
    if lower.contains("claude") {
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: true,
            supports_json_mode: true,
            max_output_tokens: Some(4096),
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Thinking,
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // Llama models — tools + streaming, no thinking/json
    if lower.contains("llama") {
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: false,
            max_output_tokens: Some(4096),
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // Gemini models — tools + streaming + json, no thinking
    if lower.contains("gemini") {
        return ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: true,
            max_output_tokens: Some(8192),
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Dense,
            provider_name: String::new(),
            model_name: model.to_string(),
        };
    }

    // Conservative defaults for unknown/unrecognized models
    ProbeResult {
        supports_tool_calling: false,
        supports_streaming: true,
        supports_thinking: false,
        supports_json_mode: false,
        max_output_tokens: Some(2048),
        context_window_tokens: None,
        runtime_kind: ModelRuntimeKind::Unknown,
        provider_name: String::new(),
        model_name: model.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_capabilities_granite() {
        let r = known_capabilities("granite-3.1-8b-instruct");
        assert!(r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(!r.supports_thinking);
        assert!(!r.supports_json_mode);
        assert_eq!(r.max_output_tokens, Some(4096));
    }

    #[test]
    fn test_known_capabilities_qwen_no_thinking() {
        let r = known_capabilities("qwen2.5-7b-instruct");
        assert!(r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(!r.supports_thinking);
        assert!(r.supports_json_mode);
    }

    #[test]
    fn test_known_capabilities_qwen_with_thinking() {
        let r = known_capabilities("qwen3-8b-thinking");
        assert!(r.supports_thinking);
    }

    #[test]
    fn test_known_capabilities_deepseek_no_thinking() {
        let r = known_capabilities("deepseek-chat");
        assert!(r.supports_tool_calling);
        assert!(!r.supports_thinking);
    }

    #[test]
    fn test_known_capabilities_deepseek_with_thinking() {
        let r = known_capabilities("deepseek-reasoner");
        assert!(r.supports_thinking);
    }

    #[test]
    fn test_known_capabilities_gpt4() {
        let r = known_capabilities("gpt-4-turbo");
        assert!(r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(!r.supports_thinking);
        assert!(r.supports_json_mode);
        assert_eq!(r.max_output_tokens, Some(16384));
    }

    #[test]
    fn test_known_capabilities_claude() {
        let r = known_capabilities("claude-3-5-sonnet-20241022");
        assert!(r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(r.supports_thinking);
        assert!(r.supports_json_mode);
    }

    #[test]
    fn test_known_capabilities_unknown() {
        let r = known_capabilities("some-unknown-model-v1");
        assert!(!r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(!r.supports_thinking);
        assert!(!r.supports_json_mode);
        assert_eq!(r.max_output_tokens, Some(2048));
    }

    #[test]
    fn test_probe_uses_known_capabilities() {
        let r = ModelCapabilityProbe::probe("openai", "gpt-4-turbo");
        assert!(r.supports_tool_calling);
        assert_eq!(r.provider_name, "openai");
        assert_eq!(r.model_name, "gpt-4-turbo");
    }

    #[test]
    fn test_adapt_request_removes_tools_when_unsupported() {
        let probe = ProbeResult {
            supports_tool_calling: false,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: false,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut req = serde_json::json!({
            "tools": [{"type": "function", "function": {"name": "test"}}],
            "tool_choice": "auto",
            "stream": true
        });
        ProviderResponseAdapter::adapt_request(&mut req, &probe);
        assert!(req.get("tools").is_none());
        assert!(req.get("tool_choice").is_none());
        assert_eq!(req["stream"], true);
    }

    #[test]
    fn test_adapt_request_disables_streaming() {
        let probe = ProbeResult {
            supports_tool_calling: true,
            supports_streaming: false,
            supports_thinking: false,
            supports_json_mode: false,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut req = serde_json::json!({"stream": true});
        ProviderResponseAdapter::adapt_request(&mut req, &probe);
        assert_eq!(req["stream"], false);
    }

    #[test]
    fn test_adapt_request_adds_json_mode() {
        let probe = ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: true,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut req = serde_json::json!({});
        ProviderResponseAdapter::adapt_request(&mut req, &probe);
        assert_eq!(req["response_format"]["type"], "json_object");
    }

    #[test]
    fn test_adapt_request_removes_thinking_when_unsupported() {
        let probe = ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: false,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut req = serde_json::json!({
            "reasoning_content": true,
            "thinking": {"budget_tokens": 1024}
        });
        ProviderResponseAdapter::adapt_request(&mut req, &probe);
        assert!(req.get("reasoning_content").is_none());
        assert!(req.get("thinking").is_none());
    }

    #[test]
    fn test_adapt_stream_event_removes_reasoning_when_unsupported() {
        let probe = ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: false,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut event = serde_json::json!({
            "choices": [{"delta": {"content": "hello"}}],
            "reasoning_content": "thinking..."
        });
        ProviderResponseAdapter::adapt_stream_event(&mut event, &probe);
        assert!(event.get("reasoning_content").is_none());
    }

    #[test]
    fn test_adapt_stream_event_keeps_reasoning_when_supported() {
        let probe = ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: true,
            supports_json_mode: false,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Thinking,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut event = serde_json::json!({
            "choices": [{"delta": {"content": "hello"}}],
            "reasoning_content": "thinking..."
        });
        ProviderResponseAdapter::adapt_stream_event(&mut event, &probe);
        assert_eq!(event["reasoning_content"], "thinking...");
    }

    #[test]
    fn test_known_capabilities_llama() {
        let r = known_capabilities("llama-3.1-70b-instruct");
        assert!(r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(!r.supports_thinking);
        assert!(!r.supports_json_mode);
    }

    #[test]
    fn test_known_capabilities_gemini() {
        let r = known_capabilities("gemini-2.0-flash");
        assert!(r.supports_tool_calling);
        assert!(r.supports_streaming);
        assert!(!r.supports_thinking);
        assert!(r.supports_json_mode);
        assert_eq!(r.max_output_tokens, Some(8192));
    }

    #[test]
    fn test_adapt_request_does_not_overwrite_existing_response_format() {
        let probe = ProbeResult {
            supports_tool_calling: true,
            supports_streaming: true,
            supports_thinking: false,
            supports_json_mode: true,
            max_output_tokens: None,
            context_window_tokens: None,
            runtime_kind: ModelRuntimeKind::Unknown,
            provider_name: "test".into(),
            model_name: "test".into(),
        };
        let mut req = serde_json::json!({"response_format": {"type": "text"}});
        ProviderResponseAdapter::adapt_request(&mut req, &probe);
        assert_eq!(req["response_format"]["type"], "text");
    }

    #[test]
    fn classifies_thinking_from_observed_reasoning_behavior() {
        let behavior = ModelBehaviorProfile {
            version: 3,
            model: "local".to_string(),
            base_url: "http://localhost:8080".to_string(),
            auto_reasoning_separated: true,
            auto_final_clean: false,
            auto_truncated_before_final: false,
            none_final_clean: true,
            none_reasoning_leak_suspected: false,
            json_clean_with_auto: true,
            json_clean_with_none: true,
            needs_text_finalizer: true,
            preferred_reasoning_format: "auto".to_string(),
        };
        assert_eq!(
            classify_model_runtime_kind(&behavior),
            ModelRuntimeKind::Thinking
        );
    }

    #[test]
    fn refines_probe_with_live_context_and_runtime_kind() {
        let behavior = ModelBehaviorProfile {
            version: 3,
            model: "local".to_string(),
            base_url: "http://localhost:8080".to_string(),
            auto_reasoning_separated: false,
            auto_final_clean: true,
            auto_truncated_before_final: false,
            none_final_clean: true,
            none_reasoning_leak_suspected: false,
            json_clean_with_auto: true,
            json_clean_with_none: true,
            needs_text_finalizer: false,
            preferred_reasoning_format: "none".to_string(),
        };
        let mut probe = ModelCapabilityProbe::probe("openai_compatible", "unknown-local");
        ModelCapabilityProbe::refine_with_runtime(&mut probe, &behavior, Some(131072));
        assert_eq!(probe.runtime_kind, ModelRuntimeKind::Dense);
        assert_eq!(probe.context_window_tokens, Some(131072));
    }
}
