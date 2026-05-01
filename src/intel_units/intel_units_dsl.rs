//! @efficiency-role: util-pure
//!
//! Intel Unit DSL Output Parsing
//!
//! Converts compact model-produced DSL output into serde_json::Value
//! that can be consumed by the existing typed post-flight and fallback machinery.
//!
//! Each intel DSL family has its own parse function. The generic
//! `parse_intel_dsl_to_value` function handles the single-line key=value
//! pattern used by most classifiers and simple assessments.

use crate::dsl::{
    expect_command, expect_field_line, expect_key_value, expect_quoted_field, expect_terminator,
    extract_block_body, parse_line, require_field, strip_first_line, DslBlockParser, DslError,
    DslErrorCode, DslLine, DslResult, ParseContext,
};
use serde_json::Value;

fn ctx() -> ParseContext {
    ParseContext {
        dsl_variant: "intel",
        line: None,
    }
}

fn ctx_named(name: &'static str) -> ParseContext {
    ParseContext {
        dsl_variant: name,
        line: None,
    }
}

/// Recursively coerce string values in a JSON Value tree.
fn coerce_json_value(v: &Value) -> Value {
    match v {
        Value::String(s) => coerce_value(s),
        Value::Object(map) => {
            let mut coerced = serde_json::Map::new();
            for (k, val) in map {
                coerced.insert(k.clone(), coerce_json_value(val));
            }
            Value::Object(coerced)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(coerce_json_value).collect()),
        other => other.clone(),
    }
}

/// Coerce string values to more specific types where unambiguous.
///
/// - `"true"` / `"True"` / `"TRUE"` → `true`
/// - `"false"` / `"False"` / `"FALSE"` → `false`
/// - Strings that parse as integers → integer Value
/// - Strings that parse as floats → float Value
/// - Everything else → stays as string
fn coerce_value(value: &str) -> Value {
    match value {
        "true" | "True" | "TRUE" => Value::Bool(true),
        "false" | "False" | "FALSE" => Value::Bool(false),
        _ => {
            if let Ok(n) = value.parse::<i64>() {
                return Value::Number(n.into());
            }
            if let Ok(n) = value.parse::<f64>() {
                if let Some(v) = serde_json::Number::from_f64(n) {
                    return Value::Number(v);
                }
            }
            Value::String(value.to_string())
        }
    }
}

/// Parse a single-line intel DSL into a JSON object of key=value pairs.
///
/// Expected format:
///   COMMAND key1=val1 key2="quoted val" key3=bare
///
/// Returns the command token and a JSON object with the parsed fields.
pub(crate) fn parse_intel_dsl_to_value(input: &str) -> DslResult<(String, Value)> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx()));
    }
    let parser = DslBlockParser::new(ctx_named("intel_record"));
    let block = parser.parse_single_line(input)?;
    let mut map = serde_json::Map::new();
    for field in &block.fields {
        map.insert(field.key.clone(), coerce_value(&field.value));
    }
    Ok((block.command, Value::Object(map)))
}

/// Parse a verdict-style intel DSL.
///
/// Expected format:
///   OK reason="text"
///   RETRY reason="text"
///
/// Returns a JSON object: {"status": "ok"/"retry"/"revise", "reason": "..."}
pub(crate) fn parse_verdict_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("verdict")));
    }
    let parser = DslBlockParser::new(ctx_named("verdict"));
    let block = parser.parse_single_line(input)?;
    let status = match block.command.as_str() {
        "OK" => "ok",
        "RETRY" => "retry",
        "REVISE" => "revise",
        other => return Err(DslError::unknown_command(ctx_named("verdict"), other)),
    };
    let reason = block
        .fields
        .iter()
        .find(|f| f.key == "reason")
        .map(|f| f.value.as_str())
        .unwrap_or("");
    Ok(serde_json::json!({
        "status": status,
        "reason": reason,
    }))
}

