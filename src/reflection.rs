//! @efficiency-role: domain-logic
//!
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
    objective: &str, // Rephrased objective for clarity
) -> Result<ProgramReflection> {
    let prompt = build_reflection_prompt(program, priors, workspace, objective);

    let messages = vec![
        ChatMessage::simple("system", &cfg.system_prompt.clone()),
        ChatMessage::simple("user", &prompt),
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
    tools: None,
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
    objective: &str, // Rephrased objective
) -> String {
    let mut prompt = String::new();

    // Add classification context (soft guidance, not rules)
    prompt.push_str("Evaluate success rate of the proposed solution: 0.0 to 1.0\n\n");

    // Classification features as context (Task 007: Decouple Classification From Execution)
    prompt.push_str("**Classification Context (use as evidence, not rules):**\n");
    if let Some((speech_act, speech_p)) = priors.speech_act_probs.first() {
        prompt.push_str(&format!(
            "- Speech act: {} ({:.0}%)\n",
            speech_act,
            speech_p * 100.0
        ));
    }
    if let Some((route, route_p)) = priors.route_probs.first() {
        prompt.push_str(&format!("- Route: {} ({:.0}%)\n", route, route_p * 100.0));
    }
    prompt.push_str(&format!("- Entropy: {:.2} (", priors.entropy));
    if priors.entropy > 0.8 {
        prompt.push_str("HIGH UNCERTAINTY - classifier is unsure)\n");
    } else if priors.entropy > 0.5 {
        prompt.push_str("moderate uncertainty)\n");
    } else {
        prompt.push_str("low uncertainty - classifier is confident)\n");
    }

    // Check for close calls
    if priors.route_probs.len() >= 2 {
        let margin = priors.route_probs[0].1 - priors.route_probs[1].1;
        prompt.push_str(&format!("- Route margin: {:.2} (", margin));
        if margin < 0.2 {
            prompt.push_str("CLOSE CALL - top routes are similar)\n");
        } else {
            prompt.push_str("clear preference)\n");
        }
    }
    prompt.push('\n');

    prompt.push_str(&format!("**Rephrased Intention:** {}\n\n", objective));
    prompt.push_str("**Proposed Solution:**\n");
    for (i, step) in program.steps.iter().enumerate() {
        prompt.push_str(&format!(
            "{}. {} ({}): {}\n",
            i + 1,
            step.id(),
            step.kind(),
            step.purpose()
        ));
    }
    prompt.push('\n');
    prompt.push_str("Output: ALWAYS return JSON format\n");
    prompt.push_str("{\n");
    prompt.push_str("  \"confidence\": <0.0 to 1.0>,\n");
    prompt.push_str("  \"justification\": \"<brief explanation>\"\n");
    prompt.push_str("}\n\n");
    prompt.push_str("Rules:\n");
    prompt.push_str("- Be honest and critical\n");
    prompt.push_str("- Justification explains WHY you gave this confidence score\n");
    prompt.push_str("- If classification is uncertain (high entropy, low margin), mention this in your concerns\n");
    prompt.push_str(
        "- If the program doesn't match the top classification but makes sense, say so\n",
    );
    prompt.push_str(
        "- If confidence < 0.51: Orchestrator will use your justification to improve the plan\n",
    );
    prompt.push_str("- If confidence >= 0.51: Justification is logged for session trace\n");

    prompt
}

/// Parse the reflection response from the model
fn parse_reflection_response(response: &str) -> Result<ProgramReflection> {
    // Try to extract JSON from the response first
    if let Some(json_str) = extract_first_json_object(response) {
        // Parse the JSON
        let value: serde_json::Value = parse_json_loose(json_str)?;

        let confidence_score = value
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let justification = value
            .get("justification")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Justification goes into concerns array
        let concerns = if !justification.is_empty() {
            vec![justification.to_string()]
        } else {
            vec![]
        };

        let is_confident = confidence_score >= 0.51;

        return Ok(ProgramReflection {
            is_confident,
            confidence_score,
            concerns,
            missing_points: vec![],
            suggested_changes: vec![],
        });
    }

    // Fallback: Try to parse structured prose from weak models
    // Look for patterns like "confidence: 0.85" or "0.85"
    parse_reflection_prose(response)
}

/// Fallback parser for reflection responses that are in prose format
/// This handles cases where weak models output numbered lists instead of JSON
fn parse_reflection_prose(response: &str) -> Result<ProgramReflection> {
    let mut is_confident = false;
    let mut confidence_score = 0.5;
    let mut concerns = Vec::new();
    let mut missing_points = Vec::new();
    let mut suggested_changes = Vec::new();

    // Try to extract confidence score from prose
    // Look for patterns like "0.85", "85%", "very confident", "no confidence"
    let response_lower = response.to_lowercase();
    if response_lower.contains("very confident") || response_lower.contains("am confident") {
        confidence_score = 0.85;
        is_confident = true;
    } else if response_lower.contains("no confidence") || response_lower.contains("not confident") {
        confidence_score = 0.3;
        is_confident = false;
    } else if response_lower.contains("moderate") || response_lower.contains("somewhat") {
        confidence_score = 0.6;
        is_confident = false;
    }

    // Try to find numeric confidence
    for word in response.split_whitespace() {
        if let Ok(num) = word
            .trim_matches(|c: char| !c.is_numeric() && c != '.')
            .parse::<f64>()
        {
            if num > 0.0 && num <= 1.0 {
                confidence_score = num;
                is_confident = num >= 0.7;
                break;
            } else if num > 1.0 && num <= 100.0 {
                confidence_score = num / 100.0;
                is_confident = confidence_score >= 0.7;
                break;
            }
        }
    }

    // Extract concerns from "What Could Go Wrong" or similar sections
    if let Some(section) = extract_section(
        response,
        &["what could go wrong", "potential issues", "risks"],
    ) {
        concerns = split_prose_points(&section);
    }

    // Extract missing points from "What's Missing" section
    if let Some(section) = extract_section(
        response,
        &["what's missing", "what is missing", "missing steps"],
    ) {
        missing_points = split_prose_points(&section);
    }

    // Extract suggested changes from "Suggested Changes" section
    if let Some(section) = extract_section(
        response,
        &["suggested changes", "specific changes", "improvements"],
    ) {
        suggested_changes = split_prose_points(&section);
    }

    // If we found any content, consider it a success
    if !concerns.is_empty() || !missing_points.is_empty() || !suggested_changes.is_empty() {
        return Ok(ProgramReflection {
            is_confident,
            confidence_score,
            concerns,
            missing_points,
            suggested_changes,
        });
    }

    // Ultimate fallback: return a default reflection
    Ok(ProgramReflection {
        is_confident: true,
        confidence_score: 0.8,
        concerns: vec!["Model output was not in expected JSON format".to_string()],
        missing_points: Vec::new(),
        suggested_changes: Vec::new(),
    })
}

/// Extract a section from prose text based on section headers
fn extract_section(text: &str, headers: &[&str]) -> Option<String> {
    let text_lower = text.to_lowercase();

    for &header in headers {
        if let Some(start) = text_lower.find(header) {
            // Find the start of the actual content (after the header and any punctuation)
            let content_start = start + header.len();

            // Find the next section header or end of text
            let next_headers = headers
                .iter()
                .filter_map(|&h| {
                    text_lower[content_start..]
                        .find(h)
                        .map(|pos| content_start + pos)
                })
                .min();

            let content_end = next_headers.unwrap_or(text.len());

            let section = text[content_start..content_end].trim();
            if !section.is_empty() {
                return Some(section.to_string());
            }
        }
    }

    None
}

/// Split prose text into individual points (by newlines, numbered lists, or sentences)
fn split_prose_points(text: &str) -> Vec<String> {
    text.split('\n')
        .filter(|line| {
            let trimmed = line.trim();
            // Skip empty lines and lines that look like headers
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("```")
                && trimmed.len() > 10
        })
        .map(|line| {
            // Remove numbering like "1. ", "2. ", etc.
            let trimmed = line.trim();
            if let Some(pos) = trimmed.find(". ") {
                if pos < 5 && trimmed[..pos].chars().all(|c| c.is_numeric()) {
                    return trimmed[pos + 2..].trim().to_string();
                }
            }
            trimmed.to_string()
        })
        .filter(|s| !s.is_empty())
        .collect()
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
        critical_keywords
            .iter()
            .any(|kw| concern_lower.contains(kw))
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

    #[test]
    fn test_parse_reflection_prose() {
        // Test with actual problematic response from session s_1774823259_583791000
        let prose = r#"Here's a critical reflection of the proposed program:

1. **Confidence Check:** I am very confident in this program. The classification priors are very low, which suggests that the classifier is over-confident.

2. **What Could Go Wrong:** There are several potential issues with this program. Firstly, the classification entropy is very low, which means that the classifier is over-confident. Secondly, the priors are very low.

3. **What's Missing:** There are several steps that should be added to this program. Firstly, the shell steps should be verified before they are executed. Secondly, the edit steps should have verification before they are executed.

4. **Prior Constraints:** The classification priors are very low, which means that the classifier is over-confident. This could lead to incorrect classifications.

5. **Suggested Changes:** There are several specific changes that could be made to improve this program. Firstly, the shell steps should be verified before they are executed. Secondly, the edit steps should have verification before they are executed."#;

        let result = parse_reflection_prose(prose);
        assert!(result.is_ok());
        let reflection = result.unwrap();

        // Should detect "very confident"
        assert!(reflection.is_confident);
        assert!(reflection.confidence_score >= 0.8);

        // Should extract some concerns
        assert!(!reflection.concerns.is_empty());

        // Should extract missing points
        assert!(!reflection.missing_points.is_empty());

        // Should extract suggested changes
        assert!(!reflection.suggested_changes.is_empty());
    }

    #[test]
    fn test_parse_reflection_prose_with_percentage() {
        let prose = r#"Confidence: 85%

What could go wrong:
- The model might hallucinate
- Missing edge cases

Suggested changes:
- Add more tests
- Improve error handling"#;

        let result = parse_reflection_prose(prose);
        assert!(result.is_ok());
        let reflection = result.unwrap();

        // Should parse 85% as 0.85
        assert!(reflection.confidence_score >= 0.8);
        assert!(reflection.is_confident);
    }
}
