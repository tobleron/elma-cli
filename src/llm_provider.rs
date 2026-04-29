//! @efficiency-role: infra-adapter
//!
//! Native Rust LLM API Client — Provider Abstraction Layer
//!
//! Replaces Python litellm dependency with native Rust implementations
//! for OpenAI, Anthropic, and OpenAI-compatible (llama.cpp) providers.
//!
//! Task 278: Replace litellm With Native Rust LLM API Client

use crate::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LLM provider type identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum LlmProvider {
    OpenAI,
    Anthropic,
    OpenAICompatible, // llama.cpp, vLLM, Ollama, etc.
    Azure,
    Groq,
}

impl std::fmt::Display for LlmProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProvider::OpenAI => write!(f, "openai"),
            LlmProvider::Anthropic => write!(f, "anthropic"),
            LlmProvider::OpenAICompatible => write!(f, "openai_compatible"),
            LlmProvider::Azure => write!(f, "azure"),
            LlmProvider::Groq => write!(f, "groq"),
        }
    }
}

impl LlmProvider {
    /// Detect provider from base_url and optional hints.
    pub(crate) fn detect(base_url: &str, model_hint: Option<&str>) -> Self {
        let url_lower = base_url.to_lowercase();

        if url_lower.contains("anthropic") || url_lower.contains("api.anthropic.com") {
            return LlmProvider::Anthropic;
        }
        if url_lower.contains("azure") || url_lower.contains("openai.azure.com") {
            return LlmProvider::Azure;
        }
        if url_lower.contains("groq") || url_lower.contains("api.groq.com") {
            return LlmProvider::Groq;
        }
        if url_lower.contains("openai") || url_lower.contains("api.openai.com") {
            return LlmProvider::OpenAI;
        }

        // Model-based hints
        if let Some(model) = model_hint {
            let model_lower = model.to_lowercase();
            if model_lower.starts_with("claude") {
                return LlmProvider::Anthropic;
            }
            if model_lower.starts_with("gpt") {
                return LlmProvider::OpenAI;
            }
            if model_lower.starts_with("llama") || model_lower.starts_with("mistral") {
                return LlmProvider::OpenAICompatible;
            }
        }

        // Default: OpenAI-compatible (llama.cpp style)
        LlmProvider::OpenAICompatible
    }

    /// Default API endpoint path for this provider.
    pub(crate) fn default_chat_path(&self) -> &str {
        match self {
            LlmProvider::OpenAI | LlmProvider::OpenAICompatible | LlmProvider::Groq => {
                "/v1/chat/completions"
            }
            LlmProvider::Anthropic => "/v1/messages",
            LlmProvider::Azure => "/openai/deployments/{deployment-id}/chat/completions",
        }
    }

    /// Default base URL if none configured.
    pub(crate) fn default_base_url(&self) -> &str {
        match self {
            LlmProvider::OpenAI => "https://api.openai.com",
            LlmProvider::Anthropic => "https://api.anthropic.com",
            LlmProvider::OpenAICompatible => "http://localhost:8080",
            LlmProvider::Azure => "https://{resource}.openai.azure.com",
            LlmProvider::Groq => "https://api.groq.com",
        }
    }
}

/// Unified chat completion request (provider-agnostic).
#[derive(Debug, Clone)]
pub(crate) struct UnifiedChatRequest {
    pub model: String,
    pub messages: Vec<UnifiedMessage>,
    pub temperature: f64,
    pub top_p: f64,
    pub max_tokens: u32,
    pub stream: bool,
    pub stop: Option<Vec<String>>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub tool_choice: Option<String>,
    pub extra_params: HashMap<String, serde_json::Value>,
}

/// Unified message (provider-agnostic).
#[derive(Debug, Clone)]
pub(crate) struct UnifiedMessage {
    pub role: String,
    pub content: String,
    pub name: Option<String>,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub tool_call_id: Option<String>,
}

