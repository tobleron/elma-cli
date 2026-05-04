//! @efficiency-role: domain-logic
//!
//! Document Summarizer Intel Unit (Task 623)
//!
//! Scaffold only — NOT wired yet. Will be activated by a future skill recipe.
//!
//! One job: summarize the most important parts of a document into a compact
//! summary that the main model can use to decide whether to go in-depth on
//! that document. Runs on the auxiliary LLM.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DocumentSummaryOutput {
    pub summary: String,
    pub key_topics: Vec<String>,
    pub estimated_relevance: f64,
}

pub(crate) struct DocumentSummarizerUnit {
    profile: Profile,
}

impl DocumentSummarizerUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }
}

impl IntelUnit for DocumentSummarizerUnit {
    fn name(&self) -> &'static str {
        "document_summarizer"
    }

    fn profile(&self) -> &Profile {
        &self.profile
    }

    fn pre_flight(&self, context: &IntelContext) -> Result<()> {
        let has_text = context
            .extra("document_text")
            .and_then(|v| v.as_str())
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if !has_text {
            return Err(anyhow::anyhow!("No document text to summarize"));
        }
        Ok(())
    }

    async fn execute(&self, context: &IntelContext) -> Result<IntelOutput> {
        let doc_text = context
            .extra("document_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let doc_name = context
            .extra("document_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let prompt = format!(
            "Summarize this document in 3-5 sentences. \
             Extract 2-3 key topics as a JSON array. \
             Rate relevance from 0.0-1.0 for a coding agent.\n\n\
             Document: {doc_name}\n\n{doc_text}"
        );

        let raw = execute_intel_text_from_user_content(
            &context.client,
            &self.profile,
            prompt,
        )
        .await?;

        Ok(IntelOutput::success(
            self.name(),
            serde_json::json!({
                "summary": raw,
                "key_topics": vec!["<extract from model>"],
                "estimated_relevance": 0.5,
            }),
            0.8,
        ))
    }

    fn post_flight(&self, output: &IntelOutput) -> Result<()> {
        if output.get_str("summary").unwrap_or("").trim().is_empty() {
            return Err(anyhow::anyhow!("Empty document summary"));
        }
        Ok(())
    }

    fn fallback(&self, context: &IntelContext, error: &str) -> Result<IntelOutput> {
        trace_fallback(self.name(), error);

        let doc_text = context
            .extra("document_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let summary: String = doc_text.chars().take(500).collect();

        Ok(IntelOutput::fallback(
            self.name(),
            serde_json::json!({
                "summary": summary,
                "key_topics": vec!["<fallback>"],
                "estimated_relevance": 0.1,
            }),
            &format!("document summarizer failed: {}", error),
        ))
    }
}
