//! @efficiency-role: domain-logic
//!
//! Program Step Helpers and JSON Serialization

use crate::*;

pub(crate) fn step_kind(s: &Step) -> &'static str {
    match s {
        Step::Shell { .. } => "shell",
        Step::Read { .. } => "read",
        Step::Search { .. } => "search",
        Step::Select { .. } => "select",
        Step::Plan { .. } => "plan",
        Step::MasterPlan { .. } => "masterplan",
        Step::Decide { .. } => "decide",
        Step::Summarize { .. } => "summarize",
        Step::Edit { .. } => "edit",
        Step::Reply { .. } => "reply",
    }
}

pub(crate) fn step_id(s: &Step) -> &str {
    match s {
        Step::Shell { id, .. } | Step::Read { id, .. } | Step::Search { id, .. } => id,
        Step::Select { id, .. } => id,
        Step::Plan { id, .. } => id,
        Step::MasterPlan { id, .. } => id,
        Step::Decide { id, .. } => id,
        Step::Summarize { id, .. } => id,
        Step::Edit { id, .. } => id,
        Step::Reply { id, .. } => id,
    }
}

pub(crate) fn step_common(s: &Step) -> &StepCommon {
    match s {
        Step::Shell { common, .. } | Step::Read { common, .. } | Step::Search { common, .. } => {
            common
        }
        Step::Select { common, .. } => common,
        Step::Plan { common, .. } => common,
        Step::MasterPlan { common, .. } => common,
        Step::Decide { common, .. } => common,
        Step::Summarize { common, .. } => common,
        Step::Edit { common, .. } => common,
        Step::Reply { common, .. } => common,
    }
}

pub(crate) fn step_purpose(s: &Step) -> String {
    let common = step_common(s);
    if !common.purpose.trim().is_empty() {
        return common.purpose.trim().to_string();
    }
    match s {
        Step::Shell { .. } => "shell".to_string(),
        Step::Read { .. } => "read".to_string(),
        Step::Search { .. } => "search".to_string(),
        Step::Select { .. } => "select".to_string(),
        Step::Plan { .. } => "plan".to_string(),
        Step::MasterPlan { .. } => "masterplan".to_string(),
        Step::Decide { .. } => "decide".to_string(),
        Step::Summarize { .. } => "summarize".to_string(),
        Step::Edit { .. } => "edit".to_string(),
        Step::Reply { .. } => "answer".to_string(),
    }
}

pub(crate) fn step_success_condition(s: &Step) -> String {
    step_common(s).success_condition.trim().to_string()
}

pub(crate) fn step_depends_on(s: &Step) -> Vec<String> {
    step_common(s).depends_on.clone()
}

pub(crate) fn program_step_json(step: &Step) -> serde_json::Value {
    let base = serde_json::json!({
        "id": step_id(step),
        "type": step_kind(step),
        "purpose": step_purpose(step),
        "depends_on": step_depends_on(step),
        "success_condition": step_success_condition(step),
    });
    let mut obj = base.as_object().cloned().unwrap_or_default();
    match step {
        Step::Shell { cmd, .. } => {
            obj.insert("cmd".to_string(), serde_json::json!(cmd));
            obj.insert(
                "placeholder_refs".to_string(),
                serde_json::json!(command_placeholder_refs(cmd)),
            );
        }
        Step::Read { path, .. } => {
            obj.insert("path".to_string(), serde_json::json!(path.trim()));
        }
        Step::Search { query, paths, .. } => {
            obj.insert("query".to_string(), serde_json::json!(query.trim()));
            obj.insert("paths".to_string(), serde_json::json!(paths));
        }
        Step::Select { instructions, .. } => {
            obj.insert(
                "instructions".to_string(),
                serde_json::json!(instructions.trim()),
            );
        }
        Step::Plan { goal, .. } | Step::MasterPlan { goal, .. } => {
            obj.insert("goal".to_string(), serde_json::json!(goal.trim()));
        }
        Step::Decide { prompt, .. } => {
            obj.insert("prompt".to_string(), serde_json::json!(prompt.trim()));
        }
        Step::Summarize {
            text, instructions, ..
        } => {
            obj.insert(
                "instructions".to_string(),
                serde_json::json!(instructions.trim()),
            );
            if !text.trim().is_empty() {
                obj.insert(
                    "text_preview".to_string(),
                    serde_json::json!(preview_text(text, 6)),
                );
            }
        }
        Step::Edit { spec, .. } => {
            obj.insert("path".to_string(), serde_json::json!(spec.path.trim()));
            obj.insert(
                "operation".to_string(),
                serde_json::json!(spec.operation.trim()),
            );
            if !spec.find.trim().is_empty() {
                obj.insert(
                    "find_preview".to_string(),
                    serde_json::json!(preview_text(&spec.find, 3)),
                );
            }
            if !spec.replace.trim().is_empty() {
                obj.insert(
                    "replace_preview".to_string(),
                    serde_json::json!(preview_text(&spec.replace, 3)),
                );
            }
            if !spec.content.trim().is_empty() {
                obj.insert(
                    "content_preview".to_string(),
                    serde_json::json!(preview_text(&spec.content, 6)),
                );
            }
        }
        Step::Reply { instructions, .. } => {
            obj.insert(
                "instructions".to_string(),
                serde_json::json!(instructions.trim()),
            );
        }
    }
    serde_json::Value::Object(obj)
}

pub(crate) fn step_result_json(result: &StepResult) -> serde_json::Value {
    serde_json::json!({
        "id": result.id,
        "type": result.kind,
        "purpose": result.purpose,
        "depends_on": result.depends_on,
        "success_condition": result.success_condition,
        "ok": result.ok,
        "summary": result.summary,
        "command": result.command,
        "raw_output": result.raw_output,
        "exit_code": result.exit_code,
        "output_bytes": result.output_bytes,
        "truncated": result.truncated,
        "timed_out": result.timed_out,
        "artifact_path": result.artifact_path,
        "artifact_kind": result.artifact_kind,
        "outcome_status": result.outcome_status,
        "outcome_reason": result.outcome_reason,
    })
}
