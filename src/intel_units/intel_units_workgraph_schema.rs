//! @efficiency-role: domain-logic
//!
//! Work Graph Schema Intel — Tasks 763-769.
//!
//! A lightweight model call that derives an abstract execution plan from the
//! user request. The schema is deliberately shallow: 3-5 phases with a small
//! action vocabulary. The model knows nothing about actual workspace files.
//! Patterns:
//!
//! - One intel unit, one role: produce a shallow execution outline
//! - Model outputs only generic phases, not concrete paths
//! - Workspace exploration populates the placeholders later
//! - Shallow JSON so small models can produce it reliably

use crate::*;
use serde::{Deserialize, Serialize};

/// A single phase in the work graph schema.
/// The model fills in ONLY generic information — no actual filenames or paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SchemaPhase {
    /// Human-readable label for the phase (shown in transcript, UI).
    pub label: String,
    /// Action verb from the constrained vocabulary:
    ///   "discover"  — list/glob a directory to find files
    ///   "read_all"  — read every file in a discovered scope
    ///   "read_one"  — read a specific file or resource
    ///   "shell"     — run a shell command
    ///   "answer"    — synthesize a final answer from gathered evidence
    pub action: String,
    /// Natural-language hint for where to discover or what to read.
    /// For "discover": "documentation directory"
    /// For "read_all":  "the discovered documentation files"
    /// For "read_one":  "the project README"
    /// The system resolves these to actual paths during exploration.
    pub scope_hint: String,
    /// Whether evidence from this phase must be consumed before moving on.
    /// true for "read_all", "read_one" — evidence must be gathered.
    /// false for "answer" — no evidence collection needed.
    pub requires_evidence: bool,
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
    /// Whether the schema is non-trivial (has actionable phases).
    pub fn has_phases(&self) -> bool {
        !self.phases.is_empty()
    }

    /// The first phase that requires workspace discovery.
    pub fn first_discovery_phase(&self) -> Option<&SchemaPhase> {
        self.phases.iter().find(|p| p.action == "discover")
    }

    /// Phases that consume discovered scope (need population).
    pub fn consumable_phases(&self) -> Vec<&SchemaPhase> {
        self.phases
            .iter()
            .filter(|p| p.action == "read_all")
            .collect()
    }
}

/// Fallback schema for Direct/Investigate complexity — single answer phase only.
pub(crate) fn direct_schema(raw_objective: &str) -> WorkGraphSchema {
    WorkGraphSchema {
        phases: vec![SchemaPhase {
            label: "Answer the request".to_string(),
            action: "answer".to_string(),
            scope_hint: raw_objective.to_string(),
            requires_evidence: false,
        }],
    }
}