/// Parse a critic/verdict DSL that may include extra fields beyond status/reason.
///
/// Expected format:
///   OK reason="text" unsupported_claims="..." missing_points="..."
pub(crate) fn parse_critic_verdict_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("critic_verdict")));
    }
    let parser = DslBlockParser::new(ctx_named("critic_verdict"));
    let block = parser.parse_single_line(input)?;
    let status = match block.command.as_str() {
        "OK" => "ok",
        "RETRY" => "retry",
        "CAUTION" => "caution",
        other => {
            return Err(DslError::unknown_command(
                ctx_named("critic_verdict"),
                other,
            ))
        }
    };
    let mut map = serde_json::Map::new();
    map.insert("status".to_string(), Value::String(status.to_string()));
    for field in &block.fields {
        let value = &field.value;
        match field.key.as_str() {
            "reason" => {
                map.insert("reason".to_string(), Value::String(value.clone()));
            }
            "unsupported_claims" => {
                let items: Vec<Value> = value
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                map.insert("unsupported_claims".to_string(), Value::Array(items));
            }
            "missing_points" => {
                let items: Vec<Value> = value
                    .split(',')
                    .filter(|s| !s.trim().is_empty())
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                map.insert("missing_points".to_string(), Value::Array(items));
            }
            _ => {
                map.insert(field.key.clone(), Value::String(value.clone()));
            }
        }
    }
    Ok(Value::Object(map))
}

