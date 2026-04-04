//! @efficiency-role: service-orchestrator
//!
//! JSON Error Handler Module
//!
//! Unified error handling for all JSON parsing operations:
//! - Circuit breaker to prevent cascade failures
//! - Safe default values for all component outputs
//! - User-facing error messages (never raw errors)
//! - Fallback usage metrics
//! - Content grounding (Phase 2)
//! - Schema validation (Phase 3) - Manual implementation

use crate::*;
use std::any::TypeId;
use std::collections::HashMap;
use std::sync::OnceLock;

// ============================================================================
// Circuit Breaker
// ============================================================================

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CircuitState {
    Closed,   // Normal operation
    Open,     // Degraded mode
    HalfOpen, // Testing recovery
}

/// JSON Error Handler with circuit breaker and fallbacks
pub(crate) struct JsonErrorHandler {
    state: CircuitState,
    consecutive_failures: u32,
    last_failure_time: Option<u64>,
    failure_threshold: u32,
    cooldown_seconds: u64,
    half_open_successes: u32,
    half_open_threshold: u32,
}

impl JsonErrorHandler {
    /// Create new error handler with default settings
    pub(crate) fn new() -> Self {
        Self {
            state: CircuitState::Closed,
            consecutive_failures: 0,
            last_failure_time: None,
            failure_threshold: 5,
            cooldown_seconds: 60,
            half_open_successes: 0,
            half_open_threshold: 3,
        }
    }

    /// Record a JSON parsing failure
    pub(crate) fn record_failure(&mut self, args: &Args, component: &str) {
        self.consecutive_failures += 1;
        self.last_failure_time = Some(now_unix_s().unwrap_or(0));

        trace(
            args,
            &format!(
                "json_failure component={} total_failures={}",
                component, self.consecutive_failures
            ),
        );

        match self.state {
            CircuitState::Closed => {
                if self.consecutive_failures >= self.failure_threshold {
                    self.state = CircuitState::Open;
                    trace(args, "json_circuit_breaker_opened");
                    self.enter_degraded_mode(args);
                }
            }
            CircuitState::Open => {
                trace(args, "json_circuit_breaker_still_open");
            }
            CircuitState::HalfOpen => {
                self.state = CircuitState::Open;
                self.half_open_successes = 0;
                trace(args, "json_circuit_breaker_recovery_failed");
            }
        }
    }

    /// Record a JSON parsing success
    pub(crate) fn record_success(&mut self, args: &Args) {
        match self.state {
            CircuitState::Closed => {
                self.consecutive_failures = 0;
            }
            CircuitState::Open => {
                if let Some(last_failure) = self.last_failure_time {
                    let now = now_unix_s().unwrap_or(0);
                    if now - last_failure >= self.cooldown_seconds {
                        self.state = CircuitState::HalfOpen;
                        self.half_open_successes = 1;
                        trace(args, "json_circuit_breaker_attempting_recovery");
                    }
                }
            }
            CircuitState::HalfOpen => {
                self.half_open_successes += 1;
                if self.half_open_successes >= self.half_open_threshold {
                    self.state = CircuitState::Closed;
                    self.consecutive_failures = 0;
                    self.half_open_successes = 0;
                    trace(args, "json_circuit_breaker_closed");
                    self.exit_degraded_mode(args);
                }
            }
        }
    }

    /// Check if in degraded mode
    pub(crate) fn is_degraded(&self) -> bool {
        self.state == CircuitState::Open
    }

    /// Get current state
    pub(crate) fn state(&self) -> CircuitState {
        self.state
    }

    fn enter_degraded_mode(&self, args: &Args) {
        trace(
            args,
            "degraded_mode_entered non_essential_features=disabled",
        );
    }

    fn exit_degraded_mode(&self, args: &Args) {
        trace(args, "degraded_mode_exited all_features_enabled");
    }
}

impl Default for JsonErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Global Error Handler Instance
// ============================================================================

