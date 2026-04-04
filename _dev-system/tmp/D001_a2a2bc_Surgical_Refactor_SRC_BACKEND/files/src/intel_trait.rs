//! @efficiency-role: data-model
//!
//! Intel Unit Trait Module
//!
//! Defines the standard interface for all intel (intelligence/reasoning) units.
//! Each intel unit follows a consistent pattern:
//! - pre_flight: Validate inputs before model call
//! - execute: Call model with dedicated prompt
//! - post_flight: Validate output before use
//! - fallback: Provide safe defaults on failure

use crate::*;

// Intel Context

/// Context passed to all intel units
#[derive(Debug, Clone)]
pub(crate) struct IntelContext {
    /// Original user message
    pub user_message: String,

    /// Route decision with probability distributions
    pub route_decision: RouteDecision,

    /// Workspace facts (file tree, recent files, etc.)
    pub workspace_facts: String,

    /// Workspace brief (project summary, key files)
    pub workspace_brief: String,

    /// Conversation excerpt (last N messages)
    pub conversation_excerpt: Vec<ChatMessage>,

    /// Complexity assessment (may be set by prior unit)
    pub complexity: Option<ComplexityAssessment>,

    /// Shared HTTP client for all intel units (prevents connection pool exhaustion)
    pub client: reqwest::Client,

    /// Optional structured inputs for units that need more than the base context
    pub extras: serde_json::Map<String, serde_json::Value>,
}

impl IntelContext {
    /// Create new context with minimal required fields
    pub fn new(
        user_message: String,
        route_decision: RouteDecision,
        workspace_facts: String,
        workspace_brief: String,
        conversation_excerpt: Vec<ChatMessage>,
        client: reqwest::Client,
    ) -> Self {
        Self {
            user_message,
            route_decision,
            workspace_facts,
            workspace_brief,
            conversation_excerpt,
            complexity: None,
            client,
            extras: serde_json::Map::new(),
        }
    }

    /// Set complexity assessment (for units that depend on it)
    pub fn with_complexity(mut self, complexity: ComplexityAssessment) -> Self {
        self.complexity = Some(complexity);
        self
    }

    /// Attach extra serialized context for richer intel-unit execution.
    pub fn with_extra<T: Serialize>(mut self, key: &str, value: T) -> Result<Self> {
        self.extras
            .insert(key.to_string(), serde_json::to_value(value)?);
        Ok(self)
    }

    /// Read an extra JSON field by key.
    pub fn extra(&self, key: &str) -> Option<&serde_json::Value> {
        self.extras.get(key)
    }
}

// Intel Output

/// Generic output from intel units
#[derive(Debug, Clone)]
pub(crate) struct IntelOutput {
    /// Unit name for tracing/logging
    pub unit_name: String,

    /// Raw output data (JSON value)
    pub data: serde_json::Value,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,

    /// Whether fallback was used instead of model output
    pub fallback_used: bool,

    /// Error message if fallback was used
    pub fallback_reason: Option<String>,
}

impl IntelOutput {
    /// Create successful output from model
    pub fn success(unit_name: &str, data: serde_json::Value, confidence: f64) -> Self {
        Self {
            unit_name: unit_name.to_string(),
            data,
            confidence,
            fallback_used: false,
            fallback_reason: None,
        }
    }

    /// Create fallback output when model fails
    pub fn fallback(unit_name: &str, data: serde_json::Value, reason: &str) -> Self {
        Self {
            unit_name: unit_name.to_string(),
            data,
            confidence: 0.5, // Neutral confidence for fallback
            fallback_used: true,
            fallback_reason: Some(reason.to_string()),
        }
    }

    /// Get a field from the output data
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Get a string field from the output data
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.data.get(key).and_then(|v| v.as_str())
    }

    /// Get a bool field from the output data
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.data.get(key).and_then(|v| v.as_bool())
    }

    /// Get a f64 field from the output data
    pub fn get_f64(&self, key: &str) -> Option<f64> {
        self.data.get(key).and_then(|v| v.as_f64())
    }

    /// Check if output contains standard fields (label or choice)
    pub fn is_standard_format(&self) -> bool {
        self.get("label").is_some() || self.get("choice").is_some()
    }
}

impl Default for IntelOutput {
    fn default() -> Self {
        Self {
            unit_name: "unknown".to_string(),
            data: serde_json::Value::Null,
            confidence: 0.5,
            fallback_used: false,
            fallback_reason: None,
        }
    }
}