/// Parse a scope/list block DSL into a JSON object.
///
/// Expected format:
///   SCOPE objective="inspect tool path"
///   F path="src/tool_loop.rs"
///   F path="src/tool_calling.rs"
///   Q text="tool_calls"
///   END
pub(crate) fn parse_scope_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("scope")));
    }
    let parser = DslBlockParser::new(ctx_named("scope"));
    let block = parser.parse_block(input, "END")?;

    let mut map = serde_json::Map::new();
    for field in &block.fields {
        map.insert(field.key.clone(), Value::String(field.value.clone()));
    }

    let mut focus_paths = Vec::new();
    let mut include_globs = Vec::new();
    let mut exclude_globs = Vec::new();
    let mut query_terms = Vec::new();
    let mut expected_artifacts = Vec::new();

    for line in &block.lines {
        if let DslLine::Command { name, fields } = line {
            match name.as_str() {
                "F" | "FP" => {
                    for f in fields {
                        if f.key == "path" {
                            focus_paths.push(Value::String(f.value.clone()));
                        }
                    }
                }
                "IG" => {
                    for f in fields {
                        if f.key == "glob" {
                            include_globs.push(Value::String(f.value.clone()));
                        }
                    }
                }
                "EG" => {
                    for f in fields {
                        if f.key == "glob" {
                            exclude_globs.push(Value::String(f.value.clone()));
                        }
                    }
                }
                "Q" => {
                    for f in fields {
                        if f.key == "text" {
                            query_terms.push(Value::String(f.value.clone()));
                        }
                    }
                }
                "A" => {
                    for f in fields {
                        if f.key == "artifact" {
                            expected_artifacts.push(Value::String(f.value.clone()));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    map.insert("focus_paths".to_string(), Value::Array(focus_paths));
    map.insert("include_globs".to_string(), Value::Array(include_globs));
    map.insert("exclude_globs".to_string(), Value::Array(exclude_globs));
    map.insert("query_terms".to_string(), Value::Array(query_terms));
    map.insert(
        "expected_artifacts".to_string(),
        Value::Array(expected_artifacts),
    );
    map.insert(
        "reason".to_string(),
        map.get("reason")
            .cloned()
            .unwrap_or(Value::String("scope from DSL".to_string())),
    );

    Ok(coerce_json_value(&Value::Object(map)))
}

/// Parse a workflow planner DSL block into a WorkflowPlannerOutput-shaped JSON object.
///
/// Expected format:
///   WORKFLOW objective="..." complexity=DIRECT risk=LOW needs_evidence=true preferred_formula=inspect_reply memory_id="" reason="..."
///   scope_objective="..." scope_reason="..."
///   F path="src/main.rs"
///   IG glob="src/**"
///   EG glob="target/**"
///   Q text="workflow"
///   A artifact="CERTIFICATION_REPORT.md"
///   ALT formula="inspect_reply"
///   END
pub(crate) fn parse_workflow_planner_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("workflow_planner")));
    }
    let parser = DslBlockParser::new(ctx_named("workflow_planner"));
    let block = parser.parse_block(input, "END")?;

    let objective = require_field(&block.fields, "objective", &ctx_named("workflow_planner"))?;
    let complexity = require_field(&block.fields, "complexity", &ctx_named("workflow_planner"))?;
    let risk = require_field(&block.fields, "risk", &ctx_named("workflow_planner"))?;
    let preferred_formula = require_field(
        &block.fields,
        "preferred_formula",
        &ctx_named("workflow_planner"),
    )?;
    let reason = require_field(&block.fields, "reason", &ctx_named("workflow_planner"))?;

    let needs_evidence = block
        .fields
        .iter()
        .find(|f| f.key == "needs_evidence")
        .map(|f| f.value.as_str())
        .unwrap_or("false");
    let needs_evidence = matches!(needs_evidence, "true" | "True" | "TRUE");

    let memory_id = block
        .fields
        .iter()
        .find(|f| f.key == "memory_id")
        .map(|f| f.value.clone())
        .unwrap_or_default();

    let scope_objective = block
        .fields
        .iter()
        .find(|f| f.key == "scope_objective")
        .map(|f| f.value.clone())
        .unwrap_or_else(|| objective.to_string());
    let scope_reason = block
        .fields
        .iter()
        .find(|f| f.key == "scope_reason")
        .map(|f| f.value.clone())
        .unwrap_or_else(|| reason.to_string());

    let mut focus_paths: Vec<Value> = Vec::new();
    let mut include_globs: Vec<Value> = Vec::new();
    let mut exclude_globs: Vec<Value> = Vec::new();
    let mut query_terms: Vec<Value> = Vec::new();
    let mut expected_artifacts: Vec<Value> = Vec::new();
    let mut alternatives: Vec<Value> = Vec::new();

    for line in &block.lines {
        let DslLine::Command { name, fields } = line else {
            continue;
        };
        match name.as_str() {
            "F" | "FP" => {
                for f in fields {
                    if f.key == "path" {
                        focus_paths.push(Value::String(f.value.clone()));
                    }
                }
            }
            "IG" => {
                for f in fields {
                    if f.key == "glob" {
                        include_globs.push(Value::String(f.value.clone()));
                    }
                }
            }
            "EG" => {
                for f in fields {
                    if f.key == "glob" {
                        exclude_globs.push(Value::String(f.value.clone()));
                    }
                }
            }
            "Q" => {
                for f in fields {
                    if f.key == "text" {
                        query_terms.push(Value::String(f.value.clone()));
                    }
                }
            }
            "A" => {
                for f in fields {
                    if f.key == "artifact" {
                        expected_artifacts.push(Value::String(f.value.clone()));
                    }
                }
            }
            "ALT" => {
                for f in fields {
                    if f.key == "formula" {
                        alternatives.push(Value::String(f.value.clone()));
                    }
                }
            }
            _ => {}
        }
    }

    Ok(coerce_json_value(&serde_json::json!({
        "objective": objective,
        "complexity": complexity,
        "risk": risk,
        "needs_evidence": needs_evidence,
        "scope": {
            "objective": scope_objective,
            "focus_paths": focus_paths,
            "include_globs": include_globs,
            "exclude_globs": exclude_globs,
            "query_terms": query_terms,
            "expected_artifacts": expected_artifacts,
            "reason": scope_reason,
        },
        "preferred_formula": preferred_formula,
        "alternatives": alternatives,
        "memory_id": memory_id,
        "reason": reason,
    })))
}

/// Parse a formula selection DSL.
///
/// Expected format:
///   FORMULA primary=inspect_reply alt=execute_reply reason="needs evidence"
pub(crate) fn parse_formula_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("formula")));
    }
    let parser = DslBlockParser::new(ctx_named("formula"));
    let block = parser.parse_single_line(input)?;

    let primary = require_field(&block.fields, "primary", &ctx_named("formula"))?;
    let mut map = serde_json::Map::new();
    map.insert("primary".to_string(), Value::String(primary.to_string()));

    let mut alternatives = Vec::new();
    for field in &block.fields {
        if field.key.starts_with("alt") || field.key == "alternative" {
            alternatives.push(Value::String(field.value.clone()));
        }
    }
    map.insert("alternatives".to_string(), Value::Array(alternatives));

    let reason = block
        .fields
        .iter()
        .find(|f| f.key == "reason")
        .map(|f| Value::String(f.value.clone()))
        .unwrap_or(Value::String("formula from DSL".to_string()));
    map.insert("reason".to_string(), reason);

    let memory_id = block
        .fields
        .iter()
        .find(|f| f.key == "memory_id")
        .map(|f| Value::String(f.value.clone()))
        .unwrap_or(Value::String(String::new()));
    map.insert("memory_id".to_string(), memory_id);

    Ok(coerce_json_value(&Value::Object(map)))
}

