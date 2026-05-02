use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::llm_provider::LlmProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum CapabilitySupport {
    #[serde(rename = "supported")]
    Supported,
    #[serde(rename = "unsupported")]
    Unsupported,
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for CapabilitySupport {
    fn default() -> Self {
        CapabilitySupport::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum TokenizerKind {
    #[serde(rename = "tiktoken")]
    Tiktoken,
    #[serde(rename = "cl100k")]
    Cl100kBase,
    #[serde(rename = "anthropic")]
    Anthropic,
    #[serde(rename = "huggingface")]
    HuggingFace,
    #[serde(rename = "estimator")]
    Estimator,
    #[serde(rename = "none")]
    None,
}

impl Default for TokenizerKind {
    fn default() -> Self {
        TokenizerKind::Estimator
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) enum CapabilitySource {
    #[serde(rename = "override")]
    UserOverride,
    #[serde(rename = "config")]
    ConfigFolder,
    #[serde(rename = "builtin")]
    BuiltIn,
    #[serde(rename = "probe")]
    ProbeResult,
    #[serde(rename = "fallback")]
    Fallback,
}

impl Default for CapabilitySource {
    fn default() -> Self {
        CapabilitySource::Fallback
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct ModelCapabilities {
    pub model_id: String,
    #[serde(default)]
    pub provider_family: Option<LlmProvider>,
    #[serde(default)]
    pub context_window_tokens: Option<u32>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub supports_tools: CapabilitySupport,
    #[serde(default)]
    pub supports_streaming: CapabilitySupport,
    #[serde(default)]
    pub supports_logprobs: CapabilitySupport,
    #[serde(default)]
    pub supports_reasoning_format_auto: CapabilitySupport,
    #[serde(default)]
    pub supports_reasoning_format_none: CapabilitySupport,
    #[serde(default)]
    pub tokenizer: TokenizerKind,
    #[serde(default)]
    pub source: CapabilitySource,
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self {
            model_id: String::new(),
            provider_family: None,
            context_window_tokens: None,
            max_output_tokens: None,
            supports_tools: CapabilitySupport::Unknown,
            supports_streaming: CapabilitySupport::Unknown,
            supports_logprobs: CapabilitySupport::Unknown,
            supports_reasoning_format_auto: CapabilitySupport::Unknown,
            supports_reasoning_format_none: CapabilitySupport::Unknown,
            tokenizer: TokenizerKind::Estimator,
            source: CapabilitySource::Fallback,
        }
    }
}

impl ModelCapabilities {
    pub fn fallback(model_id: &str) -> Self {
        Self {
            model_id: model_id.to_string(),
            provider_family: None,
            context_window_tokens: Some(4096),
            max_output_tokens: Some(2048),
            supports_tools: CapabilitySupport::Supported,
            supports_streaming: CapabilitySupport::Supported,
            supports_logprobs: CapabilitySupport::Unknown,
            supports_reasoning_format_auto: CapabilitySupport::Unknown,
            supports_reasoning_format_none: CapabilitySupport::Supported,
            tokenizer: TokenizerKind::Estimator,
            source: CapabilitySource::Fallback,
        }
    }
}

static MODEL_CAPABILITIES: OnceLock<ModelCapabilities> = OnceLock::new();

pub(crate) fn get_model_capabilities() -> &'static ModelCapabilities {
    MODEL_CAPABILITIES.get_or_init(|| ModelCapabilities::fallback("unknown"))
}

pub(crate) fn set_model_capabilities(caps: ModelCapabilities) {
    let _ = MODEL_CAPABILITIES.set(caps);
}

pub(crate) fn capability_config_path(config_root: &Path) -> PathBuf {
    config_root.join("model_capabilities.toml")
}

pub(crate) fn resolve_model_capabilities(
    config_root: &Path,
    model_id: &str,
    provider: Option<LlmProvider>,
) -> ModelCapabilities {
    let config_path = capability_config_path(config_root);

    if config_path.exists() {
        if let Ok(bytes) = std::fs::read(&config_path) {
            if let Ok(s) = String::from_utf8(bytes) {
                if let Ok(override_caps) = toml::from_str::<ModelCapabilities>(&s) {
                    if override_caps.model_id == model_id {
                        return override_caps;
                    }
                }
            }
        }
    }

    if let Some(p) = provider {
        let caps = built_in_capabilities(model_id, p);
        if caps.source != CapabilitySource::Fallback {
            return caps;
        }
    }

    ModelCapabilities::fallback(model_id)
}

fn built_in_capabilities(model_id: &str, provider: LlmProvider) -> ModelCapabilities {
    let lower = model_id.to_lowercase();

    if lower.contains("gpt-4") || lower.contains("gpt-4o") || lower.contains("gpt-4-turbo") {
        return ModelCapabilities {
            model_id: model_id.to_string(),
            provider_family: Some(provider),
            context_window_tokens: Some(128_000),
            max_output_tokens: Some(16_384),
            supports_tools: CapabilitySupport::Supported,
            supports_streaming: CapabilitySupport::Supported,
            supports_logprobs: CapabilitySupport::Supported,
            supports_reasoning_format_auto: CapabilitySupport::Supported,
            supports_reasoning_format_none: CapabilitySupport::Supported,
            tokenizer: TokenizerKind::Cl100kBase,
            source: CapabilitySource::BuiltIn,
        };
    }

    if lower.contains("gpt-3.5") || lower.contains("gpt-35") {
        return ModelCapabilities {
            model_id: model_id.to_string(),
            provider_family: Some(provider),
            context_window_tokens: Some(16_385),
            max_output_tokens: Some(4_096),
            supports_tools: CapabilitySupport::Supported,
            supports_streaming: CapabilitySupport::Supported,
            supports_logprobs: CapabilitySupport::Supported,
            supports_reasoning_format_auto: CapabilitySupport::Unsupported,
            supports_reasoning_format_none: CapabilitySupport::Supported,
            tokenizer: TokenizerKind::Cl100kBase,
            source: CapabilitySource::BuiltIn,
        };
    }

    if lower.contains("claude-3") || lower.contains("claude-3-5") || lower.contains("sonnet") || lower.contains("haiku") || lower.contains("opus") {
        return ModelCapabilities {
            model_id: model_id.to_string(),
            provider_family: Some(provider),
            context_window_tokens: Some(200_000),
            max_output_tokens: Some(4_096),
            supports_tools: CapabilitySupport::Supported,
            supports_streaming: CapabilitySupport::Supported,
            supports_logprobs: CapabilitySupport::Unknown,
            supports_reasoning_format_auto: CapabilitySupport::Supported,
            supports_reasoning_format_none: CapabilitySupport::Supported,
            tokenizer: TokenizerKind::Anthropic,
            source: CapabilitySource::BuiltIn,
        };
    }

    if lower.contains("llama-3") || lower.contains("llama-4") {
        return ModelCapabilities {
            model_id: model_id.to_string(),
            provider_family: Some(provider),
            context_window_tokens: Some(128_000),
            max_output_tokens: Some(4_096),
            supports_tools: CapabilitySupport::Supported,
            supports_streaming: CapabilitySupport::Supported,
            supports_logprobs: CapabilitySupport::Supported,
            supports_reasoning_format_auto: CapabilitySupport::Supported,
            supports_reasoning_format_none: CapabilitySupport::Supported,
            tokenizer: TokenizerKind::Estimator,
            source: CapabilitySource::BuiltIn,
        };
    }

    if lower.contains("qwen") {
        return ModelCapabilities {
            model_id: model_id.to_string(),
            provider_family: Some(provider),
            context_window_tokens: Some(32_768),
            max_output_tokens: Some(4_096),
            supports_tools: CapabilitySupport::Supported,
            supports_streaming: CapabilitySupport::Supported,
            supports_logprobs: CapabilitySupport::Unknown,
            supports_reasoning_format_auto: CapabilitySupport::Unknown,
            supports_reasoning_format_none: CapabilitySupport::Supported,
            tokenizer: TokenizerKind::Estimator,
            source: CapabilitySource::BuiltIn,
        };
    }

    ModelCapabilities::fallback(model_id)
}

pub(crate) fn clamp_max_tokens(requested: u32, capabilities: &ModelCapabilities) -> u32 {
    let cap = capabilities.max_output_tokens.unwrap_or(4096);
    requested.min(cap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_capabilities() {
        let caps = ModelCapabilities::fallback("unknown-model");
        assert_eq!(caps.source, CapabilitySource::Fallback);
        assert_eq!(caps.context_window_tokens, Some(4096));
    }

    #[test]
    fn test_clamp_max_tokens() {
        let caps = ModelCapabilities {
            model_id: "test".to_string(),
            max_output_tokens: Some(4096),
            ..Default::default()
        };
        assert_eq!(clamp_max_tokens(8192, &caps), 4096);
        assert_eq!(clamp_max_tokens(2048, &caps), 2048);
    }

    #[test]
    fn test_built_in_gpt4() {
        let caps = built_in_capabilities("gpt-4-turbo", LlmProvider::OpenAI);
        assert_eq!(caps.source, CapabilitySource::BuiltIn);
        assert_eq!(caps.context_window_tokens, Some(128_000));
    }

    #[test]
    fn test_built_in_claude() {
        let caps = built_in_capabilities("claude-3-5-sonnet-20241022", LlmProvider::Anthropic);
        assert_eq!(caps.source, CapabilitySource::BuiltIn);
        assert_eq!(caps.supports_tools, CapabilitySupport::Supported);
    }

    #[test]
    fn test_built_in_llama() {
        let caps = built_in_capabilities("Llama-3.1-70B-Instruct", LlmProvider::OpenAICompatible);
        assert_eq!(caps.source, CapabilitySource::BuiltIn);
    }

    #[test]
    fn test_token_count_estimator() {
        let caps = ModelCapabilities::default();
        // "hello world" = 2 tokens in cl100k
        let count = token_count("hello world", &caps);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_context_window() {
        let caps = ModelCapabilities {
            context_window_tokens: Some(128_000),
            ..Default::default()
        };
        assert_eq!(context_window_tokens(&caps), 128_000);
    }

    #[test]
    fn test_context_window_fallback() {
        let caps = ModelCapabilities::default();
        // No context window set, should fall back to 8192
        assert_eq!(context_window_tokens(&caps), 8192);
    }
}

pub(crate) fn token_count(text: &str, capabilities: &ModelCapabilities) -> usize {
    crate::token_counter::count_tokens_for_model(text, capabilities.tokenizer)
}

pub(crate) fn context_window_tokens(capabilities: &ModelCapabilities) -> usize {
    capabilities.context_window_tokens.unwrap_or(8192) as usize
}