impl UnifiedMessage {
    pub fn system(content: &str) -> Self {
        Self {
            role: "system".to_string(),
            content: content.to_string(),
            name: None,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: "user".to_string(),
            content: content.to_string(),
            name: None,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.to_string(),
            name: None,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool(tool_call_id: &str, content: &str) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.to_string(),
            name: None,
            reasoning_content: None,
            tool_calls: None,
            tool_call_id: Some(tool_call_id.to_string()),
        }
    }
}

/// Unified chat completion response.
#[derive(Debug, Clone)]
pub(crate) struct UnifiedChatResponse {
    pub id: Option<String>,
    pub model: Option<String>,
    pub choices: Vec<UnifiedChoice>,
    pub usage: Option<UnifiedUsage>,
    pub raw_response: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnifiedChoice {
    pub message: UnifiedChoiceMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnifiedChoiceMessage {
    pub role: String,
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone)]
pub(crate) struct UnifiedUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

/// Provider trait — each LLM API implements this.
pub(crate) trait LlmProviderClient: Send + Sync {
    /// Convert unified request to provider-specific HTTP request body.
    fn build_request_body(&self, req: &UnifiedChatRequest) -> Result<serde_json::Value>;

    /// Build provider-specific headers.
    fn build_headers(&self, api_key: &str) -> Result<HeaderMap>;

    /// Get the chat completion endpoint path.
    fn chat_endpoint(&self) -> &str;

    /// Parse provider-specific response into unified format.
    fn parse_response(&self, raw: &str) -> Result<UnifiedChatResponse>;

    /// Get provider identifier.
    fn provider_type(&self) -> LlmProvider;
}

/// OpenAI-compatible provider (llama.cpp, vLLM, Ollama, OpenAI).
pub(crate) struct OpenAICompatibleClient {
    base_url: String,
}

impl OpenAICompatibleClient {
    pub(crate) fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

impl LlmProviderClient for OpenAICompatibleClient {
    fn build_request_body(&self, req: &UnifiedChatRequest) -> Result<serde_json::Value> {
        let messages: Vec<serde_json::Value> = req
            .messages
            .iter()
            .map(|m| {
                let mut msg = serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                });
                if let Some(name) = &m.name {
                    msg["name"] = serde_json::Value::String(name.clone());
                }
                if let Some(tool_calls) = &m.tool_calls {
                    msg["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
                }
                if let Some(tool_call_id) = &m.tool_call_id {
                    msg["tool_call_id"] = serde_json::Value::String(tool_call_id.clone());
                }
                if let Some(reasoning_content) = &m.reasoning_content {
                    msg["reasoning_content"] = serde_json::Value::String(reasoning_content.clone());
                }
                msg
            })
            .collect();

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "temperature": req.temperature,
            "top_p": req.top_p,
            "max_tokens": req.max_tokens,
            "stream": req.stream,
        });

        if let Some(stop) = &req.stop {
            body["stop"] = serde_json::to_value(stop).unwrap_or_default();
        }

