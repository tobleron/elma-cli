//! @efficiency-role: domain-logic
//!
//! Program Refinement Module
//!
//! Provides iterative program refinement based on execution feedback.
//! This enables Elma to autonomously revise plans when objectives are not achieved.

use crate::*;

/// Context for program refinement
#[derive(Debug, Clone)]
pub struct RefinementContext {
    /// Original user objective
    pub original_objective: String,
    /// Results from previous execution attempts
    pub step_results: Vec<StepResult>,
    /// Current program being refined
    pub current_program: Program,
    /// Iteration number (0-based)
    pub iteration: u32,
    /// Reason for refinement (e.g., "incomplete", "failed", "user_feedback")
    pub refinement_reason: String,
}

/// Result of objective achievement check
#[derive(Debug, Clone)]
pub struct ObjectiveAchievement {
    /// Whether the objective appears to be achieved
    pub is_achieved: bool,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
    /// Evidence supporting the assessment
    pub evidence: Vec<String>,
    /// Missing or incomplete aspects
    pub gaps: Vec<String>,
}

/// Check if the objective has been achieved based on step results
pub fn check_objective_achievement(
    objective: &str,
    step_results: &[StepResult],
) -> ObjectiveAchievement {
    let mut evidence = Vec::new();
    let mut gaps = Vec::new();

    // Check if any steps failed
    let failed_steps: Vec<&StepResult> = step_results.iter().filter(|s| !s.ok).collect();
    let successful_steps: Vec<&StepResult> = step_results.iter().filter(|s| s.ok).collect();

    if failed_steps.is_empty() && successful_steps.is_empty() {
        // No execution yet
        return ObjectiveAchievement {
            is_achieved: false,
            confidence: 0.0,
            evidence: vec!["No steps executed yet".to_string()],
            gaps: vec!["No execution results to evaluate".to_string()],
        };
    }

    // Check for successful non-reply steps (actual work done)
    let work_steps: Vec<&StepResult> = successful_steps
        .iter()
        .filter(|s| !s.kind.eq_ignore_ascii_case("reply"))
        .copied()
        .collect();

    if work_steps.is_empty() {
        gaps.push("No substantive work steps completed".to_string());
    } else {
        for step in &work_steps {
            evidence.push(format!("Completed: {} ({})", step.id, step.kind));
        }
    }

    // Check for failures
    for step in &failed_steps {
        gaps.push(format!(
            "Failed: {} ({}) - {}",
            step.id,
            step.kind,
            step.outcome_reason.as_deref().unwrap_or("unknown reason")
        ));
    }

    // Check if there's a final reply with content
    let has_final_reply = step_results
        .iter()
        .any(|s| s.kind.eq_ignore_ascii_case("reply") && !s.summary.trim().is_empty());

    if has_final_reply {
        evidence.push("Final reply generated".to_string());
    } else {
        gaps.push("No final reply generated".to_string());
    }

    // Calculate confidence based on success rate and completion
    let total_work_steps = work_steps.len() + failed_steps.len();
    let success_rate = if total_work_steps > 0 {
        work_steps.len() as f64 / total_work_steps as f64
    } else {
        0.0
    };

    let confidence = if failed_steps.is_empty() && has_final_reply {
        0.8 + (success_rate * 0.2) // High confidence if no failures
    } else if success_rate > 0.5 {
        0.5 + (success_rate * 0.3) // Medium confidence
    } else {
        success_rate * 0.5 // Low confidence
    };

    // Objective is achieved if:
    // 1. No failures
    // 2. At least one work step completed
    // 3. Final reply exists
    let is_achieved =
        failed_steps.is_empty() && !work_steps.is_empty() && has_final_reply && confidence >= 0.7;

    ObjectiveAchievement {
        is_achieved,
        confidence,
        evidence,
        gaps,
    }
}

