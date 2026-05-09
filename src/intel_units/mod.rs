//! @efficiency-role: domain-logic
//!
//! Intel Units
//!
//! This module contains Elma's trait-based intel units.
//! Re-exports from sub-modules for backward compatibility.

mod intel_units_batch_planner; // Task 501: Context-budget batch planner
mod intel_units_capability;
mod intel_units_claim_mapper;
mod intel_units_clarification; // Task 452: Clarification and completion tools
mod intel_units_classifier;
mod intel_units_continuity;
mod intel_units_document_summarizer;
mod intel_units_evidence_quality;
mod intel_units_evidence_staleness;
mod intel_units_evidence_sufficiency;
mod intel_units_final_cleaner;
mod intel_units_final_summary;
mod intel_units_goal_consistency;
mod intel_units_graph_assessment;
mod intel_units_workgraph_schema; // Tasks 763-769: Work graph schema intel
mod intel_units_repair;
mod intel_units_responder;
pub(crate) mod intel_units_task_management; // Task 494: Task creation intel unit
mod intel_units_thought_summary; // Task 622: Thought summary (auxiliary LLM)
mod intel_units_turn_summary; // Task 623: Document summarizer (scaffold)

// Re-export maestro types for external use

// Re-export all intel units for backward compatibility
pub(crate) use intel_units_batch_planner::*;
pub(crate) use intel_units_capability::*;
pub(crate) use intel_units_claim_mapper::*;
pub(crate) use intel_units_clarification::*; // Task 452
pub(crate) use intel_units_classifier::*;
pub(crate) use intel_units_continuity::*;
pub(crate) use intel_units_document_summarizer::*; // Task 623
pub(crate) use intel_units_evidence_quality::*;
pub(crate) use intel_units_evidence_staleness::*;
pub(crate) use intel_units_evidence_sufficiency::*;
pub(crate) use intel_units_final_cleaner::*;
pub(crate) use intel_units_final_summary::*;
pub(crate) use intel_units_goal_consistency::*;
pub(crate) use intel_units_graph_assessment::*;
pub(crate) use intel_units_repair::*;
pub(crate) use intel_units_responder::*;
pub(crate) use intel_units_thought_summary::*; // Task 622
pub(crate) use intel_units_turn_summary::*; // Task 501
pub(crate) use intel_units_workgraph_schema::*; // Tasks 763-769

use crate::intel_trait::*;
use crate::*;

// ============================================================================
// Evidence Compactor Unit
// ============================================================================

/// Evidence Compactor Intel Unit
///
/// Compacts large evidence into a more concise form.
pub(crate) struct EvidenceCompactorUnit {
    profile: Profile,
}

impl EvidenceCompactorUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for EvidenceCompactorUnit {
    fn name(&self) -> &'static str {
        "evidence_compactor"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, _context: &IntelContext) -> Result<()> {
        // No specific pre-flight checks
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let objective = context
            .extra("objective")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let purpose = context
            .extra("purpose")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let scope = context
            .extra("scope")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let cmd = context
            .extra("cmd")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        let output = context
            .extra("output")
            .cloned()
            .unwrap_or_else(|| serde_json::json!(context.workspace_facts));
        let result: EvidenceCompact = execute_intel_json_from_user_content(
            &context.client,
            &self.profile,
            crate::intel_narrative::build_evidence_compactor_narrative(
                &objective, &purpose, &scope, &cmd, &output,
            ),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::to_value(result)?,
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("compacted_evidence").is_none() && output.get("summary").is_none() {
            return Err(anyhow::anyhow!(
                "Missing 'compacted_evidence' or 'summary' field"
            ));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "compacted_evidence": context.workspace_facts,
                "reason": "fallback: returned original evidence".to_string(),
            }),
            &format!("evidence compactor failed: {}", error),
        ))
    }
}

// ============================================================================
// Formatter Unit
// ============================================================================

/// Formatter Intel Unit
///
/// Cleans up and structures the final response for terminal display.
pub(crate) struct FormatterUnit {
    profile: Profile,
}

impl FormatterUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for FormatterUnit {
    fn name(&self) -> &'static str {
        "formatter"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        if context.user_message.trim().is_empty() {
            return Err(anyhow::anyhow!("Empty input text"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        // Formatter uses text-out task logic but for the "user_message" content which is the draft to format
        let result = execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            context.user_message.clone(),
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({ "formatted_text": result }),
            0.9,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get("formatted_text").is_none() {
            return Err(anyhow::anyhow!("Missing 'formatted_text' field"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "formatted_text": context.user_message.clone(),
                "reason": "fallback: return original text",
            }),
            &format!("formatter failed: {}", error),
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

