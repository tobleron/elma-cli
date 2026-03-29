//! Pre-Execution Reflection Module
//!
//! This module provides pre-execution reflection capabilities,
//! allowing Elma to catch flawed plans before they waste execution time.
//!
//! DESIGN RATIONALE:
//! The model can reason about contradictions (seen in session analysis),
//! but only AFTER producing output. Pre-execution reflection catches
//! issues earlier, saving time and improving reliability.

use crate::*;

/// Result of pre-execution reflection
#[derive(Debug, Clone)]
pub struct ProgramReflection {
    /// Whether the model is confident in the program
    pub is_confident: bool,
    /// Identified concerns or issues
    pub concerns: Vec<String>,
    /// Missing steps or considerations
    pub missing_points: Vec<String>,
    /// Suggested changes
    pub suggested_changes: Vec<String>,
    /// Confidence score (0.0 to 1.0)
    pub confidence_score: f64,
}

/// Reflect on a program before execution
///
/// This asks the model to critique its own program:
/// 1. Are you confident this will achieve the objective?
/// 2. What could go wrong?
/// 3. What's missing?
/// 4. Do the priors constrain you inappropriately?
pub async fn reflect_on_program(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    program: &Program,
    priors: &ClassificationFeatures,
    workspace: &str,
) -> Result<ProgramReflection> {
    let prompt = build_reflection_prompt(program, priors, workspace);
    
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
    };
    
    let response = chat_once(client, chat_url, &request).await?;
    let response_text = extract_response_text(&response);
    
    // Parse the reflection response
    parse_reflection_response(&response_text)
}

/// Build the reflection prompt
fn build_reflection_prompt(
    program: &Program,
    priors: &ClassificationFeatures,
    workspace: &str,
) -> String {
    let mut prompt = String::new();
    
    prompt.push_str("## Pre-Execution Reflection\n\n");
    prompt.push_str("Critique this program BEFORE execution. Be honest and critical.\n\n");
    
    prompt.push_str("### Proposed Program\n\n");
    prompt.push_str(&format!("**Objective:** {}\n\n", program.objective));
    prompt.push_str("**Steps:**\n");
    for (i, step) in program.steps.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. {} ({}) - purpose: {}\n",
            i + 1,
            step.id(),
            step.kind(),
            step.purpose()
        ));
    }
    prompt.push('\n');
    
    prompt.push_str("### Classification Priors (SOFT EVIDENCE)\n\n");
    prompt.push_str(&format!(
        "**Route Probabilities:** {}\n",
        format_route_distribution(&priors.route_probs)
    ));
    prompt.push_str(&format!(
        "**Speech Act Probabilities:** {}\n",
        format_route_distribution(&priors.speech_act_probs)
    ));
    prompt.push_str(&format!(
        "**Classification Entropy:** {:.2}\n\n",
        priors.entropy
    ));
    
    if priors.entropy < 0.1 {
        prompt.push_str("⚠️ **WARNING:** Classification entropy is very low. ");
        prompt.push_str("The classifier is over-confident. ");
        prompt.push_str("Consider whether the priors are constraining you inappropriately.\n\n");
    }
    
    prompt.push_str("### Workspace Context\n\n");
    let workspace_preview = if workspace.len() > 500 {
        &workspace[..500]
    } else {
        workspace
    };
    prompt.push_str(workspace_preview);
    if workspace.len() > 500 {
        prompt.push_str("\n...(truncated)");
    }
    prompt.push_str("\n\n");
    
    prompt.push_str("### Reflection Questions\n\n");
    prompt.push_str("Answer each question honestly:\n\n");
    prompt.push_str("1. **Confidence Check:** Are you confident this program will achieve the objective? ");
    prompt.push_str("Rate your confidence from 0.0 (no confidence) to 1.0 (very confident).\n\n");
    
    prompt.push_str("2. **What Could Go Wrong:** Identify potential issues, risks, or failure modes. ");
    prompt.push_str("What assumptions are you making that might be wrong?\n\n");
    
    prompt.push_str("3. **What's Missing:** Are there any steps that should be added? ");
    prompt.push_str("Any verification, error handling, or edge cases not covered?\n\n");
    
    prompt.push_str("4. **Prior Constraints:** Do the classification priors (route, speech act, etc.) ");
    prompt.push_str("constrain you inappropriately? Should you override them based on the actual user request?\n\n");
    
    prompt.push_str("5. **Suggested Changes:** What specific changes would you make to improve this program?\n\n");
    
    prompt.push_str("### Output Format\n\n");
    prompt.push_str("Return your reflection in this JSON format:\n");
    prompt.push_str("```\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"is_confident\": true,\n");
    prompt.push_str("  \"confidence_score\": 0.85,\n");
    prompt.push_str("  \"concerns\": [\"concern 1\", \"concern 2\"],\n");
    prompt.push_str("  \"missing_points\": [\"missing step 1\"],\n");
    prompt.push_str("  \"suggested_changes\": [\"change 1\"]\n");
    prompt.push_str("}\n");
    prompt.push_str("```\n\n");
    
    prompt.push_str("Be critical. It's better to identify issues now than waste execution time.\n");
    
    prompt
}