static ERROR_HANDLER: OnceLock<Mutex<JsonErrorHandler>> = OnceLock::new();

/// Get global error handler instance
pub(crate) fn get_error_handler() -> &'static Mutex<JsonErrorHandler> {
    ERROR_HANDLER.get_or_init(|| Mutex::new(JsonErrorHandler::new()))
}

/// Record JSON failure globally
pub(crate) fn record_json_failure(args: &Args, component: &str) {
    let mut handler = get_error_handler().lock().unwrap();
    handler.record_failure(args, component);
}

/// Record JSON success globally
pub(crate) fn record_json_success(args: &Args) {
    let mut handler = get_error_handler().lock().unwrap();
    handler.record_success(args);
}

/// Check if system is in degraded mode
pub(crate) fn is_degraded_mode() -> bool {
    get_error_handler().lock().unwrap().is_degraded()
}

// ============================================================================
// Safe Default Values
// ============================================================================

/// Get safe default critic verdict
pub(crate) fn default_critic_verdict() -> CriticVerdict {
    CriticVerdict {
        status: "ok".to_string(),
        reason: "Critic output unavailable, assuming step is valid".to_string(),
        program: None,
    }
}

/// Get safe default outcome verdict based on exit code
pub(crate) fn default_outcome_verdict(exit_code: i32) -> OutcomeVerificationVerdict {
    if exit_code == 0 {
        OutcomeVerificationVerdict {
            status: "ok".to_string(),
            reason: "Step completed successfully (exit code 0)".to_string(),
        }
    } else {
        OutcomeVerificationVerdict {
            status: "retry".to_string(),
            reason: format!("Step failed with exit code {}", exit_code),
        }
    }
}

/// Get safe default sufficiency verdict
pub(crate) fn default_sufficiency_verdict() -> ExecutionSufficiencyVerdict {
    ExecutionSufficiencyVerdict {
        status: "ok".to_string(),
        reason: "Sufficiency verification unavailable, assuming objective is satisfied".to_string(),
        program: None,
    }
}

/// Get safe default formula selection based on route
pub(crate) fn default_formula_selection(route: &str) -> FormulaSelection {
    let (primary, alternatives) = match route {
        "CHAT" => ("reply_only", vec!["capability_reply"]),
        "SHELL" => ("execute_reply", vec!["inspect_reply"]),
        "PLAN" => ("plan_reply", vec!["execute_reply"]),
        "MASTERPLAN" => ("masterplan_reply", vec!["plan_reply"]),
        "DECIDE" => ("inspect_decide_reply", vec!["decide_reply"]),
        _ => ("reply_only", vec![]),
    };

    FormulaSelection {
        primary: primary.to_string(),
        alternatives: alternatives.iter().map(|s| s.to_string()).collect(),
        reason: "Default formula selection due to parsing failure".to_string(),
        memory_id: String::new(),
    }
}

/// Get safe default scope
pub(crate) fn default_scope(objective: &str) -> ScopePlan {
    ScopePlan {
        objective: objective.to_string(),
        focus_paths: vec![],
        include_globs: vec![],
        exclude_globs: vec![],
        query_terms: vec![],
        expected_artifacts: vec![],
        reason: "Default scope due to parsing failure".to_string(),
    }
}