/// Parse a selection DSL (list of items with reason).
///
/// Expected format:
///   ITEM value="src/main.rs"
///   ITEM value="src/lib.rs"
///   REASON text="most likely entry points"
///   END
pub(crate) fn parse_selection_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("selection")));
    }
    let parser = DslBlockParser::new(ctx_named("selection"));
    let block = parser.parse_block(input, "END")?;

    let mut items = Vec::new();
    let mut reason = String::from("selection from DSL");

    // Include the header fields as the first item
    for f in &block.fields {
        if f.key == "value" {
            items.push(Value::String(f.value.clone()));
        }
    }

    for line in &block.lines {
        if let DslLine::Command { name, fields } = line {
            match name.as_str() {
                "ITEM" => {
                    for f in fields {
                        if f.key == "value" {
                            items.push(Value::String(f.value.clone()));
                        }
                    }
                }
                "REASON" => {
                    for f in fields {
                        if f.key == "text" {
                            reason = f.value.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(coerce_json_value(&serde_json::json!({
        "items": items,
        "reason": reason,
    })))
}

/// Parse a list-style output DSL into a JSON object with list field.
///
/// Expected format:
///   RESULT key1=val1
///   ITEM value=item1
///   ITEM value=item2
///   END
pub(crate) fn parse_list_dsl(
    input: &str,
    list_command: &str,
    list_field: &str,
) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("list_dsl")));
    }
    let parser = DslBlockParser::new(ctx_named("list_dsl"));
    let block = parser.parse_block(input, "END")?;

    let mut map = serde_json::Map::new();
    for field in &block.fields {
        map.insert(field.key.clone(), Value::String(field.value.clone()));
    }

    let mut list_items = Vec::new();
    // Include header line's matching fields
    if block.command == list_command {
        for f in &block.fields {
            list_items.push(Value::String(f.value.clone()));
        }
    }
    for line in &block.lines {
        if let DslLine::Command { name, fields } = line {
            if name == list_command {
                for f in fields {
                    list_items.push(Value::String(f.value.clone()));
                }
            }
        }
    }
    map.insert(list_field.to_string(), Value::Array(list_items));

    Ok(Value::Object(map))
}

/// Parse a record-style intel DSL (multiple key=value pairs, no block).
/// Returns just the Value map (command token is discarded).
pub(crate) fn parse_record_dsl_to_value(input: &str) -> DslResult<Value> {
    let (_, value) = parse_intel_dsl_to_value(input)?;
    Ok(coerce_json_value(&value))
}

/// Parse a claim block DSL into a claims array.
///
/// Expected format:
///   CLAIM statement="text" evidence_ids="e_001,e_002" status=GROUNDED
///   CLAIM statement="text2" evidence_ids="" status=UNGROUNDED
///   REASON text="summary"
///   END
///
/// Returns:
///   {"claims": [{"statement":"...","evidence_ids":["e_001"],"status":"GROUNDED"},...], "reason": "..."}
pub(crate) fn parse_claim_block_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("claim_block")));
    }
    let parser = DslBlockParser::new(ctx_named("claim_block"));
    let block = parser.parse_block(input, "END")?;

    let mut claims = Vec::new();
    let mut reason = String::from("claim block from DSL");

    for line in &block.lines {
        if let DslLine::Command { name, fields } = line {
            match name.as_str() {
                "CLAIM" => {
                    let mut claim = serde_json::Map::new();
                    for f in fields {
                        match f.key.as_str() {
                            "statement" => {
                                claim.insert(
                                    "statement".to_string(),
                                    Value::String(f.value.clone()),
                                );
                            }
                            "evidence_ids" => {
                                let ids: Vec<Value> = f
                                    .value
                                    .split(',')
                                    .filter(|s| !s.trim().is_empty())
                                    .map(|s| Value::String(s.trim().to_string()))
                                    .collect();
                                claim.insert("evidence_ids".to_string(), Value::Array(ids));
                            }
                            "status" => {
                                claim.insert("status".to_string(), Value::String(f.value.clone()));
                            }
                            _ => {
                                claim.insert(f.key.clone(), Value::String(f.value.clone()));
                            }
                        }
                    }
                    claims.push(Value::Object(claim));
                }
                "REASON" => {
                    for f in fields {
                        if f.key == "text" {
                            reason = f.value.clone();
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(serde_json::json!({
        "claims": claims,
        "reason": reason,
    }))
}

/// Parse an intel DSL output, auto-detecting the format based on content.
///
/// Heuristics:
/// - If starts with OK/RETRY/REVISE: use verdict parser
/// - If contains a block with END: use scope/selection/claim parser
/// - Otherwise: use single-line record parser
pub(crate) fn parse_auto_dsl(input: &str) -> DslResult<Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DslError::empty(ctx()));
    }

    let first_line = trimmed.split('\n').next().unwrap_or("").trim();

    // Detecting verdict DSL: first token is OK, RETRY, or REVISE
    let first_token = first_line.split_whitespace().next().unwrap_or("");
    match first_token {
        "OK" | "RETRY" | "REVISE" => {
            // Check if it's a simple verdict or rich critic verdict
            if trimmed.contains("unsupported_claims") || trimmed.contains("missing_points") {
                return parse_critic_verdict_dsl(trimmed);
            }
            return parse_verdict_dsl(trimmed);
        }
        "CAUTION" => {
            return parse_critic_verdict_dsl(trimmed);
        }
        "SCOPE" => {
            return parse_scope_dsl(trimmed);
        }
        "WORKFLOW" => {
            return parse_workflow_planner_dsl(trimmed);
        }
        "FORMULA" => {
            return parse_formula_dsl(trimmed);
        }
        "OBJECTIVE" => {
            return parse_pyramid_block_dsl(trimmed);
        }
        "NEXT" => {
            return parse_next_action_dsl(trimmed);
        }
        "ITEM" | "SELECTION" => {
            return parse_selection_dsl(trimmed);
        }
        _ => {}
    }

    // Check for block format (has END marker)
    if trimmed.contains("\nEND") || trimmed.ends_with("\nEND") {
        if trimmed.contains("F path=") || trimmed.contains("FP path=") {
            return parse_scope_dsl(trimmed);
        }
        if trimmed.contains("ITEM value=") || trimmed.contains("REASON text=") {
            return parse_selection_dsl(trimmed);
        }
        if trimmed.contains("CLAIM statement=") || first_token == "CLAIM" {
            return parse_claim_block_dsl(trimmed);
        }
        // Generic list parse
        return parse_list_dsl(trimmed, "ITEM", "items");
    }

    // Default: single-line record parser
    let value = parse_record_dsl_to_value(trimmed)?;
    Ok(coerce_json_value(&value))
}

/// Parse a pyramid block DSL (OBJECTIVE + GOAL + TASK lines) into a JSON object.
///
/// Expected format:
///   OBJECTIVE text="..." risk=low|medium|high
///   GOAL text="..." evidence_needed=true|false
///   GOAL text="..." evidence_needed=true|false
///   TASK id=1 text="..." status=pending
///   TASK id=2 text="..." status=ready
///   END
///
/// Returns:
///   {"objective": "...", "risk": "...", "goals": [...], "tasks": [...]}
pub(crate) fn parse_pyramid_block_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("pyramid")));
    }
    let parser = DslBlockParser::new(ctx_named("pyramid"));
    let block = parser.parse_block(input, "END")?;

    if block.command != "OBJECTIVE" {
        return Err(DslError::unknown_command(
            ctx_named("pyramid"),
            &block.command,
        ));
    }

    let objective = require_field(&block.fields, "text", &ctx_named("pyramid"))?;
    let risk = block
        .fields
        .iter()
        .find(|f| f.key == "risk")
        .map(|f| f.value.clone())
        .unwrap_or_else(|| "low".to_string());

    let mut goals = Vec::new();
    let mut tasks = Vec::new();

    for line in &block.lines {
        if let DslLine::Command { name, fields } = line {
            match name.as_str() {
                "GOAL" => {
                    let text = require_field(fields, "text", &ctx_named("pyramid"))?;
                    let evidence_needed = fields
                        .iter()
                        .find(|f| f.key == "evidence_needed")
                        .map(|f| matches!(f.value.as_str(), "true" | "True" | "TRUE"))
                        .unwrap_or(false);
                    goals.push(serde_json::json!({
                        "text": text,
                        "evidence_needed": evidence_needed,
                    }));
                }
                "TASK" => {
                    let id_val = require_field(fields, "id", &ctx_named("pyramid"))?;
                    let text = require_field(fields, "text", &ctx_named("pyramid"))?;
                    let status = fields
                        .iter()
                        .find(|f| f.key == "status")
                        .map(|f| f.value.clone())
                        .unwrap_or_else(|| "pending".to_string());
                    tasks.push(serde_json::json!({
                        "id": id_val,
                        "text": text,
                        "status": status,
                    }));
                }
                _ => {}
            }
        }
    }

    Ok(coerce_json_value(&serde_json::json!({
        "objective": objective,
        "risk": risk,
        "goals": goals,
        "tasks": tasks,
    })))
}

