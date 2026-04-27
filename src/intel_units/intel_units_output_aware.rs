//! @efficiency-role: domain-logic
//!
//! Output-Aware Intel Unit — generates creative plain-text summaries
//! for large command output. Called AFTER execution when output exceeds threshold.

use crate::intel_trait::*;
use crate::*;
use serde::{Deserialize, Serialize};

pub(crate) struct OutputAwareUnit {
    profile: Profile,
}

impl OutputAwareUnit {
    pub fn new(profile: Profile) -> Self {
        Self { profile }
    }

    /// Generate a plain-text creative summary of large command output.
    /// Called only when output exceeds the line threshold.
    /// Returns plain text — no JSON parsing risk.
    pub(crate) async fn summarize(
        &self,
        client: &reqwest::Client,
        chat_url: &Url,
        user_message: &str,
        objective: &str,
        purpose: &str,
        cmd: &str,
        output: &str,
        artifact_path: Option<&str>,
    ) -> Result<String> {
        let narrative = format!(
            r##"USER REQUEST:
{user_message}

OBJECTIVE:
{objective}

STEP PURPOSE:
{purpose}

COMMAND EXECUTED:
{cmd}

COMMAND OUTPUT (first 3000 chars, truncated):
{truncated_output}

The full output ({output_lines} lines) was saved as an artifact{artifact_note}.

Create a concise, friendly summary of what the command found.
Focus on what the user actually asked for, not every detail.
Use plain text — no JSON, no code fences, no markup.
Keep it to 2-3 short paragraphs maximum.
If the output contains errors or warnings, mention them plainly."##,
            user_message = user_message.trim(),
            objective = objective.trim(),
            purpose = purpose.trim(),
            cmd = cmd.trim(),
            truncated_output = truncate_for_context(output, 3000),
            output_lines = output.lines().count(),
            artifact_note = artifact_path
                .map(|p| format!(" at: {}", p))
                .unwrap_or_else(|| "".to_string()),
        );

        let req = ChatCompletionRequest {
            model: self.profile.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: self.profile.system_prompt.clone(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    summarized: false,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: narrative,
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                    summarized: false,
                },
            ],
            temperature: self.profile.temperature,
            top_p: self.profile.top_p,
            stream: false,
            max_tokens: self.profile.max_tokens,
            n_probs: None,
            repeat_penalty: Some(self.profile.repeat_penalty),
            reasoning_format: Some(self.profile.reasoning_format.clone()),
            grammar: None,
        };

        let resp = crate::ui_chat::chat_once_with_timeout(
            client,
            chat_url,
            &req,
            self.profile.timeout_s.min(45),
        )
        .await?;

        let text = crate::extract_response_text(&resp);
        Ok(text.trim().to_string())
    }
}

fn truncate_for_context(text: &str, max_bytes: usize) -> String {
    let bytes = text.as_bytes();
    if bytes.len() <= max_bytes {
        return text.to_string();
    }
    let truncate_at = text
        .char_indices()
        .take_while(|(idx, _)| *idx < max_bytes)
        .last()
        .map(|(idx, c)| idx + c.len_utf8())
        .unwrap_or(max_bytes.min(text.len()));
    let mut result = text[..truncate_at].to_string();
    result.push_str("\n... [output truncated]");
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_for_context_short() {
        let input = "short output";
        let result = truncate_for_context(input, 100);
        assert_eq!(result, "short output");
    }

    #[test]
    fn test_truncate_for_context_long() {
        let input = "a".repeat(5000);
        let result = truncate_for_context(&input, 3000);
        assert!(result.len() < 5000);
        assert!(result.ends_with("[output truncated]"));
    }
}