/// Get safe default program for fallback
pub(crate) fn default_fallback_program(line: &str, route: &str) -> Program {
    match route {
        "CHAT" => Program {
            objective: line.to_string(),
            steps: vec![Step::Reply {
                id: "fallback".to_string(),
                instructions: format!("Respond to the user's message: {}", line),
                common: StepCommon {
                    purpose: "direct chat response fallback".to_string(),
                    depends_on: vec![],
                    success_condition: "response sent".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            }],
        },
        _ => Program {
            objective: line.to_string(),
            steps: vec![Step::Reply {
                id: "fallback".to_string(),
                instructions: "Elma encountered an issue planning this task. Please try breaking it into smaller steps or rephrasing your request.".to_string(),
                common: StepCommon {
                    purpose: "fallback clarification".to_string(),
                    depends_on: vec![],
                    success_condition: "user receives clarification request".to_string(),
                    parent_id: None,
                    depth: None,
                    unit_type: None,
                },
            }],
        },
    }
}

// ============================================================================
// User-Facing Error Messages
// ============================================================================

/// Get user-facing error message (NEVER show raw JSON errors)
pub(crate) fn user_facing_json_error_message() -> &'static str {
    "I encountered an issue processing your request. This has been logged for improvement. Could you try rephrasing or breaking this into smaller steps?"
}

// ============================================================================
// Metrics & Logging
// ============================================================================

/// Log fallback usage for metrics
pub(crate) fn log_fallback_usage(args: &Args, component: &str, reason: &str, fallback_type: &str) {
    trace(
        args,
        &format!(
            "json_fallback_used component={} reason={} type={}",
            component, reason, fallback_type
        ),
    );
}

// ============================================================================
// Content Grounding (Phase 2)
// ============================================================================

// ... (existing grounding code)

// ============================================================================
// Schema Validation (Phase 3) - Manual Implementation
// ============================================================================

pub(crate) enum FieldType {
    String,
    Number,
    Boolean,
    StringArray,
    Object,
    Choice(&'static [&'static str]),
}

impl FieldType {
    pub(crate) fn matches(&self, value: &serde_json::Value) -> bool {
        match self {
            FieldType::String => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::StringArray => value
                .as_array()
                .map(|items| items.iter().all(|item| item.is_string()))
                .unwrap_or(false),
            FieldType::Object => value.is_object(),
            FieldType::Choice(allowed) => value
                .as_str()
                .map(|s| allowed.iter().any(|choice| choice.eq_ignore_ascii_case(s)))
                .unwrap_or(false),
        }
    }

    pub(crate) fn describe(&self) -> String {
        match self {
            FieldType::String => "string".to_string(),
            FieldType::Number => "number".to_string(),
            FieldType::Boolean => "boolean".to_string(),
            FieldType::StringArray => "array of strings".to_string(),
            FieldType::Object => "object".to_string(),
            FieldType::Choice(allowed) => format!("one of {}", allowed.join(", ")),
        }
    }
}

pub(crate) trait FieldValidator: Send + Sync {
    fn validate(&self, value: &serde_json::Value) -> Option<String>;
}

pub(crate) struct JsonSchema {
    pub(crate) required_fields: Vec<&'static str>,
    pub(crate) field_types: HashMap<&'static str, FieldType>,
    pub(crate) validators: Vec<Box<dyn FieldValidator + Send + Sync>>,
}

pub(crate) struct EntropyValidator {
    field: &'static str,
}

impl EntropyValidator {
    pub(crate) fn new(field: &'static str) -> Self {
        Self { field }
    }
}

impl FieldValidator for EntropyValidator {
    fn validate(&self, value: &serde_json::Value) -> Option<String> {
        let entropy = value.get(self.field)?.as_f64()?;
        if !(0.0..=1.0).contains(&entropy) {
            return Some(format!(
                "Field '{}' must be between 0.0 and 1.0",
                self.field
            ));
        }
        None
    }
}

pub(crate) struct RequiredChoiceValidator {
    field: &'static str,
    allowed: &'static [&'static str],
}