/// Request a work graph schema from the model.
/// Uses a focused prompt with a constrained output contract.
/// The model receives only the user request — no workspace info.
pub(crate) async fn request_workgraph_schema(
    client: &reqwest::Client,
    profile: &Profile,
    user_request: &str,
) -> Result<WorkGraphSchema> {
    // Build a focused prompt: what the unit does, what contract it fulfills.
    // Principle-first: no examples, the action vocabulary is the contract.
    /// Build a focused prompt: what the unit does, what contract it fulfills.
    /// Principle-first: the action vocabulary IS the contract.
    /// Kept deliberately sparse — <512 output tokens, the model only produces
    /// a short JSON list.
    let system_prompt = "\
Produce a short JSON execution plan for the user's request.

Output: {\"phases\":[{\"label\":\"...\",\"action\":\"...\",\"scope_hint\":\"...\",\"requires_evidence\":bool}]}

Actions (use exactly one of these words):
  discover  — list a directory to find files
  read_all  — read every file from a discovered scope
  read_one  — read a single specific resource
  shell     — run a shell command
  answer    — synthesize a final answer (always last)

You know nothing about actual workspace files. Never invent filenames or paths.
3-5 phases. Keep scope_hint under 60 chars.
Output ONLY valid JSON. No explanation. No markdown.";

    let user_content = format!("Request: {}", user_request);
    let req = crate::llm_config::chat_request_from_profile(
        profile,
        vec![
            ChatMessage::simple("system", system_prompt),
            ChatMessage::simple("user", &user_content),
        ],
        crate::llm_config::ChatRequestOptions {
            temperature: Some(0.0),
            max_tokens: Some(256),
            stream: Some(false),
            ..Default::default()
        },
    );

    let chat_url = crate::intel_trait::intel_chat_url(profile)?;
    let resp = crate::ui::ui_chat::chat_once_with_timeout(client, &chat_url, &req, profile.timeout_s)
        .await
        .context("WorkGraph schema request failed")?;

    let content = resp
        .choices
        .first()
        .and_then(|c| c.message.content.clone())
        .unwrap_or_else(|| "{}".to_string());
    let content = crate::text_utils::strip_thinking_blocks(&content);

    let schema: WorkGraphSchema = serde_json::from_str::<WorkGraphSchema>(&content)
        .or_else(|_| {
            if let Some(start) = content.find("```json") {
                let inner = &content[start + 7..];
                if let Some(end) = inner.find("```") {
                    serde_json::from_str(&inner[..end])
                } else {
                    Err(serde::de::Error::custom("no closing ```"))
                }
            } else if let Some(start) = content.find('{') {
                if let Some(end) = content.rfind('}') {
                    serde_json::from_str(&content[start..=end])
                } else {
                    Err(serde::de::Error::custom("no closing brace"))
                }
            } else {
                Err(serde::de::Error::custom("no JSON found"))
            }
        })
        .unwrap_or_else(|_| direct_schema(user_request));

    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_schema() {
        let schema = direct_schema("what is 2+2?");
        assert_eq!(schema.phases.len(), 1);
        assert_eq!(schema.phases[0].action, "answer");
    }

    #[test]
    fn test_workgraph_schema_default() {
        let schema = WorkGraphSchema::default();
        assert!(schema.phases.is_empty());
        assert!(!schema.has_phases());
    }

    #[test]
    fn test_schema_serde_roundtrip() {
        let json = r#"{"phases":[{"label":"Discover docs","action":"discover","scope_hint":"documentation directory","requires_evidence":false},{"label":"Read docs","action":"read_all","scope_hint":"the discovered documentation files","requires_evidence":true}]}"#;
        let schema: WorkGraphSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.phases.len(), 2);
        assert_eq!(schema.phases[0].action, "discover");
        assert_eq!(schema.phases[1].action, "read_all");
        assert!(!schema.phases[0].requires_evidence);
        assert!(schema.phases[1].requires_evidence);
        assert!(schema.has_phases());
    }

    #[test]
    fn test_first_discovery_phase() {
        let json = r#"{"phases":[
            {"label":"List docs","action":"discover","scope_hint":"docs","requires_evidence":false},
            {"label":"Read docs","action":"read_all","scope_hint":"discovered docs","requires_evidence":true}
        ]}"#;
        let schema: WorkGraphSchema = serde_json::from_str(json).unwrap();
        let discovery = schema.first_discovery_phase().unwrap();
        assert_eq!(discovery.action, "discover");
        assert_eq!(discovery.scope_hint, "docs");
    }

    #[test]
    fn test_consumable_phases() {
        let json = r#"{"phases":[
            {"label":"Discover","action":"discover","scope_hint":"docs","requires_evidence":false},
            {"label":"Read all","action":"read_all","scope_hint":"discovered","requires_evidence":true},
            {"label":"Answer","action":"answer","scope_hint":"synthesize","requires_evidence":false}
        ]}"#;
        let schema: WorkGraphSchema = serde_json::from_str(json).unwrap();
        let consumable = schema.consumable_phases();
        assert_eq!(consumable.len(), 1);
        assert_eq!(consumable[0].action, "read_all");
    }
}