/// Parse a NEXT action selection DSL.
///
/// Expected format:
///   NEXT task_id=1 action=edit reason="..."
///
/// Returns:
///   {"task_id": 1, "action": "edit", "reason": "..."}
pub(crate) fn parse_next_action_dsl(input: &str) -> DslResult<Value> {
    let input = input.trim();
    if input.is_empty() {
        return Err(DslError::empty(ctx_named("next_action")));
    }
    let parser = DslBlockParser::new(ctx_named("next_action"));
    let block = parser.parse_single_line(input)?;

    if block.command != "NEXT" {
        return Err(DslError::unknown_command(
            ctx_named("next_action"),
            &block.command,
        ));
    }

    let task_id = require_field(&block.fields, "task_id", &ctx_named("next_action"))?;
    let action = require_field(&block.fields, "action", &ctx_named("next_action"))?;
    let reason = block
        .fields
        .iter()
        .find(|f| f.key == "reason")
        .map(|f| f.value.clone())
        .unwrap_or_else(|| "next task from pyramid".to_string());

    Ok(coerce_json_value(&serde_json::json!({
        "task_id": task_id,
        "action": action,
        "reason": reason,
    })))
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_verdict_ok() {
        let result = parse_verdict_dsl("OK reason=\"step completed\"").unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["reason"], "step completed");
    }

    #[test]
    fn test_parse_verdict_retry() {
        let result = parse_verdict_dsl("RETRY reason=\"output incomplete\"").unwrap();
        assert_eq!(result["status"], "retry");
        assert_eq!(result["reason"], "output incomplete");
    }

    #[test]
    fn test_parse_verdict_empty_raises() {
        assert!(parse_verdict_dsl("").is_err());
    }

    #[test]
    fn test_parse_verdict_unknown_command() {
        assert!(parse_verdict_dsl("MAYBE reason=\"maybe\"").is_err());
    }

    #[test]
    fn test_parse_record_dsl_simple() {
        let result = parse_record_dsl_to_value("CLASSIFIER complexity=DIRECT risk=LOW").unwrap();
        assert_eq!(result["complexity"], "DIRECT");
        assert_eq!(result["risk"], "LOW");
    }

    #[test]
    fn test_parse_record_dsl_quoted() {
        let result = parse_record_dsl_to_value(
            "INTENT surface_intent=\"user greeting\" output_type=message",
        )
        .unwrap();
        assert_eq!(result["surface_intent"], "user greeting");
        assert_eq!(result["output_type"], "message");
    }

    #[test]
    fn test_parse_scope_dsl_basic() {
        let dsl = "\
SCOPE objective=\"inspect module\"
F path=\"src/main.rs\"
F path=\"src/lib.rs\"
Q text=\"main\"
END";
        let result = parse_scope_dsl(dsl).unwrap();
        assert_eq!(result["objective"], "inspect module");
        let paths = result["focus_paths"].as_array().unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "src/main.rs");
        assert_eq!(paths[1], "src/lib.rs");
        let queries = result["query_terms"].as_array().unwrap();
        assert_eq!(queries[0], "main");
    }

    #[test]
    fn test_parse_formula_dsl() {
        let result =
            parse_formula_dsl("FORMULA primary=inspect_reply reason=\"needs evidence\"").unwrap();
        assert_eq!(result["primary"], "inspect_reply");
        assert_eq!(result["reason"], "needs evidence");
    }

    #[test]
    fn test_parse_formula_dsl_with_alt() {
        let result = parse_formula_dsl(
            "FORMULA primary=inspect_reply alt1=execute_reply alt2=search_first reason=\"testing\"",
        )
        .unwrap();
        assert_eq!(result["primary"], "inspect_reply");
        let alts = result["alternatives"].as_array().unwrap();
        assert_eq!(alts.len(), 2);
        assert_eq!(alts[0], "execute_reply");
        assert_eq!(alts[1], "search_first");
    }

    #[test]
    fn test_parse_selection_dsl() {
        let dsl = "\
ITEM value=\"src/main.rs\"
ITEM value=\"src/lib.rs\"
REASON text=\"entry points\"
END";
        let result = parse_selection_dsl(dsl).unwrap();
        let items = result["items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0], "src/main.rs");
        assert_eq!(result["reason"], "entry points");
    }

    #[test]
    fn test_parse_critic_verdict_rich() {
        let result = parse_critic_verdict_dsl(
            "OK reason=\"looks good\" unsupported_claims=\"claim1,claim2\"",
        )
        .unwrap();
        assert_eq!(result["status"], "ok");
        assert_eq!(result["reason"], "looks good");
        let claims = result["unsupported_claims"].as_array().unwrap();
        assert_eq!(claims.len(), 2);
    }

    #[test]
    fn test_parse_workflow_planner_dsl_basic() {
        let dsl = "\
WORKFLOW objective=\"Inspect repo\" complexity=DIRECT risk=LOW needs_evidence=true preferred_formula=inspect_reply memory_id=\"\" reason=\"needs evidence\" scope_objective=\"Inspect repo\" scope_reason=\"need scope\"
F path=\"src/main.rs\"
IG glob=\"src/**\"
EG glob=\"target/**\"
Q text=\"workflow\"
A artifact=\"report\"
ALT formula=\"inspect_reply\"
ALT formula=\"execute_reply\"
END";
        let result = parse_workflow_planner_dsl(dsl).unwrap();
        assert_eq!(result["objective"], "Inspect repo");
        assert_eq!(result["complexity"], "DIRECT");
        assert_eq!(result["risk"], "LOW");
        assert_eq!(result["needs_evidence"], true);
        assert_eq!(result["preferred_formula"], "inspect_reply");
        let scope = result["scope"].as_object().unwrap();
        assert_eq!(scope["objective"], "Inspect repo");
        let paths = scope["focus_paths"].as_array().unwrap();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "src/main.rs");
        let alts = result["alternatives"].as_array().unwrap();
        assert_eq!(alts.len(), 2);
        assert_eq!(alts[0], "inspect_reply");
    }

    #[test]
    fn test_parse_auto_dsl_verdict() {
        let result = parse_auto_dsl("OK reason=\"done\"").unwrap();
        assert_eq!(result["status"], "ok");
    }

    #[test]
    fn test_parse_auto_dsl_record() {
        let result = parse_auto_dsl("ASSESS complexity=DIRECT risk=LOW").unwrap();
        assert_eq!(result["complexity"], "DIRECT");
    }

    #[test]
    fn test_parse_auto_dsl_scope() {
        let dsl = "\
SCOPE objective=\"test\"
F path=\"a.rs\"
END";
        let result = parse_auto_dsl(dsl).unwrap();
        assert_eq!(result["objective"], "test");
    }

    #[test]
    fn test_parse_auto_dsl_empty() {
        assert!(parse_auto_dsl("").is_err());
    }

    #[test]
    fn test_coerce_bool_values() {
        let result =
            parse_record_dsl_to_value("ASSESS needs_evidence=true needs_tools=false").unwrap();
        assert_eq!(result["needs_evidence"], true);
        assert_eq!(result["needs_tools"], false);
    }

    #[test]
    fn test_coerce_numeric_values() {
        let result = parse_record_dsl_to_value("ASSESS count=42 confidence=0.85").unwrap();
        assert_eq!(result["count"], 42);
        assert_eq!(result["confidence"], 0.85);
    }

    #[test]
    fn test_parse_list_dsl() {
        let dsl = "\
RESULT type=files
ITEM value=src/main.rs
ITEM value=src/lib.rs
END";
        let result = parse_list_dsl(dsl, "ITEM", "files").unwrap();
        assert_eq!(result["type"], "files");
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0], "src/main.rs");
        assert_eq!(files[1], "src/lib.rs");
    }

    #[test]
    fn test_parse_pyramid_block_dsl_basic() {
        let dsl = "\
OBJECTIVE text=\"Add tests to routing module\" risk=medium
GOAL text=\"Cover routing_infer.rs\" evidence_needed=true
GOAL text=\"Cover routing_calc.rs\" evidence_needed=false
TASK id=1 text=\"Test infer_digit_router\" status=ready
TASK id=2 text=\"Test parse_router_distribution\" status=pending
END";
        let result = parse_pyramid_block_dsl(dsl).unwrap();
        assert_eq!(result["objective"], "Add tests to routing module");
        assert_eq!(result["risk"], "medium");
        let goals = result["goals"].as_array().unwrap();
        assert_eq!(goals.len(), 2);
        assert_eq!(goals[0]["text"], "Cover routing_infer.rs");
        assert_eq!(goals[0]["evidence_needed"], true);
        assert_eq!(goals[1]["evidence_needed"], false);
        let tasks = result["tasks"].as_array().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["id"], 1);
        assert_eq!(tasks[0]["status"], "ready");
        assert_eq!(tasks[1]["id"], 2);
        assert_eq!(tasks[1]["status"], "pending");
    }

    #[test]
    fn test_parse_pyramid_block_empty_raises() {
        assert!(parse_pyramid_block_dsl("").is_err());
    }

    #[test]
    fn test_parse_next_action_dsl_basic() {
        let result =
            parse_next_action_dsl("NEXT task_id=1 action=edit reason=\"needs tests first\"")
                .unwrap();
        assert_eq!(result["task_id"], 1);
        assert_eq!(result["action"], "edit");
        assert_eq!(result["reason"], "needs tests first");
    }

    #[test]
    fn test_parse_next_action_dsl_minimal() {
        let result = parse_next_action_dsl("NEXT task_id=2 action=read").unwrap();
        assert_eq!(result["task_id"], 2);
        assert_eq!(result["action"], "read");
    }

    #[test]
    fn test_parse_next_action_empty_raises() {
        assert!(parse_next_action_dsl("").is_err());
    }

    #[test]
    fn test_parse_auto_dsl_objective() {
        let dsl = "\
OBJECTIVE text=\"Test\" risk=low
END";
        let result = parse_auto_dsl(dsl).unwrap();
        assert_eq!(result["objective"], "Test");
    }

    #[test]
    fn test_parse_auto_dsl_next() {
        let result = parse_auto_dsl("NEXT task_id=3 action=search").unwrap();
        assert_eq!(result["task_id"], 3);
        assert_eq!(result["action"], "search");
    }
}