impl RequiredChoiceValidator {
    pub(crate) fn new(field: &'static str, allowed: &'static [&'static str]) -> Self {
        Self { field, allowed }
    }
}

impl FieldValidator for RequiredChoiceValidator {
    fn validate(&self, value: &serde_json::Value) -> Option<String> {
        let Some(actual) = value.get(self.field).and_then(|v| v.as_str()) else {
            return None;
        };
        if self
            .allowed
            .iter()
            .any(|choice| choice.eq_ignore_ascii_case(actual))
        {
            None
        } else {
            Some(format!(
                "Field '{}' must be one of {}",
                self.field,
                self.allowed.join(", ")
            ))
        }
    }
}

pub(crate) struct ReasonLengthValidator {
    field: &'static str,
    min: usize,
    max: usize,
}

impl ReasonLengthValidator {
    pub(crate) fn new(field: &'static str, min: usize, max: usize) -> Self {
        Self { field, min, max }
    }
}

impl FieldValidator for ReasonLengthValidator {
    fn validate(&self, value: &serde_json::Value) -> Option<String> {
        let len = value.get(self.field)?.as_str()?.trim().len();
        if len < self.min || len > self.max {
            return Some(format!(
                "Field '{}' length must be between {} and {} characters",
                self.field, self.min, self.max
            ));
        }
        None
    }
}

struct NestedScopeValidator;

impl FieldValidator for NestedScopeValidator {
    fn validate(&self, value: &serde_json::Value) -> Option<String> {
        let scope = value.get("scope")?;
        let Some(scope_obj) = scope.as_object() else {
            return Some("Field 'scope' must be an object".to_string());
        };

        let required = [
            ("objective", FieldType::String),
            ("focus_paths", FieldType::StringArray),
            ("include_globs", FieldType::StringArray),
            ("exclude_globs", FieldType::StringArray),
            ("query_terms", FieldType::StringArray),
            ("expected_artifacts", FieldType::StringArray),
            ("reason", FieldType::String),
        ];

        for (field, expected) in required {
            let Some(v) = scope_obj.get(field) else {
                return Some(format!("Field 'scope.{}' is required", field));
            };
            if !expected.matches(v) {
                return Some(format!(
                    "Field 'scope.{}' must be {}",
                    field,
                    expected.describe()
                ));
            }
        }

        None
    }
}

fn schema(required_fields: Vec<&'static str>) -> JsonSchema {
    JsonSchema {
        required_fields,
        field_types: HashMap::new(),
        validators: Vec::new(),
    }
}

fn classification_schema(choices: &'static [&'static str]) -> JsonSchema {
    let mut schema = schema(vec!["choice", "label", "reason", "entropy"]);
    schema
        .field_types
        .insert("choice", FieldType::Choice(choices));
    schema
        .field_types
        .insert("label", FieldType::Choice(choices));
    schema.field_types.insert("reason", FieldType::String);
    schema.field_types.insert("entropy", FieldType::Number);
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new("choice", choices)));
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new("label", choices)));
    schema
        .validators
        .push(Box::new(ReasonLengthValidator::new("reason", 1, 200)));
    schema
        .validators
        .push(Box::new(EntropyValidator::new("entropy")));
    schema
}

fn critic_like_schema(choices: &'static [&'static str]) -> JsonSchema {
    let mut schema = schema(vec!["status", "reason"]);
    schema
        .field_types
        .insert("status", FieldType::Choice(choices));
    schema.field_types.insert("reason", FieldType::String);
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new("status", choices)));
    schema
        .validators
        .push(Box::new(ReasonLengthValidator::new("reason", 1, 200)));
    schema
}

fn scope_schema() -> JsonSchema {
    let mut schema = schema(vec![
        "objective",
        "focus_paths",
        "include_globs",
        "exclude_globs",
        "query_terms",
        "expected_artifacts",
        "reason",
    ]);
    schema.field_types.insert("objective", FieldType::String);
    schema
        .field_types
        .insert("focus_paths", FieldType::StringArray);
    schema
        .field_types
        .insert("include_globs", FieldType::StringArray);
    schema
        .field_types
        .insert("exclude_globs", FieldType::StringArray);
    schema
        .field_types
        .insert("query_terms", FieldType::StringArray);
    schema
        .field_types
        .insert("expected_artifacts", FieldType::StringArray);
    schema.field_types.insert("reason", FieldType::String);
    schema
        .validators
        .push(Box::new(ReasonLengthValidator::new("reason", 1, 200)));
    schema
}