        if let Some(tools) = &req.tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.function.name,
                            "description": t.function.description,
                            "parameters": t.function.parameters,
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::Value::Array(tool_defs);
        }

        if let Some(tool_choice) = &req.tool_choice {
            body["tool_choice"] = serde_json::Value::String(tool_choice.clone());
        }

        for (key, value) in &req.extra_params {
            body[key] = value.clone();
        }

        Ok(body)
    }

    fn build_headers(&self, api_key: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        if !api_key.is_empty() {
            let auth = format!("Bearer {}", api_key);
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_str(&auth).context("Invalid API key format")?,
            );
        }
        Ok(headers)
    }

    fn chat_endpoint(&self) -> &str {
        "/v1/chat/completions"
    }

    fn parse_response(&self, raw: &str) -> Result<UnifiedChatResponse> {
        let resp: ChatCompletionResponse =
            serde_json::from_str(raw).context("Invalid JSON from OpenAI-compatible provider")?;

        let choices: Vec<UnifiedChoice> = resp
            .choices
            .into_iter()
            .map(|c| {
                let msg = c.message;
                UnifiedChoice {
                    message: UnifiedChoiceMessage {
                        role: msg.role.unwrap_or_else(|| "assistant".to_string()),
                        content: msg.content.unwrap_or_default(),
                        reasoning_content: msg.reasoning_content,
                        tool_calls: msg.tool_calls,
                    },
                    finish_reason: c.finish_reason,
                }
            })
            .collect();

        let usage = resp.usage.map(|u| UnifiedUsage {
            prompt_tokens: u.prompt_tokens.unwrap_or(0),
            completion_tokens: u.completion_tokens.unwrap_or(0),
            total_tokens: u.total_tokens.unwrap_or(0),
        });

        Ok(UnifiedChatResponse {
            id: resp.id,
            model: resp.model,
            choices,
            usage,
            raw_response: Some(raw.to_string()),
        })
    }

    fn provider_type(&self) -> LlmProvider {
        LlmProvider::OpenAICompatible
    }
}

/// Anthropic Claude provider.
pub(crate) struct AnthropicClient {
    base_url: String,
}

impl AnthropicClient {
    pub(crate) fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }
}

impl LlmProviderClient for AnthropicClient {
    fn build_request_body(&self, req: &UnifiedChatRequest) -> Result<serde_json::Value> {
        // Anthropic uses a different message format — system prompt is separate
        let mut system_content = String::new();
        let mut messages: Vec<serde_json::Value> = Vec::new();

        for m in &req.messages {
            if m.role == "system" {
                if !system_content.is_empty() {
                    system_content.push_str("\n\n");
                }
                system_content.push_str(&m.content);
            } else if m.role == "tool" {
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": m.content,
                }));
            } else {
                messages.push(serde_json::json!({
                    "role": m.role,
                    "content": m.content,
                }));
            }
        }

        let mut body = serde_json::json!({
            "model": req.model,
            "messages": messages,
            "temperature": req.temperature,
            "top_p": req.top_p,
            "max_tokens": req.max_tokens,
            "stream": req.stream,
        });

        if !system_content.is_empty() {
            body["system"] = serde_json::Value::String(system_content);
        }

        if let Some(stop) = &req.stop {
            body["stop_sequences"] = serde_json::to_value(stop).unwrap_or_default();
        }

        if let Some(tools) = &req.tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.function.name,
                        "description": t.function.description,
                        "input_schema": t.function.parameters,
                    })
                })
                .collect();
            body["tools"] = serde_json::Value::Array(tool_defs);
        }

        Ok(body)
    }

    fn build_headers(&self, api_key: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(api_key).context("Invalid Anthropic API key")?,
        );
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
        Ok(headers)
    }

    fn chat_endpoint(&self) -> &str {
        "/v1/messages"
    }

    fn parse_response(&self, raw: &str) -> Result<UnifiedChatResponse> {
        // Anthropic response format
        #[derive(Debug, Deserialize)]
        struct AnthropicResponse {
            #[serde(default)]
            id: Option<String>,
            #[serde(default)]
            model: Option<String>,
            #[serde(rename = "type", default)]
            response_type: Option<String>,
            #[serde(default)]
            role: Option<String>,
            #[serde(default)]
            content: Option<Vec<AnthropicContentBlock>>,
            #[serde(default)]
            stop_reason: Option<String>,
            #[serde(default)]
            stop_sequence: Option<String>,
            #[serde(default)]
            usage: Option<AnthropicUsage>,
        }

        #[derive(Debug, Deserialize)]
        struct AnthropicContentBlock {
            #[serde(rename = "type")]
            block_type: String,
            #[serde(default)]
            text: Option<String>,
        }

        #[derive(Debug, Deserialize)]
        struct AnthropicUsage {
            #[serde(default)]
            input_tokens: Option<u64>,
            #[serde(default)]
            output_tokens: Option<u64>,
        }

        let resp: AnthropicResponse =
            serde_json::from_str(raw).context("Invalid JSON from Anthropic API")?;

        let content_text = resp
            .content
            .unwrap_or_default()
            .into_iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join("\n");

        let choices = vec![UnifiedChoice {
            message: UnifiedChoiceMessage {
                role: resp.role.unwrap_or_else(|| "assistant".to_string()),
                content: content_text,
                reasoning_content: None, // Anthropic doesn't expose reasoning separately in this API version
                tool_calls: None,
            },
            finish_reason: resp.stop_reason,
        }];

        let usage = resp.usage.map(|u| UnifiedUsage {
            prompt_tokens: u.input_tokens.unwrap_or(0),
            completion_tokens: u.output_tokens.unwrap_or(0),
            total_tokens: u.input_tokens.unwrap_or(0) + u.output_tokens.unwrap_or(0),
        });

        Ok(UnifiedChatResponse {
            id: resp.id,
            model: resp.model,
            choices,
            usage,
            raw_response: Some(raw.to_string()),
        })
    }

    fn provider_type(&self) -> LlmProvider {
        LlmProvider::Anthropic
    }
}

