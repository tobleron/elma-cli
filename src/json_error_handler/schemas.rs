//! @efficiency-role: domain-logic
//!
//! JSON Error Handler - Schema Definitions Module
//!
//! Schema validation, field types, validators, and grounding logic:
//! - FieldType, JsonSchema, FieldValidator traits and implementations
//! - Schema construction functions for all intel unit types
//! - Schema validation for critic and outcome verdicts
//! - Deterministic fix functions
//! - Content grounding and claim verification
//! - SchemaValidationError and GroundingError types

use crate::*;
use anyhow::bail;
use std::any::TypeId;
use std::collections::HashMap;

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

// ============================================================================
// Schema Complexity Validation (Task 378)
// ============================================================================

const MAX_NESTING_DEPTH: usize = 2;
const MAX_REQUIRED_FIELDS: usize = 3;
const MAX_TOTAL_FIELDS: usize = 5;

fn max_nesting_depth(schema: &JsonSchema) -> usize {
    let mut max_depth = 1;
    for (_, ft) in &schema.field_types {
        match ft {
            FieldType::Object => {
                // Object fields count as nesting level 2 (the object itself)
                max_depth = max_depth.max(2);
            }
            _ => {}
        }
    }
    max_depth
}

fn has_object_arrays(schema: &JsonSchema) -> bool {
    for (_, ft) in &schema.field_types {
        match ft {
            FieldType::StringArray => {
                // StringArray is fine (only strings). Object arrays would require
                // a different FieldType variant that we don't currently have.
                // We check for any array type that might hold objects.
            }
            _ => {}
        }
    }
    // Current FieldType only supports StringArray which is always a 1D string array.
    // No object arrays are possible with the current type system.
    false
}

pub(crate) fn validate_schema_complexity(schema: &JsonSchema, type_name: &str) -> Result<()> {
    if schema.required_fields.len() > MAX_REQUIRED_FIELDS {
        bail!(
            "Schema for '{}' has {} required fields (max {})",
            type_name,
            schema.required_fields.len(),
            MAX_REQUIRED_FIELDS
        );
    }
    if schema.field_types.len() > MAX_TOTAL_FIELDS {
        bail!(
            "Schema for '{}' has {} total fields (max {})",
            type_name,
            schema.field_types.len(),
            MAX_TOTAL_FIELDS
        );
    }
    let depth = max_nesting_depth(schema);
    if depth > MAX_NESTING_DEPTH {
        bail!(
            "Schema for '{}' has nesting depth {} (max {})",
            type_name,
            depth,
            MAX_NESTING_DEPTH
        );
    }
    if has_object_arrays(schema) {
        bail!("Schema for '{}' contains arrays of objects (banned)", type_name);
    }
    Ok(())
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_critic_schema_within_limits() {
        let schema = critic_like_schema(&["ok", "retry"]);
        assert!(validate_schema_complexity(&schema, "CriticVerdict").is_ok());
    }

    #[test]
    fn test_formula_schema_exceeds_required_fields() {
        let schema = formula_schema();
        // FormulaSelection has 4 required fields (primary, alternatives, reason, memory_id)
        let result = validate_schema_complexity(&schema, "FormulaSelection");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("required fields"));
    }

    #[test]
    fn test_scope_schema_exceeds_required_fields() {
        let schema = scope_schema();
        let result = validate_schema_complexity(&schema, "ScopePlan");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("7 required fields"));
        assert!(err.contains("ScopePlan"));
    }

    #[test]
    fn test_complexity_schema_exceeds_total_fields() {
        let schema = complexity_schema();
        let result = validate_schema_complexity(&schema, "ComplexityAssessment");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("7 total fields"));
    }

    #[test]
    fn test_workflow_schema_exceeds_required_and_nesting() {
        let schema = workflow_schema();
        let result = validate_schema_complexity(&schema, "WorkflowPlannerOutput");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        // workflow_schema has 9 required fields
        assert!(err.contains("required fields"));
    }

    #[test]
    fn test_validate_accepts_simple_schema() {
        let mut schema = schema(vec!["choice", "reason"]);
        schema.field_types.insert("choice", FieldType::Choice(&["a", "b"]));
        schema.field_types.insert("reason", FieldType::String);
        assert!(validate_schema_complexity(&schema, "Simple").is_ok());
    }

    #[test]
    fn test_validate_rejects_too_many_required() {
        let schema = schema(vec!["a", "b", "c", "d"]);
        let result = validate_schema_complexity(&schema, "TooMany");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("4 required fields"));
    }

    #[test]
    fn test_validate_rejects_too_many_total() {
        let mut schema = schema(vec!["a"]);
        for i in 0..6 {
            schema.field_types.insert(
                Box::leak(format!("field_{}", i).into_boxed_str()),
                FieldType::String,
            );
        }
        let result = validate_schema_complexity(&schema, "TooManyTotal");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("6 total fields"));
    }
}

// Re-export verdict validation types and functions
pub(crate) use super::schemas_verdicts::{
    deterministic_fix_critic_verdict, deterministic_fix_outcome_verdict, ground_critic_reason,
    validate_critic_verdict, validate_outcome_verdict, GroundingError, SchemaValidationError,
};