fn formula_schema() -> JsonSchema {
    let choices = &[
        "reply_only",
        "capability_reply",
        "inspect_reply",
        "inspect_summarize_reply",
        "inspect_decide_reply",
        "inspect_edit_verify_reply",
        "execute_reply",
        "plan_reply",
        "masterplan_reply",
    ];
    let mut schema = schema(vec!["primary", "alternatives", "reason", "memory_id"]);
    schema
        .field_types
        .insert("primary", FieldType::Choice(choices));
    schema
        .field_types
        .insert("alternatives", FieldType::StringArray);
    schema.field_types.insert("reason", FieldType::String);
    schema.field_types.insert("memory_id", FieldType::String);
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new("primary", choices)));
    schema
        .validators
        .push(Box::new(ReasonLengthValidator::new("reason", 1, 200)));
    schema
}

fn workflow_schema() -> JsonSchema {
    let complexity_choices = &["DIRECT", "INVESTIGATE", "MULTISTEP", "OPEN_ENDED"];
    let risk_choices = &["LOW", "MEDIUM", "HIGH"];
    let formula_choices = &[
        "reply_only",
        "capability_reply",
        "inspect_reply",
        "inspect_summarize_reply",
        "inspect_decide_reply",
        "inspect_edit_verify_reply",
        "execute_reply",
        "plan_reply",
        "masterplan_reply",
    ];

    let mut schema = schema(vec![
        "objective",
        "complexity",
        "risk",
        "needs_evidence",
        "scope",
        "preferred_formula",
        "alternatives",
        "memory_id",
        "reason",
    ]);
    schema.field_types.insert("objective", FieldType::String);
    schema
        .field_types
        .insert("complexity", FieldType::Choice(complexity_choices));
    schema
        .field_types
        .insert("risk", FieldType::Choice(risk_choices));
    schema
        .field_types
        .insert("needs_evidence", FieldType::Boolean);
    schema.field_types.insert("scope", FieldType::Object);
    schema
        .field_types
        .insert("preferred_formula", FieldType::Choice(formula_choices));
    schema
        .field_types
        .insert("alternatives", FieldType::StringArray);
    schema.field_types.insert("memory_id", FieldType::String);
    schema.field_types.insert("reason", FieldType::String);
    schema.validators.push(Box::new(NestedScopeValidator));
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new(
            "complexity",
            complexity_choices,
        )));
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new("risk", risk_choices)));
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new(
            "preferred_formula",
            formula_choices,
        )));
    schema
        .validators
        .push(Box::new(ReasonLengthValidator::new("reason", 1, 200)));
    schema
}

fn complexity_schema() -> JsonSchema {
    let complexity_choices = &["DIRECT", "INVESTIGATE", "MULTISTEP", "OPEN_ENDED"];
    let risk_choices = &["LOW", "MEDIUM", "HIGH"];
    let mut schema = schema(vec!["complexity", "risk"]);
    schema
        .field_types
        .insert("complexity", FieldType::Choice(complexity_choices));
    schema
        .field_types
        .insert("risk", FieldType::Choice(risk_choices));
    schema
        .field_types
        .insert("needs_evidence", FieldType::Boolean);
    schema.field_types.insert("needs_tools", FieldType::Boolean);
    schema
        .field_types
        .insert("needs_decision", FieldType::Boolean);
    schema.field_types.insert("needs_plan", FieldType::Boolean);
    schema
        .field_types
        .insert("suggested_pattern", FieldType::String);
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new(
            "complexity",
            complexity_choices,
        )));
    schema
        .validators
        .push(Box::new(RequiredChoiceValidator::new("risk", risk_choices)));
    schema
}