/// Parse the reflection response from the model
fn parse_reflection_response(response: &str) -> Result<ProgramReflection> {
    // Try to extract JSON from the response
    let json_str = extract_first_json_object(response)
        .ok_or_else(|| anyhow::anyhow!("No JSON object found in reflection response"))?;
    
    // Parse the JSON
    let value: serde_json::Value = parse_json_loose(json_str)?;
    
    let is_confident = value
        .get("is_confident")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    
    let confidence_score = value
        .get("confidence_score")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5);
    
    let concerns = value
        .get("concerns")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    
    let missing_points = value
        .get("missing_points")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    
    let suggested_changes = value
        .get("suggested_changes")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    
    Ok(ProgramReflection {
        is_confident,
        confidence_score,
        concerns,
        missing_points,
        suggested_changes,
    })
}

/// Decide whether to proceed with execution based on reflection
pub fn should_proceed_with_execution(reflection: &ProgramReflection) -> bool {
    // Proceed if:
    // 1. Model is confident, OR
    // 2. Confidence score is above threshold, AND
    // 3. No critical concerns identified
    
    const CONFIDENCE_THRESHOLD: f64 = 0.6;
    
    if reflection.is_confident && reflection.confidence_score >= CONFIDENCE_THRESHOLD {
        return true;
    }
    
    // Don't proceed if there are critical concerns
    let critical_keywords = ["cannot", "impossible", "missing evidence", "wrong route"];
    let has_critical_concern = reflection.concerns.iter().any(|concern| {
        let concern_lower = concern.to_lowercase();
        critical_keywords.iter().any(|kw| concern_lower.contains(kw))
    });
    
    if has_critical_concern {
        return false;
    }
    
    // Proceed with caution if confidence is moderate
    reflection.confidence_score >= CONFIDENCE_THRESHOLD - 0.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_proceed_confident_program() {
        let reflection = ProgramReflection {
            is_confident: true,
            confidence_score: 0.85,
            concerns: vec![],
            missing_points: vec![],
            suggested_changes: vec![],
        };
        assert!(should_proceed_with_execution(&reflection));
    }

    #[test]
    fn test_should_not_proceed_critical_concerns() {
        let reflection = ProgramReflection {
            is_confident: false,
            confidence_score: 0.4,
            concerns: vec!["Cannot execute without workspace evidence".to_string()],
            missing_points: vec!["Missing file inspection step".to_string()],
            suggested_changes: vec!["Add file inspection first".to_string()],
        };
        assert!(!should_proceed_with_execution(&reflection));
    }

    #[test]
    fn test_should_proceed_moderate_confidence() {
        let reflection = ProgramReflection {
            is_confident: false,
            confidence_score: 0.65,
            concerns: vec!["Could add verification step".to_string()],
            missing_points: vec![],
            suggested_changes: vec![],
        };
        assert!(should_proceed_with_execution(&reflection));
    }

    #[test]
    fn test_should_not_proceed_low_confidence() {
        let reflection = ProgramReflection {
            is_confident: false,
            confidence_score: 0.3,
            concerns: vec![],
            missing_points: vec![],
            suggested_changes: vec![],
        };
        assert!(!should_proceed_with_execution(&reflection));
    }
}
