//! @efficiency-role: data-model
//!
//! Types - API and Runtime Types

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SnapshotManifest {
    pub(crate) version: u32,
    pub(crate) snapshot_id: String,
    pub(crate) created_unix_s: u64,
    pub(crate) automatic: bool,
    pub(crate) reason: String,
    pub(crate) repo_root: String,
    pub(crate) git_aware: bool,
    pub(crate) scope_mode: String,
    pub(crate) file_count: u64,
    pub(crate) files: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct SnapshotCreateResult {
    pub(crate) snapshot_id: String,
    pub(crate) snapshot_dir: PathBuf,
    pub(crate) manifest_path: PathBuf,
    pub(crate) file_count: u64,
    pub(crate) automatic: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct RollbackResult {
    pub(crate) snapshot_id: String,
    pub(crate) manifest_path: PathBuf,
    pub(crate) restored_files: u64,
    pub(crate) removed_files: u64,
    pub(crate) verified_files: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelsList {
    pub(crate) data: Option<Vec<ModelItem>>,
    pub(crate) models: Option<Vec<ModelItem>>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelItem {
    pub(crate) id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChatMessage {
    pub(crate) role: String,
    pub(crate) content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) summarized: bool,
}

impl ChatMessage {
    pub fn simple(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: content.to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            summarized: false,
        }
    }

    pub fn mark_summarized(&mut self) {
        self.summarized = true;
    }

    pub fn is_summarized(&self) -> bool {
        self.summarized
    }
}

// Tool Calling Types
// ToolDefinition and ToolFunction are now defined in the elma-tools crate.
// Re-export them here for backward compatibility with existing imports.
pub(crate) use elma_tools::{ToolDefinition, ToolFunction};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolCall {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) call_type: String,
    pub(crate) function: ToolFunctionCall,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolFunctionCall {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ChatCompletionRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) temperature: f64,
    pub(crate) top_p: f64,
    pub(crate) stream: bool,
    pub(crate) max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) n_probs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repeat_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_format: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) grammar: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<Vec<ToolDefinition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ArtifactRecord {
    pub(crate) artifact_id: String,
    pub(crate) source_step_id: String,
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) bytes_written: u64,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) created_unix_s: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct ShellExecutionResult {
    pub(crate) exit_code: i32,
    pub(crate) inline_text: String,
    pub(crate) bytes_written: u64,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) artifact_path: Option<PathBuf>,
    pub(crate) artifact_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
    pub(crate) choices: Vec<Choice>,
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) created: Option<i64>,
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) system_fingerprint: Option<String>,
    #[serde(default)]
    pub(crate) usage: Option<Usage>,
    #[serde(default)]
    pub(crate) timings: Option<Timings>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    pub(crate) message: ChoiceMessage,
    #[serde(default)]
    pub(crate) finish_reason: Option<String>,
    #[serde(default)]
    pub(crate) logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ChoiceMessage {
    #[allow(dead_code)]
    pub(crate) role: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) reasoning_content: Option<String>,
    #[serde(default)]
    pub(crate) tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Usage {
    #[serde(default)]
    pub(crate) prompt_tokens: Option<u64>,
    #[serde(default)]
    pub(crate) completion_tokens: Option<u64>,
    #[serde(default)]
    pub(crate) total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Timings {
    #[serde(default)]
    pub(crate) prompt_n: Option<u64>,
    #[serde(default)]
    pub(crate) prompt_ms: Option<f64>,
    #[serde(default)]
    pub(crate) predicted_n: Option<u64>,
    #[serde(default)]
    pub(crate) predicted_ms: Option<f64>,
    #[serde(default)]
    pub(crate) predicted_per_second: Option<f64>,
    #[serde(default)]
    pub(crate) cache_n: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StepResult {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) purpose: String,
    pub(crate) depends_on: Vec<String>,
    pub(crate) success_condition: String,
    pub(crate) ok: bool,
    pub(crate) summary: String,
    pub(crate) command: Option<String>,
    pub(crate) raw_output: Option<String>,
    pub(crate) exit_code: Option<i32>,
    pub(crate) output_bytes: Option<u64>,
    pub(crate) truncated: bool,
    pub(crate) timed_out: bool,
    pub(crate) artifact_path: Option<String>,
    pub(crate) artifact_kind: Option<String>,
    pub(crate) outcome_status: Option<String>,
    pub(crate) outcome_reason: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ComplexityAssessment {
    #[serde(default)]
    pub(crate) complexity: String,
    #[serde(default)]
    pub(crate) needs_evidence: bool,
    #[serde(default)]
    pub(crate) needs_tools: bool,
    #[serde(default)]
    pub(crate) needs_decision: bool,
    #[serde(default)]
    pub(crate) needs_plan: bool,
    #[serde(default)]
    pub(crate) risk: String,
    #[serde(default)]
    pub(crate) suggested_pattern: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct FormulaSelection {
    #[serde(default)]
    pub(crate) primary: String,
    #[serde(default)]
    pub(crate) alternatives: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) memory_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct WorkflowPlannerOutput {
    #[serde(default)]
    pub(crate) objective: String,
    #[serde(default)]
    pub(crate) complexity: String,
    #[serde(default)]
    pub(crate) risk: String,
    #[serde(default)]
    pub(crate) needs_evidence: bool,
    #[serde(default)]
    pub(crate) scope: ScopePlan,
    #[serde(default)]
    pub(crate) preferred_formula: String,
    #[serde(default)]
    pub(crate) alternatives: Vec<String>,
    #[serde(default)]
    pub(crate) memory_id: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct EvidenceModeDecision {
    #[serde(default)]
    pub(crate) mode: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ExpertAdvisorAdvice {
    #[serde(default)]
    pub(crate) expert_advice: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct CommandRepair {
    #[serde(default)]
    pub(crate) cmd: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct RepairSemanticsVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct ExecutionSufficiencyVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) program: Option<super::types_core::Program>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct OutcomeVerificationVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct MemoryGateVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct CommandPreflightVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) cmd: String,
    #[serde(default)]
    pub(crate) question: String,
    #[serde(default)]
    pub(crate) execution_mode: String,
    #[serde(default)]
    pub(crate) artifact_kind: String,
    #[serde(default)]
    pub(crate) preview_strategy: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct SelectionOutput {
    #[serde(default)]
    pub(crate) items: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct RenameSuggestion {
    #[serde(default)]
    pub(crate) identifier: String,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ScopePlan {
    #[serde(default)]
    pub(crate) objective: String,
    #[serde(default)]
    pub(crate) focus_paths: Vec<String>,
    #[serde(default)]
    pub(crate) include_globs: Vec<String>,
    #[serde(default)]
    pub(crate) exclude_globs: Vec<String>,
    #[serde(default)]
    pub(crate) query_terms: Vec<String>,
    #[serde(default)]
    pub(crate) expected_artifacts: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct EvidenceCompact {
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) key_facts: Vec<String>,
    #[serde(default)]
    pub(crate) noise: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ArtifactClassification {
    #[serde(default)]
    pub(crate) safe: Vec<String>,
    #[serde(default)]
    pub(crate) maybe: Vec<String>,
    #[serde(default)]
    pub(crate) keep: Vec<String>,
    #[serde(default)]
    pub(crate) ignore: Vec<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct ClaimCheckVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) unsupported_claims: Vec<String>,
    #[serde(default)]
    pub(crate) missing_points: Vec<String>,
    #[serde(default)]
    pub(crate) rewrite_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FormulaMemoryRecord {
    pub(crate) id: String,
    pub(crate) created_unix_s: u64,
    #[serde(default)]
    pub(crate) model_id: String,
    #[serde(default)]
    pub(crate) active_run_id: String,
    pub(crate) user_message: String,
    pub(crate) route: String,
    pub(crate) complexity: String,
    pub(crate) formula: String,
    pub(crate) objective: String,
    pub(crate) title: String,
    pub(crate) program_signature: String,
    #[serde(default)]
    pub(crate) last_success_unix_s: u64,
    #[serde(default)]
    pub(crate) last_failure_unix_s: u64,
    #[serde(default)]
    pub(crate) success_count: u64,
    #[serde(default)]
    pub(crate) failure_count: u64,
    #[serde(default)]
    pub(crate) disabled: bool,
    #[serde(default)]
    pub(crate) artifact_mode_capable: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AgentPlan {
    pub(crate) objective: String,
    pub(crate) current_program: super::types_core::Program,
    pub(crate) program_history: Vec<super::types_core::Program>,
    pub(crate) attempts: u32,
    pub(crate) executed_steps: usize,
    pub(crate) max_steps: usize,
    pub(crate) recovery_failures: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct AutonomousLoopOutcome {
    pub(crate) program: super::types_core::Program,
    pub(crate) step_results: Vec<StepResult>,
    pub(crate) final_reply: Option<String>,
    pub(crate) reasoning_clean: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(crate) struct CriticVerdict {
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
    #[serde(default)]
    pub(crate) program: Option<super::types_core::Program>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct RiskReviewVerdict {
    #[serde(default)]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) reason: String,
}