pub(crate) fn schema_for_type<T: 'static>() -> Option<JsonSchema> {
    let type_id = TypeId::of::<T>();

    if type_id == TypeId::of::<FormulaSelection>() {
        Some(formula_schema())
    } else if type_id == TypeId::of::<ScopePlan>() {
        Some(scope_schema())
    } else if type_id == TypeId::of::<WorkflowPlannerOutput>() {
        Some(workflow_schema())
    } else if type_id == TypeId::of::<ComplexityAssessment>() {
        Some(complexity_schema())
    } else if type_id == TypeId::of::<CriticVerdict>() {
        Some(critic_like_schema(&["ok", "retry"]))
    } else if type_id == TypeId::of::<OutcomeVerificationVerdict>() {
        Some(critic_like_schema(&["ok", "retry"]))
    } else if type_id == TypeId::of::<ExecutionSufficiencyVerdict>() {
        Some(critic_like_schema(&["ok", "retry"]))
    } else if type_id == TypeId::of::<RepairSemanticsVerdict>() {
        Some(critic_like_schema(&["ok", "retry"]))
    } else if type_id == TypeId::of::<ClaimCheckVerdict>() {
        let mut schema = critic_like_schema(&["ok", "revise"]);
        schema
            .field_types
            .insert("unsupported_claims", FieldType::StringArray);
        schema
            .field_types
            .insert("missing_points", FieldType::StringArray);
        schema
            .field_types
            .insert("rewrite_instructions", FieldType::String);
        Some(schema)
    } else if type_id == TypeId::of::<RiskReviewVerdict>() {
        Some(critic_like_schema(&["ok", "caution"]))
    } else {
        None
    }
}

/// Validate critic verdict against schema (manual implementation)
pub(crate) fn validate_critic_verdict(
    args: &Args,
    verdict: &CriticVerdict,
) -> Result<(), SchemaValidationError> {
    let mut errors = Vec::new();

    // Validate status field
    if verdict.status != "ok" && verdict.status != "retry" {
        errors.push(format!(
            "Invalid status '{}': must be 'ok' or 'retry'",
            verdict.status
        ));
    }

    // Validate reason field
    if verdict.reason.trim().is_empty() {
        errors.push("Reason cannot be empty".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        trace(
            args,
            &format!("critic_schema_validation_failed errors={:?}", errors),
        );
        Err(SchemaValidationError::ValidationErrors(errors))
    }
}

/// Validate outcome verdict against schema (manual implementation)
pub(crate) fn validate_outcome_verdict(
    args: &Args,
    verdict: &OutcomeVerificationVerdict,
) -> Result<(), SchemaValidationError> {
    let mut errors = Vec::new();

    // Validate status field
    if verdict.status != "ok" && verdict.status != "retry" {
        errors.push(format!(
            "Invalid status '{}': must be 'ok' or 'retry'",
            verdict.status
        ));
    }

    // Validate reason field
    if verdict.reason.trim().is_empty() {
        errors.push("Reason cannot be empty".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        trace(
            args,
            &format!("outcome_schema_validation_failed errors={:?}", errors),
        );
        Err(SchemaValidationError::ValidationErrors(errors))
    }
}

/// Deterministic fix based on schema errors
pub(crate) fn deterministic_fix_critic_verdict(
    args: &Args,
    verdict: &CriticVerdict,
    errors: &[String],
) -> Option<CriticVerdict> {
    let mut fixed = verdict.clone();
    let mut was_fixed = false;

    for error in errors {
        let error_lower = error.to_lowercase();

        // Fix missing or invalid status
        if error_lower.contains("status") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            } else if error_lower.contains("enum") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            }
        }

        // Fix missing or empty reason
        if error_lower.contains("reason") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            } else if error_lower.contains("minlength") || error_lower.contains("empty") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            }
        }

        // Fix invalid program field
        if error_lower.contains("program") {
            if error_lower.contains("additional") {
                fixed.program = None;
                was_fixed = true;
            }
        }
    }

    if was_fixed {
        trace(args, "deterministic_fix_applied to critic verdict");
        Some(fixed)
    } else {
        trace(args, "deterministic_fix_failed no applicable fixes");
        None
    }
}