// Intel Unit Trait

/// Common interface for all intel units
pub(crate) trait IntelUnit: Send + Sync {
    /// Unit name for tracing/logging
    fn name(&self) -> &'static str;

    /// Profile configuration for this unit
    fn profile(&self) -> &Profile;

    /// Pre-flight validation (context, inputs)
    /// Default implementation always succeeds (units can override).
    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        Ok(())
    }

    /// Execute the intel unit (model call)
    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput>;

    /// Post-flight verification (output validation)
    /// Default implementation always succeeds (units can override).
    fn post_flight(&self, _output: &IntelOutput) -> Result<()> {
        Ok(())
    }

    /// Fallback when execute() or post_flight() fails
    /// Default implementation returns generic fallback (units SHOULD override).
    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::Value::Null,
            &format!("generic fallback: {}", error),
        ))
    }

    /// Execute with automatic fallback handling
    /// Returns Ok(output) in all cases (fallback ensures success).
    async fn execute_with_fallback(&self, context: &IntelContext) -> Result<IntelOutput> {
        if let Err(error) = self.pre_flight(context) {
            trace_verbose(
                true,
                &format!("intel_{}_preflight_failed error={}", self.name(), error),
            );
            return self.fallback(context, &format!("pre-flight: {}", error));
        }
        self.execute_and_verify(context).await
    }

    /// Internal: execute model call and verify post-flight
    async fn execute_and_verify(&self, context: &IntelContext) -> Result<IntelOutput> {
        match self.execute(context).await {
            Ok(output) => self.verify_or_fallback(output, context).await,
            Err(error) => {
                trace_verbose(
                    true,
                    &format!("intel_{}_execute_failed error={}", self.name(), error),
                );
                self.fallback(context, &format!("execution: {}", error))
            }
        }
    }

    /// Internal: run post-flight verification, fallback on failure
    async fn verify_or_fallback(
        &self,
        output: IntelOutput,
        context: &IntelContext,
    ) -> Result<IntelOutput> {
        if let Err(error) = self.post_flight(&output) {
            trace_verbose(
                true,
                &format!("intel_{}_postflight_failed error={}", self.name(), error),
            );
            return self.fallback(context, &format!("post-flight: {}", error));
        }
        Ok(output)
    }
}

// ============================================================================
// Specialized Output Types
// ============================================================================

/// Shared helper: extract two bool fields from IntelOutput
fn extract_bool_fields(output: &IntelOutput, a: &str, b: &str) -> (bool, bool) {
    (
        output.get_bool(a).unwrap_or(false),
        output.get_bool(b).unwrap_or(false),
    )
}

/// Shared helper: build a test ProbabilityDecision
#[cfg(test)]
fn test_prob_decision(choice: &str) -> ProbabilityDecision {
    ProbabilityDecision {
        choice: choice.to_string(),
        source: "test".to_string(),
        distribution: vec![(choice.to_string(), 1.0)],
        margin: 1.0,
        entropy: 0.0,
    }
}

