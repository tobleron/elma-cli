//! @efficiency-role: infra-config
//!
//! Central runtime configuration and request construction for llama.cpp-compatible calls.

use crate::*;

static LLM_RUNTIME_CONFIG: OnceLock<LlmRuntimeConfig> = OnceLock::new();
static SAVED_BASE_URL: OnceLock<String> = OnceLock::new();

pub(crate) fn set_saved_base_url(url: &str) {
    let _ = SAVED_BASE_URL.set(url.to_string());
}

fn saved_base_url() -> &'static str {
    SAVED_BASE_URL
        .get()
        .map(|s| s.as_str())
        .unwrap_or("http://localhost:8080")
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct LlmRuntimeConfig {
    pub(crate) version: u32,
    pub(crate) http_timeout_s: u64,
    pub(crate) request_timeout_s: u64,
    pub(crate) final_answer_timeout_s: u64,
    pub(crate) tool_loop_timeout_s: u64,
    pub(crate) model_probe_timeout_s: u64,
    pub(crate) max_response_tokens_cap: u32,
    pub(crate) tool_loop_max_tokens_cap: u32,
    pub(crate) model_probe_logprobs_n_probs: u32,
    pub(crate) router_calibration_n_probs: u32,
    pub(crate) default_repeat_penalty: f64,
}

impl Default for LlmRuntimeConfig {
    fn default() -> Self {
        Self {
            version: 1,
            http_timeout_s: 120,
            request_timeout_s: 120,
            final_answer_timeout_s: 60,
            tool_loop_timeout_s: 120,
            model_probe_timeout_s: 120,
            max_response_tokens_cap: 16384,
            tool_loop_max_tokens_cap: 16384,
            model_probe_logprobs_n_probs: 8,
            router_calibration_n_probs: 64,
            default_repeat_penalty: 1.0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ChatRequestOptions {
    pub(crate) temperature: Option<f64>,
    pub(crate) top_p: Option<f64>,
    pub(crate) stream: Option<bool>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) n_probs: Option<u32>,
    pub(crate) repeat_penalty: Option<Option<f64>>,
    pub(crate) reasoning_format: Option<Option<String>>,
    pub(crate) grammar: Option<String>,
    pub(crate) tools: Option<Vec<ToolDefinition>>,
}

impl ChatRequestOptions {
    pub(crate) fn deterministic(max_tokens: u32) -> Self {
        Self {
            temperature: Some(0.0),
            top_p: Some(1.0),
            max_tokens: Some(max_tokens),
            repeat_penalty: Some(Some(runtime_llm_config().default_repeat_penalty)),
            reasoning_format: Some(Some("none".to_string())),
            ..Self::default()
        }
    }
}

pub(crate) fn runtime_llm_config() -> &'static LlmRuntimeConfig {
    LLM_RUNTIME_CONFIG.get_or_init(LlmRuntimeConfig::default)
}

pub(crate) fn set_runtime_llm_config(config: LlmRuntimeConfig) {
    let _ = LLM_RUNTIME_CONFIG.set(config);
}

pub(crate) fn runtime_config_path(config_root: &Path) -> PathBuf {
    config_root.join("runtime.toml")
}

pub(crate) fn load_or_create_runtime_llm_config(config_root: &Path) -> Result<LlmRuntimeConfig> {
    let path = runtime_config_path(config_root);
    if path.exists() {
        let bytes = std::fs::read(&path)
            .with_context(|| format!("Failed to read runtime config at {}", path.display()))?;
        let s = String::from_utf8(bytes).context("runtime config is not valid UTF-8")?;
        return toml::from_str(&s)
            .with_context(|| format!("Failed to parse runtime config at {}", path.display()));
    }

    let config = LlmRuntimeConfig::default();
    let s = toml::to_string_pretty(&config).context("Failed to serialize runtime config")?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    std::fs::write(&path, s.as_bytes())
        .with_context(|| format!("Failed to write runtime config at {}", path.display()))?;
    Ok(config)
}

pub(crate) fn chat_request_from_profile(
    profile: &Profile,
    messages: Vec<ChatMessage>,
    options: ChatRequestOptions,
) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: profile.model.clone(),
        messages,
        temperature: options.temperature.unwrap_or(profile.temperature),
        top_p: options.top_p.unwrap_or(profile.top_p),
        stream: options.stream.unwrap_or(false),
        max_tokens: options.max_tokens.unwrap_or(profile.max_tokens).max(1),
        n_probs: options.n_probs,
        repeat_penalty: options
            .repeat_penalty
            .unwrap_or(Some(profile.repeat_penalty)),
        reasoning_format: options
            .reasoning_format
            .unwrap_or(Some(profile.reasoning_format.clone())),
        grammar: options.grammar,
        tools: options.tools,
    }
}

pub(crate) fn chat_request_system_user(
    profile: &Profile,
    system: &str,
    user: &str,
    options: ChatRequestOptions,
) -> ChatCompletionRequest {
    chat_request_from_profile(
        profile,
        vec![
            ChatMessage::simple("system", system),
            ChatMessage::simple("user", user),
        ],
        options,
    )
}

pub(crate) fn ad_hoc_profile(model: &str, name: &str) -> Profile {
    let cfg = runtime_llm_config();
    Profile {
        version: 1,
        name: name.to_string(),
        base_url: saved_base_url().to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: cfg.default_repeat_penalty,
        reasoning_format: "none".to_string(),
        max_tokens: 8192,
        timeout_s: cfg.request_timeout_s,
        system_prompt: String::new(),
    }
}