/// Deterministic fix based on schema errors
pub(crate) fn deterministic_fix_outcome_verdict(
    args: &Args,
    verdict: &OutcomeVerificationVerdict,
    errors: &[String],
) -> Option<OutcomeVerificationVerdict> {
    let mut fixed = verdict.clone();
    let mut was_fixed = false;

    for error in errors {
        let error_lower = error.to_lowercase();

        // Fix missing or invalid status
        if error_lower.contains("status") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            } else if error_lower.contains("enum") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            }
        }

        // Fix missing or empty reason
        if error_lower.contains("reason") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            } else if error_lower.contains("minlength") || error_lower.contains("empty") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            }
        }
    }

    if was_fixed {
        trace(args, "deterministic_fix_applied to outcome verdict");
        Some(fixed)
    } else {
        trace(args, "deterministic_fix_failed no applicable fixes");
        None
    }
}

/// Schema validation error
#[derive(Debug, Clone)]
pub(crate) enum SchemaValidationError {
    SerializationError(String),
    SchemaParseError(String),
    ValidationErrors(Vec<String>),
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchemaValidationError::SerializationError(e) => {
                write!(f, "Failed to serialize verdict: {}", e)
            }
            SchemaValidationError::SchemaParseError(e) => {
                write!(f, "Failed to parse schema: {}", e)
            }
            SchemaValidationError::ValidationErrors(errors) => {
                write!(f, "Schema validation failed: {}", errors.join("; "))
            }
        }
    }
}

impl std::error::Error for SchemaValidationError {}

/// Check if critic reason is grounded in actual step outputs
///
/// This catches hallucinated critic claims like "output does not match"
/// when the output is actually valid.
pub(crate) fn ground_critic_reason(
    args: &Args,
    critic_reason: &str,
    step_results: &[StepResult],
) -> Result<(), GroundingError> {
    let reason_lower = critic_reason.to_lowercase();

    // Hallucination patterns - claims that need verification against actual outputs
    let hallucination_patterns = [
        "does not match",
        "output does not match",
        "expected.*but got",
        "only shows",
        "missing",
        "incomplete",
        "not actually satisfy",
    ];

    for pattern in &hallucination_patterns {
        if reason_lower.contains(pattern) {
            // Verify claim against actual outputs
            if !verify_claim_against_outputs(critic_reason, step_results) {
                trace(
                    args,
                    &format!("critic_reason_grounding_failed pattern={}", pattern),
                );
                return Err(GroundingError::HallucinatedClaim(critic_reason.to_string()));
            }
        }
    }

    Ok(())
}

/// Verify a critic claim against actual step outputs
fn verify_claim_against_outputs(claim: &str, outputs: &[StepResult]) -> bool {
    let claim_lower = claim.to_lowercase();

    // Check each step output
    for output in outputs {
        // If claim says "doesn't match" but step succeeded with valid output, claim is hallucinated
        if claim_lower.contains("does not match") || claim_lower.contains("output does not match") {
            if let Some(raw_output) = &output.raw_output {
                // Check if output is non-empty and step succeeded
                if !raw_output.trim().is_empty() && output.ok {
                    // Also check exit code
                    if output.exit_code.unwrap_or(0) == 0 {
                        // Output is valid, claim is hallucinated
                        return false;
                    }
                }
            }
        }

        // If claim says "missing" but we have output, verify it's actually missing
        if claim_lower.contains("missing") {
            if let Some(raw_output) = &output.raw_output {
                if !raw_output.trim().is_empty() {
                    // Output exists, claim may be hallucinated
                    // (unless it's about specific content being missing)
                    return true; // Be conservative - allow the claim
                }
            }
        }
    }

    true // Claim appears grounded (conservative - allow when uncertain)
}