/// Specialized output for complexity assessment
#[derive(Debug, Clone)]
pub(crate) struct ComplexityOutput {
    pub assessment: ComplexityAssessment,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl ComplexityOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let assessment: ComplexityAssessment = serde_json::from_value(output.data.clone())
            .map_err(|e| anyhow::anyhow!("Failed to parse complexity assessment: {}", e))?;
        Ok(Self {
            assessment,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

/// Specialized output for evidence needs assessment
#[derive(Debug, Clone)]
pub(crate) struct EvidenceNeedsOutput {
    pub needs_evidence: bool,
    pub needs_tools: bool,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl EvidenceNeedsOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let (needs_evidence, needs_tools) =
            extract_bool_fields(output, "needs_evidence", "needs_tools");
        Ok(Self {
            needs_evidence,
            needs_tools,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

/// Specialized output for action needs assessment
#[derive(Debug, Clone)]
pub(crate) struct ActionNeedsOutput {
    pub needs_decision: bool,
    pub needs_plan: bool,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl ActionNeedsOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let (needs_decision, needs_plan) =
            extract_bool_fields(output, "needs_decision", "needs_plan");
        Ok(Self {
            needs_decision,
            needs_plan,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

/// Specialized output for pattern suggestion
#[derive(Debug, Clone)]
pub(crate) struct PatternSuggestionOutput {
    pub suggested_pattern: String,
    pub confidence: f64,
    pub fallback_used: bool,
}

impl PatternSuggestionOutput {
    pub fn from_intel_output(output: &IntelOutput) -> Result<Self> {
        let suggested_pattern = output
            .get_str("suggested_pattern")
            .unwrap_or("reply_only")
            .to_string();
        Ok(Self {
            suggested_pattern,
            confidence: output.confidence,
            fallback_used: output.fallback_used,
        })
    }
}

// Helper Functions

/// Trace fallback usage for metrics
pub(crate) fn trace_fallback(unit_name: &str, error: &str) {
    eprintln!("[INTEL_FALLBACK] unit={} error={}", unit_name, error);
}

/// Trace verbose output (only when verbose mode enabled)
fn trace_verbose(verbose: bool, message: &str) {
    if verbose {
        eprintln!("[INTEL_VERBOSE] {}", message);
    }
}

pub(crate) fn intel_chat_url(profile: &Profile) -> Result<Url> {
    Url::parse(&profile.base_url)
        .map_err(|e| anyhow::anyhow!("Invalid base_url '{}': {}", profile.base_url, e))?
        .join("/v1/chat/completions")
        .map_err(|e| anyhow::anyhow!("Failed to build chat URL: {}", e))
}

pub(crate) fn neutral_route_decision() -> RouteDecision {
    let base = ProbabilityDecision {
        choice: String::new(),
        source: "compat_wrapper".to_string(),
        distribution: Vec::new(),
        margin: 0.0,
        entropy: 1.0,
    };
    RouteDecision {
        route: String::new(),
        source: "compat_wrapper".to_string(),
        distribution: Vec::new(),
        margin: 0.0,
        entropy: 1.0,
        speech_act: base.clone(),
        workflow: base.clone(),
        mode: base,
    }
}

pub(crate) fn apply_profile_grammar(
    profile: &Profile,
    req: &mut ChatCompletionRequest,
) -> Result<()> {
    if let Some(config_root) = crate::ui_chat::get_config_root_for_intel() {
        if let Some(grammar_content) =
            crate::json_grammar::get_grammar_for_profile(&profile.name, config_root)?
        {
            let grammar_str = crate::json_grammar::load_grammar(&grammar_content, config_root)?;
            req.grammar = Some(grammar_str);
        }
    }
    Ok(())
}

pub(crate) fn build_intel_request(
    profile: &Profile,
    messages: Vec<ChatMessage>,
) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: profile.model.clone(),
        messages,
        temperature: profile.temperature,
        top_p: profile.top_p,
        stream: false,
        max_tokens: profile.max_tokens,
        n_probs: None,
        repeat_penalty: Some(profile.repeat_penalty),
        reasoning_format: Some(profile.reasoning_format.clone()),
        grammar: None,
    }
}

pub(crate) fn build_intel_system_user_request(
    profile: &Profile,
    user_content: String,
) -> ChatCompletionRequest {
    build_intel_request(
        profile,
        vec![
            ChatMessage {
                role: "system".to_string(),
                content: profile.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ],
    )
}

pub(crate) async fn execute_intel_json_for_profile<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    profile: &Profile,
    mut req: ChatCompletionRequest,
) -> Result<T> {
    apply_profile_grammar(profile, &mut req)?;
    let chat_url = intel_chat_url(profile)?;
    chat_json_with_repair_for_profile_timeout(
        client,
        &chat_url,
        &req,
        &profile.name,
        profile.timeout_s,
    )
    .await
}

fn log_intel_trace(unit_name: &str, profile: &Profile, chat_url: &Url, grammar_injected: bool) {
    append_trace_log_line(&format!(
        "[INTEL_EXECUTE] unit={} model={}",
        unit_name, profile.model
    ));
    append_trace_log_line(&format!(
        "[INTEL_HTTP_START] url={} timeout={}s",
        profile.base_url, profile.timeout_s
    ));
    append_trace_log_line(&format!("[INTEL_HTTP_URL] final_url={}", chat_url));
    if grammar_injected {
        append_trace_log_line(&format!(
            "[INTEL_GRAMMAR] injected grammar for unit={}",
            unit_name
        ));
    }
}

pub(crate) async fn execute_traced_intel_json_for_profile<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    unit_name: &str,
    profile: &Profile,
    mut req: ChatCompletionRequest,
) -> Result<T> {
    let chat_url = intel_chat_url(profile)?;
    let had_grammar = req.grammar.is_some();
    apply_profile_grammar(profile, &mut req)?;
    log_intel_trace(
        unit_name,
        profile,
        &chat_url,
        had_grammar || req.grammar.is_some(),
    );

    let result = chat_json_with_repair_for_profile_timeout(
        client,
        &chat_url,
        &req,
        &profile.name,
        profile.timeout_s,
    )
    .await?;

    append_trace_log_line(&format!(
        "[INTEL_HTTP_DONE] unit={} received response",
        unit_name
    ));
    Ok(result)
}

pub(crate) async fn execute_intel_json_from_user_content<T: DeserializeOwned + 'static>(
    client: &reqwest::Client,
    profile: &Profile,
    user_content: String,
) -> Result<T> {
    let req = build_intel_system_user_request(profile, user_content);
    execute_intel_json_for_profile(client, profile, req).await
}

pub(crate) async fn execute_intel_text_for_profile(
    client: &reqwest::Client,
    profile: &Profile,
    mut req: ChatCompletionRequest,
) -> Result<String> {
    apply_profile_grammar(profile, &mut req)?;
    let chat_url = intel_chat_url(profile)?;
    let resp = chat_once_with_timeout(client, &chat_url, &req, profile.timeout_s).await?;
    Ok(resp
        .choices
        .first()
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_default()
        .trim()
        .to_string())
}

pub(crate) async fn execute_intel_text_from_user_content(
    client: &reqwest::Client,
    profile: &Profile,
    user_content: String,
) -> Result<String> {
    let req = build_intel_system_user_request(profile, user_content);
    execute_intel_text_for_profile(client, profile, req).await
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intel_output_success() {
        let data = serde_json::json!({"complexity": "DIRECT", "risk": "LOW"});
        let output = IntelOutput::success("test_unit", data.clone(), 0.9);
        assert_eq!(output.unit_name, "test_unit");
        assert_eq!(output.confidence, 0.9);
        assert!(!output.fallback_used);
        assert_eq!(output.get_str("complexity"), Some("DIRECT"));
    }

    #[test]
    fn test_intel_output_fallback() {
        let data = serde_json::json!({"complexity": "INVESTIGATE", "risk": "LOW"});
        let output = IntelOutput::fallback("test_unit", data.clone(), "model timeout");
        assert_eq!(output.unit_name, "test_unit");
        assert_eq!(output.confidence, 0.5);
        assert!(output.fallback_used);
        assert_eq!(output.fallback_reason, Some("model timeout".to_string()));
    }

    #[test]
    fn test_intel_output_field_accessors() {
        let data = serde_json::json!({
            "complexity": "DIRECT",
            "risk": "LOW",
            "needs_evidence": false,
            "confidence": 0.95
        });
        let output = IntelOutput::success("test_unit", data, 0.9);

        assert_eq!(output.get_str("complexity"), Some("DIRECT"));
        assert_eq!(output.get_str("risk"), Some("LOW"));
        assert_eq!(output.get_bool("needs_evidence"), Some(false));
        assert_eq!(output.get_f64("confidence"), Some(0.95));
        assert_eq!(output.get_str("nonexistent"), None);
    }

    #[test]
    fn test_intel_context_builder() {
        let pd = |c: &str| test_prob_decision(c);
        let route_decision = RouteDecision {
            route: "CHAT".to_string(),
            source: "test".to_string(),
            distribution: vec![("CHAT".to_string(), 1.0)],
            margin: 1.0,
            entropy: 0.0,
            speech_act: pd("CHAT"),
            workflow: pd("CHAT"),
            mode: pd("INSPECT"),
        };

        let client = reqwest::Client::new();
        let context = IntelContext::new(
            "test message".to_string(),
            route_decision.clone(),
            "workspace facts".to_string(),
            "workspace brief".to_string(),
            vec![],
            client.clone(),
        );

        assert_eq!(context.user_message, "test message");
        assert_eq!(context.route_decision.route, "CHAT");
        assert!(context.complexity.is_none());

        let context_with_complexity = context.with_complexity(ComplexityAssessment {
            complexity: "DIRECT".to_string(),
            risk: "LOW".to_string(),
            ..ComplexityAssessment::default()
        });

        assert!(context_with_complexity.complexity.is_some());
    }
}