/// Build a refinement prompt for the model
pub fn build_refinement_prompt(
    context: &RefinementContext,
    achievement: &ObjectiveAchievement,
) -> String {
    let mut prompt = String::new();

    prompt.push_str("## Program Refinement Request\n\n");
    prompt.push_str(&format!("**Iteration:** {}\n\n", context.iteration + 1));
    prompt.push_str(&format!(
        "**Original Objective:** {}\n\n",
        context.original_objective
    ));

    prompt.push_str("## Current Status\n\n");
    prompt.push_str(&format!(
        "**Objective Achievement:** {} (confidence: {:.1}%)\n\n",
        if achievement.is_achieved {
            "ACHIEVED"
        } else {
            "NOT ACHIEVED"
        },
        achievement.confidence * 100.0
    ));

    if !achievement.evidence.is_empty() {
        prompt.push_str("**Completed:**\n");
        for item in &achievement.evidence {
            prompt.push_str(&format!("- {}\n", item));
        }
        prompt.push('\n');
    }

    if !achievement.gaps.is_empty() {
        prompt.push_str("**Issues/Gaps:**\n");
        for item in &achievement.gaps {
            prompt.push_str(&format!("- {}\n", item));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Refinement Reason\n\n");
    prompt.push_str(&format!("{}\n\n", context.refinement_reason));

    prompt.push_str("## Current Program Structure\n\n");
    prompt.push_str(&format!(
        "**Objective:** {}\n\n",
        context.current_program.objective
    ));
    prompt.push_str("**Current Steps:**\n");
    for (i, step) in context.current_program.steps.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. {} ({}) - purpose: {}\n",
            i + 1,
            step.id(),
            step.kind(),
            step.purpose()
        ));
    }
    prompt.push('\n');

    prompt.push_str("## Step Execution Results\n\n");
    for (i, result) in context.step_results.iter().enumerate() {
        prompt.push_str(&format!(
            "**Step {}** ({}): {}\n",
            i + 1,
            result.id,
            result.kind
        ));
        prompt.push_str(&format!(
            "- Status: {}\n",
            if result.ok { "OK" } else { "FAILED" }
        ));
        if !result.summary.is_empty() {
            prompt.push_str(&format!(
                "- Summary: {}\n",
                truncate_text(&result.summary, 200)
            ));
        }
        if let Some(ref reason) = result.outcome_reason {
            prompt.push_str(&format!("- Reason: {}\n", truncate_text(reason, 200)));
        }
        prompt.push('\n');
    }

    prompt.push_str("## Instructions\n\n");
    prompt.push_str("Your task is to revise the program to better achieve the objective.\n\n");
    prompt.push_str("**Consider:**\n");
    prompt.push_str("1. What went wrong or is incomplete?\n");
    prompt.push_str("2. What steps need to be added, removed, or modified?\n");
    prompt.push_str("3. Are there missing verification or follow-up steps?\n");
    prompt.push_str("4. Is the objective still appropriate, or does it need refinement?\n\n");

    prompt.push_str("**Output:**\n");
    prompt.push_str("Return ONLY one valid JSON object representing the revised program.\n");
    prompt.push_str("The program must have the same structure as the original.\n\n");

    prompt
}

/// Request program refinement from the model
pub async fn refine_program(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    context: &RefinementContext,
    achievement: &ObjectiveAchievement,
) -> Result<Program> {
    let prompt = build_refinement_prompt(context, achievement);

    let messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: cfg.system_prompt.clone(),
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt,
        },
    ];

    let request = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages,
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };

    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);

    // Parse the response as a Program
    // Use extract_first_json_object to handle models that wrap JSON in markdown or add prose
    let json_str =
        crate::routing::extract_first_json_object(&response_text).unwrap_or(&response_text);
    parse_json_loose(json_str).context("Failed to parse refined program from model response")
}

/// Truncate text to maximum length
fn truncate_text(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_achieved_objective() {
        let step_results = vec![
            StepResult {
                id: "s1".to_string(),
                kind: "shell".to_string(),
                ok: true,
                summary: "File contents captured".to_string(),
                ..StepResult::default()
            },
            StepResult {
                id: "r1".to_string(),
                kind: "reply".to_string(),
                ok: true,
                summary: "Summary provided to user".to_string(),
                ..StepResult::default()
            },
        ];

        let achievement = check_objective_achievement("test objective", &step_results);
        assert!(achievement.is_achieved);
        assert!(achievement.confidence > 0.7);
    }

    #[test]
    fn detects_failed_objective() {
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            ok: false,
            summary: "".to_string(),
            outcome_reason: Some("Command failed".to_string()),
            ..StepResult::default()
        }];

        let achievement = check_objective_achievement("test objective", &step_results);
        assert!(!achievement.is_achieved);
        assert!(achievement.gaps.iter().any(|g| g.contains("Failed")));
    }

    #[test]
    fn detects_incomplete_objective() {
        let step_results = vec![StepResult {
            id: "s1".to_string(),
            kind: "shell".to_string(),
            ok: true,
            summary: "Partial work done".to_string(),
            ..StepResult::default()
        }];

        let achievement = check_objective_achievement("test objective", &step_results);
        assert!(!achievement.is_achieved);
        assert!(achievement.gaps.iter().any(|g| g.contains("reply")));
    }
}
