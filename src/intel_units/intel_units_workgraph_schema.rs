//! @efficiency-role: domain-logic
//!
//! Work Graph Schema Intel — Tasks 763-769.
//!
//! Two-pass planning architecture:
//!   Pass 1: Comprehensive Planner — produces a detailed numbered outline
//!           (easier for small models than structured JSON)
//!   Pass 2: Schema Converter — converts the outline to WorkGraphSchema JSON
//!           with depth levels assigned from the outline numbering.
//!
//! Planning is done BEFORE the work graph. The graph only executes what
//! the plan decided. Structure: Objective(depth 0) → SubGoal(depth 1) →
//! Instruction(depth 2, smallest unit).

use crate::*;
use serde::{Deserialize, Serialize};

/// A single phase in the work graph schema.
/// Depth alone determines the role:
///   0 = Objective (root container)
///   1 = SubGoal (actionable step, groups its depth-2 children)
///   2 = Instruction (smallest unit, executed by the model via tool-calling)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaPhase {
    pub label: String,
    pub depth: u8,
}

/// The complete work graph schema — a flat ordered list of execution phases.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct WorkGraphSchema {
    /// Ordered execution phases.
    pub phases: Vec<SchemaPhase>,
}

impl Default for WorkGraphSchema {
    fn default() -> Self {
        Self { phases: Vec::new() }
    }
}

impl WorkGraphSchema {
    pub fn has_phases(&self) -> bool {
        !self.phases.is_empty()
    }
}

/// Fallback schema for Direct complexity — single phase only (depth 0).
pub(crate) fn direct_schema(raw_objective: &str) -> WorkGraphSchema {
    WorkGraphSchema {
        phases: vec![SchemaPhase {
            label: "Answer the request".to_string(),
            depth: 0,
        }],
    }
}

// ── Two-Pass Planning ───────────────────────────────────────────────────────
//
// Pass 1 (Model):  request_comprehensive_plan()   → numbered outline text
// Pass 2 (Deterministic):  parse_outline_to_schema() → WorkGraphSchema JSON
// Planning is done BEFORE the work graph. The graph only executes.

/// Request a detailed numbered outline from the model (Pass 1).
/// Small models produce text outlines more reliably than structured JSON.
async fn request_comprehensive_plan(
    client: &reqwest::Client,
    profile: &Profile,
    user_request: &str,
) -> Result<String> {
    let system_prompt = "\
Create a comprehensive execution plan for the user's request as a numbered outline.

Format: depth-based numbering
  1.   = top-level objective (what to achieve)
   1.1.  = sub-goal (concrete step toward the objective)
    1.1.1. = instruction (specific action to take)

Rules:
- Include 1-3 levels of depth as needed.
- Each line describes ONE concrete action.
- Use action verbs: read, list, find, run, ls, execute, summarize, synthesize.
- Never invent filenames — use descriptive placeholders instead.
- Keep lines short (under 80 chars).
- Place the final synthesis step last.
- Output ONLY the numbered outline. No preamble, no explanation.";

    let user_content = format!("Request: {}", user_request);
    let req = crate::llm_config::chat_request_from_profile(
        profile,
        vec![
            ChatMessage::simple("system", system_prompt),
            ChatMessage::simple("user", &user_content),
        ],
        crate::llm_config::ChatRequestOptions {
            temperature: Some(0.0),
            max_tokens: Some(512),
            stream: Some(false),
            ..Default::default()
        },
    );

    let chat_url = crate::intel_trait::intel_chat_url(profile)?;
    let resp = crate::ui::ui_chat::chat_once_with_timeout(client, &chat_url, &req, profile.timeout_s)
        .await
        .context("Comprehensive planner request failed")?;

    let content = resp
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_default();
    Ok(crate::text_utils::strip_thinking_blocks(&content))
}

/// Deterministically parse a numbered outline into WorkGraphSchema.
/// No model call — depth is derived from outline numbering structure.
fn parse_outline_to_schema(outline: &str) -> WorkGraphSchema {
    let mut phases: Vec<SchemaPhase> = Vec::new();

    for line in outline.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let (depth, label) = parse_outline_line(trimmed);
        if label.is_empty() {
            continue;
        }

        let short_label = if label.len() > 80 {
            format!("{}...", &label[..77])
        } else {
            label
        };

        phases.push(SchemaPhase {
            label: short_label,
            depth,
        });
    }

    if phases.is_empty() {
        return WorkGraphSchema::default();
    }
    WorkGraphSchema { phases }
}

/// Extract (depth, label_text) from a numbered outline line.
fn parse_outline_line(line: &str) -> (u8, String) {
    let leading_spaces = line.chars().take_while(|c| c.is_whitespace()).count();
    let rest = &line[leading_spaces..];
    if let Some((num_part, label)) = rest.split_once(' ') {
        let cleaned = num_part.trim_end_matches('.');
        let dot_count = cleaned.chars().filter(|&c| c == '.').count();
        // "1." = 0 dots → depth 0, "1.1." = 1 dot → depth 1, "1.1.1." = 2 dots → depth 2
        let depth = dot_count.min(2) as u8;
        return (depth, label.trim().to_string());
    }

    // Fallback: use leading spaces (4 spaces per depth level)
    if leading_spaces > 0 {
        let depth = ((leading_spaces / 4) as u8).min(2);
        let label = rest.trim().to_string();
        if !label.is_empty() {
            return (depth, label);
        }
    }

    (0, String::new())
}

/// Two-pass request: comprehensive planner → deterministic schema parser.
/// Falls back to direct schema on planner failure.
pub(crate) async fn request_workgraph_schema(
    client: &reqwest::Client,
    profile: &Profile,
    user_request: &str,
) -> Result<WorkGraphSchema> {
    let outline = request_comprehensive_plan(client, profile, user_request).await?;
    if outline.trim().is_empty() {
        return Ok(direct_schema(user_request));
    }
    let schema = parse_outline_to_schema(&outline);
    if schema.has_phases() {
        Ok(schema)
    } else {
        Ok(direct_schema(user_request))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_schema() {
        let schema = direct_schema("what is 2+2?");
        assert_eq!(schema.phases.len(), 1);
        assert_eq!(schema.phases[0].depth, 0);
    }

    #[test]
    fn test_workgraph_schema_default() {
        let schema = WorkGraphSchema::default();
        assert!(schema.phases.is_empty());
        assert!(!schema.has_phases());
    }

    #[test]
    fn test_schema_serde_roundtrip() {
        let json = r#"{"phases":[{"label":"Discover docs","depth":1},{"label":"Read docs","depth":2}]}"#;
        let schema: WorkGraphSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.phases.len(), 2);
        assert_eq!(schema.phases[0].depth, 1);
        assert_eq!(schema.phases[1].depth, 2);
        assert_eq!(schema.phases[0].label, "Discover docs");
        assert!(schema.has_phases());
    }

    #[test]
    fn test_parse_outline() {
        let outline = "\
1. Read all docs
  1.1. List files
    1.1.1. ls workspace
  1.2. Read each file
    1.2.1. read README
    1.2.2. read AGENTS";
        let schema = parse_outline_to_schema(outline);
        assert_eq!(schema.phases.len(), 6);
        assert_eq!(schema.phases[0].depth, 0); // 1.
        assert_eq!(schema.phases[1].depth, 1); // 1.1.
        assert_eq!(schema.phases[2].depth, 2); // 1.1.1.
        assert_eq!(schema.phases[3].depth, 1); // 1.2.
        assert_eq!(schema.phases[4].depth, 2); // 1.2.1.
        assert_eq!(schema.phases[5].depth, 2); // 1.2.2.
    }
}