/// Unified LLM client that dispatches to the appropriate provider.
pub(crate) struct UnifiedLlmClient {
    http_client: reqwest::Client,
    provider: Box<dyn LlmProviderClient>,
    api_key: String,
}

impl UnifiedLlmClient {
    /// Create a new unified client.
    pub(crate) fn new(
        provider: Box<dyn LlmProviderClient>,
        api_key: String,
        http_client: Option<reqwest::Client>,
    ) -> Self {
        Self {
            http_client: http_client.unwrap_or_else(|| reqwest::Client::new()),
            provider,
            api_key,
        }
    }

    /// Send a chat completion request.
    pub(crate) async fn chat(
        &self,
        base_url: &str,
        req: &UnifiedChatRequest,
    ) -> Result<UnifiedChatResponse> {
        let body = self.provider.build_request_body(req)?;
        let headers = self.provider.build_headers(&self.api_key)?;

        let url = format!("{}{}", base_url, self.provider.chat_endpoint());
        let url = Url::parse(&url).with_context(|| format!("Invalid URL: {}", url))?;

        let resp = self
            .http_client
            .post(url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .context("LLM request failed")?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .context("Failed to read LLM response body")?;

        if !status.is_success() {
            anyhow::bail!("LLM API returned HTTP {}: {}", status, text);
        }

        self.provider.parse_response(&text)
    }

    /// Get the provider type.
    pub(crate) fn provider_type(&self) -> LlmProvider {
        self.provider.provider_type()
    }
}

/// Convert ChatCompletionRequest to UnifiedChatRequest.
pub(crate) fn to_unified_request(req: &ChatCompletionRequest) -> UnifiedChatRequest {
    let messages: Vec<UnifiedMessage> = req
        .messages
        .iter()
        .map(|m| UnifiedMessage {
            role: m.role.clone(),
            content: m.content.clone(),
            name: m.name.clone(),
            reasoning_content: m.reasoning_content.clone(),
            tool_calls: m.tool_calls.clone(),
            tool_call_id: m.tool_call_id.clone(),
        })
        .collect();

    let mut extra_params = HashMap::new();
    if let Some(n_probs) = req.n_probs {
        extra_params.insert("n_probs".to_string(), serde_json::json!(n_probs));
    }
    if let Some(repeat_penalty) = req.repeat_penalty {
        extra_params.insert(
            "repeat_penalty".to_string(),
            serde_json::json!(repeat_penalty),
        );
    }
    if let Some(reasoning_format) = &req.reasoning_format {
        extra_params.insert(
            "reasoning_format".to_string(),
            serde_json::json!(reasoning_format),
        );
    }
    if let Some(grammar) = &req.grammar {
        extra_params.insert("grammar".to_string(), serde_json::json!(grammar));
    }

    UnifiedChatRequest {
        model: req.model.clone(),
        messages,
        temperature: req.temperature,
        top_p: req.top_p,
        max_tokens: req.max_tokens,
        stream: req.stream,
        stop: None,
        tools: req.tools.clone(),
        tool_choice: None,
        extra_params,
    }
}

/// Convert UnifiedChatResponse to ChatCompletionResponse.
pub(crate) fn to_chat_response(resp: &UnifiedChatResponse) -> ChatCompletionResponse {
    let choices: Vec<Choice> = resp
        .choices
        .iter()
        .map(|c| Choice {
            message: ChoiceMessage {
                role: Some(c.message.role.clone()),
                content: Some(c.message.content.clone()),
                reasoning_content: c.message.reasoning_content.clone(),
                tool_calls: c.message.tool_calls.clone(),
            },
            finish_reason: c.finish_reason.clone(),
            logprobs: None,
        })
        .collect();

    let usage = resp.usage.as_ref().map(|u| Usage {
        prompt_tokens: Some(u.prompt_tokens),
        completion_tokens: Some(u.completion_tokens),
        total_tokens: Some(u.total_tokens),
    });

    ChatCompletionResponse {
        choices,
        id: resp.id.clone(),
        created: None,
        model: resp.model.clone(),
        system_fingerprint: None,
        usage,
        timings: None,
    }
}

/// Create a provider client based on provider type and base URL.
pub(crate) fn create_provider_client(
    provider: LlmProvider,
    base_url: &str,
) -> Box<dyn LlmProviderClient> {
    match provider {
        LlmProvider::Anthropic => Box::new(AnthropicClient::new(base_url)),
        LlmProvider::OpenAI
        | LlmProvider::OpenAICompatible
        | LlmProvider::Azure
        | LlmProvider::Groq => Box::new(OpenAICompatibleClient::new(base_url)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_detection_openai() {
        assert_eq!(
            LlmProvider::detect("https://api.openai.com", None),
            LlmProvider::OpenAI
        );
    }

    #[test]
    fn test_provider_detection_anthropic() {
        assert_eq!(
            LlmProvider::detect("https://api.anthropic.com", None),
            LlmProvider::Anthropic
        );
    }

    #[test]
    fn test_provider_detection_model_hint() {
        assert_eq!(
            LlmProvider::detect("http://localhost:8080", Some("claude-3-opus")),
            LlmProvider::Anthropic
        );
        assert_eq!(
            LlmProvider::detect("http://localhost:8080", Some("gpt-4")),
            LlmProvider::OpenAI
        );
        assert_eq!(
            LlmProvider::detect("http://localhost:8080", Some("llama-3-70b")),
            LlmProvider::OpenAICompatible
        );
    }

    #[test]
    fn test_provider_detection_default() {
        assert_eq!(
            LlmProvider::detect("http://localhost:8080", None),
            LlmProvider::OpenAICompatible
        );
    }

    #[test]
    fn test_openai_compatible_request_body() {
        let client = OpenAICompatibleClient::new("http://localhost:8080");
        let req = UnifiedChatRequest {
            model: "llama-3-8b".to_string(),
            messages: vec![
                UnifiedMessage::system("You are helpful."),
                UnifiedMessage::user("Hello"),
            ],
            temperature: 0.7,
            top_p: 1.0,
            max_tokens: 1024,
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
            extra_params: HashMap::new(),
        };

        let body = client.build_request_body(&req).unwrap();
        assert_eq!(body["model"], "llama-3-8b");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["temperature"], 0.7);
        assert_eq!(body["max_tokens"], 1024);
    }

    #[test]
    fn test_anthropic_request_body() {
        let client = AnthropicClient::new("https://api.anthropic.com");
        let req = UnifiedChatRequest {
            model: "claude-3-opus".to_string(),
            messages: vec![
                UnifiedMessage::system("You are Claude."),
                UnifiedMessage::user("Hello"),
            ],
            temperature: 0.7,
            top_p: 1.0,
            max_tokens: 1024,
            stream: false,
            stop: None,
            tools: None,
            tool_choice: None,
            extra_params: HashMap::new(),
        };

        let body = client.build_request_body(&req).unwrap();
        assert_eq!(body["model"], "claude-3-opus");
        assert_eq!(body["system"], "You are Claude.");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["max_tokens"], 1024);
    }

    #[test]
    fn test_openai_headers() {
        let client = OpenAICompatibleClient::new("http://localhost:8080");
        let headers = client.build_headers("test-key").unwrap();
        assert_eq!(headers.get(AUTHORIZATION).unwrap(), "Bearer test-key");
    }

    #[test]
    fn test_anthropic_headers() {
        let client = AnthropicClient::new("https://api.anthropic.com");
        let headers = client.build_headers("sk-ant-test").unwrap();
        assert_eq!(headers.get("x-api-key").unwrap(), "sk-ant-test");
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
    }

    #[test]
    fn test_unified_message_helpers() {
        let sys = UnifiedMessage::system("Be helpful");
        assert_eq!(sys.role, "system");
        assert_eq!(sys.content, "Be helpful");

        let user = UnifiedMessage::user("Hi");
        assert_eq!(user.role, "user");

        let assistant = UnifiedMessage::assistant("Hello!");
        assert_eq!(assistant.role, "assistant");

        let tool = UnifiedMessage::tool("call_123", "result");
        assert_eq!(tool.role, "tool");
        assert_eq!(tool.tool_call_id, Some("call_123".to_string()));
    }

    #[test]
    fn test_to_unified_request_conversion() {
        let req = ChatCompletionRequest {
            model: "llama-3-8b".to_string(),
            messages: vec![ChatMessage::simple("user", "Hello")],
            temperature: 0.5,
            top_p: 1.0,
            stream: false,
            max_tokens: 512,
            n_probs: Some(5),
            repeat_penalty: Some(1.1),
            reasoning_format: Some("auto".to_string()),
            grammar: None,
            tools: None,
        };

        let unified = to_unified_request(&req);
        assert_eq!(unified.model, "llama-3-8b");
        assert_eq!(unified.messages.len(), 1);
        assert_eq!(unified.temperature, 0.5);
        assert!(unified.extra_params.contains_key("n_probs"));
        assert!(unified.extra_params.contains_key("repeat_penalty"));
        assert!(unified.extra_params.contains_key("reasoning_format"));
    }

    #[test]
    fn test_provider_display_names() {
        assert_eq!(LlmProvider::OpenAI.to_string(), "openai");
        assert_eq!(LlmProvider::Anthropic.to_string(), "anthropic");
        assert_eq!(
            LlmProvider::OpenAICompatible.to_string(),
            "openai_compatible"
        );
        assert_eq!(LlmProvider::Azure.to_string(), "azure");
        assert_eq!(LlmProvider::Groq.to_string(), "groq");
    }

    #[test]
    fn test_default_chat_paths() {
        assert_eq!(
            LlmProvider::OpenAI.default_chat_path(),
            "/v1/chat/completions"
        );
        assert_eq!(LlmProvider::Anthropic.default_chat_path(), "/v1/messages");
    }

    #[test]
    fn test_default_base_urls() {
        assert_eq!(
            LlmProvider::OpenAI.default_base_url(),
            "https://api.openai.com"
        );
        assert_eq!(
            LlmProvider::Anthropic.default_base_url(),
            "https://api.anthropic.com"
        );
        assert_eq!(
            LlmProvider::OpenAICompatible.default_base_url(),
            "http://localhost:8080"
        );
    }
}
