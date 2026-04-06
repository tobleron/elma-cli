//! @efficiency-role: domain-logic
//!
//! Maestro Intel Unit — generates numbered high-level instructions
//! from the user's objective and context narrative.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

/// Extract first JSON object from text, stripping markdown code fences.
fn extract_json_from_text(text: &str) -> String {
    let t = text.trim();
    // Strip markdown code fences
    let t = t.strip_prefix("```json").unwrap_or(t);
    let t = t.strip_prefix("```").unwrap_or(t);
    let t = t.strip_suffix("```").unwrap_or(t);
    let t = t.trim();

    // Find first { and last }
    if let (Some(start), Some(end)) = (t.find('{'), t.rfind('}')) {
        if end > start {
            return t[start..=end].to_string();
        }
    }
    t.to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct MaestroInstruction {
    pub(crate) num: u32,
    pub(crate) instruction: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct MaestroOutput {
    pub(crate) steps: Vec<MaestroInstruction>,
}

pub(crate) struct MaestroUnit {
    profile: Profile,
}

impl MaestroUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for MaestroUnit {
    fn name(&self) -> &'static str {
        "the_maestro"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty user message"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let intent = context
            .extra("intent")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let expert_advice = context
            .extra("expert_advice")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let narrative = build_maestro_narrative(
            &context.user_message,
            intent,
            expert_advice,
            &context.workspace_facts,
            &context.workspace_brief,
            &context.conversation_excerpt,
        );

        // Use text extraction for robustness — strip markdown fences manually
        let raw_text = crate::intel_trait::execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            narrative,
        )
        .await?;

        // Find JSON object in text (strip markdown code fences)
        let cleaned = extract_json_from_text(&raw_text);

        // Accept both {"steps": [{num, instruction}]} AND flat {num, instruction}
        // Small models often drop the outer wrapper
        let result: MaestroOutput = if let Ok(steps_wrapper) =
            serde_json::from_str::<MaestroOutput>(&cleaned)
        {
            steps_wrapper
        } else if let Ok(single_instr) = serde_json::from_str::<MaestroInstruction>(&cleaned) {
            // Model returned a single instruction without wrapper — wrap it
            MaestroOutput {
                steps: vec![single_instr],
            }
        } else if let Ok(instr_array) = serde_json::from_str::<Vec<MaestroInstruction>>(&cleaned) {
            // Model returned a bare array of instructions
            MaestroOutput { steps: instr_array }
        } else {
            return Err(anyhow::anyhow!(
                "Maestro JSON parse error: could not parse as {{\"steps\": [...]}} or [{{\"num\":..., \"instruction\":...}}]. Raw: {}",
                &cleaned.chars().take(300).collect::<String>()
            ));
        };

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("steps").is_none() {
            return Err(anyhow::anyhow!("Missing 'steps' field in maestro output"));
        }
        Ok(())
    }

    fn fallback(&self, _context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);
        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "steps": [{ "num": 1, "instruction": "Address the user's request using available tools and knowledge." }]
            }),
            &format!("maestro failed: {}", error),
        ))
    }
}

pub(crate) fn build_maestro_narrative(
    user_message: &str,
    intent: &str,
    expert_advice: &str,
    workspace_facts: &str,
    workspace_brief: &str,
    conversation: &[ChatMessage],
) -> String {
    let conversation_text = crate::tuning_support::conversation_excerpt(conversation, 8);

    format!(
        r#"USER MESSAGE:
{user_message}

INTENT:
{intent}

EXPERT ADVICE:
{expert_advice}

WORKSPACE FACTS:
{facts}

WORKSPACE BRIEF:
{brief}

CONVERSATION SO FAR (most recent last):
{conversation}"#,
        user_message = user_message.trim(),
        intent = intent.trim(),
        expert_advice = expert_advice.trim(),
        facts = workspace_facts.trim(),
        brief = workspace_brief.trim(),
        conversation = conversation_text,
    )
}