/// Grounding error
#[derive(Debug, Clone)]
pub(crate) enum GroundingError {
    HallucinatedClaim(String),
}

impl std::fmt::Display for GroundingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GroundingError::HallucinatedClaim(claim) => {
                write!(f, "Critic reason appears hallucinated: {}", claim)
            }
        }
    }
}

impl std::error::Error for GroundingError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_opens_after_threshold() {
        let args = Args::parse_from(["elma-cli"]);
        let mut handler = JsonErrorHandler::new();

        for i in 1..=5 {
            handler.record_failure(&args, "test");
            if i < 5 {
                assert_eq!(handler.state(), CircuitState::Closed);
            } else {
                assert_eq!(handler.state(), CircuitState::Open);
            }
        }
    }

    #[test]
    fn test_circuit_breaker_resets_on_success() {
        let args = Args::parse_from(["elma-cli"]);
        let mut handler = JsonErrorHandler::new();

        for _ in 0..4 {
            handler.record_failure(&args, "test");
        }
        assert_eq!(handler.consecutive_failures, 4);

        handler.record_success(&args);
        assert_eq!(handler.consecutive_failures, 0);
    }

    #[test]
    fn test_default_critic_verdict_is_safe() {
        let verdict = default_critic_verdict();
        assert_eq!(verdict.status, "ok");
        assert!(verdict.reason.contains("unavailable"));
        assert!(verdict.program.is_none());
    }

    #[test]
    fn test_default_outcome_verdict_uses_exit_code() {
        let ok_verdict = default_outcome_verdict(0);
        assert_eq!(ok_verdict.status, "ok");

        let retry_verdict = default_outcome_verdict(1);
        assert_eq!(retry_verdict.status, "retry");
        assert!(retry_verdict.reason.contains("exit code 1"));
    }

    #[test]
    fn test_user_facing_message_never_shows_raw_error() {
        let message = user_facing_json_error_message();
        assert!(!message.contains("parse_error"));
        assert!(!message.contains("JSON"));
        assert!(!message.contains("error"));
        assert!(message.contains("issue") || message.contains("try"));
    }

    #[test]
    fn test_grounding_catches_hallucination() {
        let args = Args::parse_from(["elma-cli"]);
        let step_outputs = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            purpose: "list files".to_string(),
            depends_on: vec![],
            success_condition: "files listed".to_string(),
            ok: true,
            summary: "Files listed successfully".to_string(),
            command: Some("ls -ltr".to_string()),
            raw_output: Some("file1.txt\nfile2.txt".to_string()),
            exit_code: Some(0),
            output_bytes: Some(20),
            truncated: false,
            timed_out: false,
            artifact_path: None,
            artifact_kind: None,
            outcome_status: None,
            outcome_reason: None,
        }];

        // This claim is hallucinated - output IS valid
        let hallucinated_claim = "output does not match the intended operation";
        assert!(ground_critic_reason(&args, hallucinated_claim, &step_outputs).is_err());
    }

    #[test]
    fn test_grounding_allows_valid_criticism() {
        let args = Args::parse_from(["elma-cli"]);
        let step_outputs = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            purpose: "list files".to_string(),
            depends_on: vec![],
            success_condition: "files listed".to_string(),
            ok: false,
            summary: "Command failed".to_string(),
            command: Some("ls -ltr".to_string()),
            raw_output: Some("command not found".to_string()),
            exit_code: Some(127),
            output_bytes: Some(20),
            truncated: false,
            timed_out: false,
            artifact_path: None,
            artifact_kind: None,
            outcome_status: None,
            outcome_reason: None,
        }];

        // This claim is grounded - output IS invalid
        let grounded_claim = "command failed with exit code 127";
        assert!(ground_critic_reason(&args, grounded_claim, &step_outputs).is_ok());
    }
}
