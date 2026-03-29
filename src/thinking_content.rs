//! Thinking Content Extraction and Processing
//!
//! This module provides professional-grade thinking/reasoning content extraction
//! from thinking models (DeepSeek R1, Qwen3.5 thinking modes, etc.).
//!
//! It supports multiple thinking content formats:
//! 1. llama.cpp sentinel markers: `<<<reasoning_content_start>>>...<<<reasoning_content_end>>>`
//! 2. XML think tags: `<think>...</think>`
//! 3. OpenAI/llama.cpp `reasoning_content` field
//! 4. Other common thinking markers (`<thinking>`, `<thought>`, etc.)
//!
//! Architecture follows llama.cpp WebUI patterns for compatibility.

use serde::{Deserialize, Serialize};

/// llama.cpp sentinel markers (matches llama.cpp WebUI parseReasoningContent)
const LLAMA_REASONING_START: &str = "<<<reasoning_content_start>>>";
const LLAMA_REASONING_END: &str = "<<<reasoning_content_end>>>";

/// XML think tag markers
const THINK_OPEN: &str = "<think>";
const THINK_CLOSE: &str = "</think>";

/// Alternative thinking tag markers (less common)
const THINKING_OPEN: &str = "<thinking>";
const THINKING_CLOSE: &str = "</thinking>";
const THOUGHT_OPEN: &str = "<thought>";
const THOUGHT_CLOSE: &str = "</thought>";
const REASONING_OPEN: &str = "<reasoning>";
const REASONING_CLOSE: &str = "</reasoning>";

/// Result of thinking content extraction
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThinkingExtraction {
    /// Extracted thinking/reasoning content (if any)
    pub thinking: Option<String>,
    /// Final answer content (thinking removed)
    pub final_answer: String,
    /// Whether thinking markers were detected
    pub has_thinking_markers: bool,
    /// Source of thinking extraction
    pub source: ThinkingSource,
}

/// Source of thinking content extraction
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum ThinkingSource {
    /// No thinking detected
    #[default]
    None,
    /// Extracted from llama.cpp sentinel markers
    LlamaSentinel,
    /// Extracted from <think></think> tags
    ThinkTag,
    /// Extracted from <thinking> tags
    ThinkingTag,
    /// Extracted from <thought> tags
    ThoughtTag,
    /// Extracted from <reasoning> tags
    ReasoningTag,
    /// Extracted from reasoning_content field
    ReasoningField,
    /// Combined: field + content markers
    Combined,
}

/// Extract thinking content from llama.cpp sentinel markers
///
/// Mirrors llama.cpp WebUI parseReasoningContent():
/// - Splits content into plain + reasoning parts based on sentinel markers
/// - Unterminated reasoning marker consumes rest of content
pub fn extract_llama_sentinel_reasoning(content: &str) -> (String, Option<String>) {
    let mut plain_parts: Vec<&str> = Vec::new();
    let mut reasoning_parts: Vec<&str> = Vec::new();

    let mut cursor = 0usize;
    while cursor < content.len() {
        // Find next reasoning start marker
        let Some(start_idx_rel) = content[cursor..].find(LLAMA_REASONING_START) else {
            // No more markers - rest is plain content
            plain_parts.push(&content[cursor..]);
            break;
        };
        let start_idx = cursor + start_idx_rel;
        
        // Content before marker is plain
        plain_parts.push(&content[cursor..start_idx]);

        // Extract reasoning content
        let reasoning_start = start_idx + LLAMA_REASONING_START.len();
        if reasoning_start >= content.len() {
            break;
        }

        // Find closing marker
        let Some(end_idx_rel) = content[reasoning_start..].find(LLAMA_REASONING_END) else {
            // Unterminated - rest is reasoning
            reasoning_parts.push(&content[reasoning_start..]);
            break;
        };
        let end_idx = reasoning_start + end_idx_rel;
        reasoning_parts.push(&content[reasoning_start..end_idx]);
        cursor = end_idx + LLAMA_REASONING_END.len();
    }

    let plain = plain_parts.join("");
    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join("\n\n"))
    };
    (plain, reasoning)
}

/// Extract thinking content from XML-style tags
fn extract_xml_thinking(content: &str, open: &str, close: &str) -> (String, Option<String>) {
    let mut thinking_parts: Vec<String> = Vec::new();
    let mut final_parts: Vec<&str> = Vec::new();
    let mut rest = content;

    while let Some(open_idx) = rest.find(open) {
        // Content before tag is final answer
        final_parts.push(&rest[..open_idx]);
        
        let after_open = &rest[open_idx + open.len()..];
        
        // Find closing tag
        if let Some(close_idx) = after_open.find(close) {
            // Complete tag - extract thinking
            let thinking = after_open[..close_idx].trim();
            if !thinking.is_empty() {
                thinking_parts.push(thinking.to_string());
            }
            rest = &after_open[close_idx + close.len()..];
        } else {
            // Unclosed tag - rest is thinking
            let thinking = after_open.trim();
            if !thinking.is_empty() {
                thinking_parts.push(thinking.to_string());
            }
            rest = "";
            break;
        }
    }
    
    // Remaining content is final answer
    final_parts.push(rest);
    
    let final_answer = final_parts.join("");
    let thinking = if thinking_parts.is_empty() {
        None
    } else {
        Some(thinking_parts.join("\n\n"))
    };
    
    (final_answer, thinking)
}

/// Extract thinking from reasoning_content field
fn extract_from_field(reasoning_content: Option<&str>) -> Option<String> {
    reasoning_content
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Main thinking extraction function
///
/// Extracts thinking content from multiple sources:
/// 1. reasoning_content field (from API response)
/// 2. llama.cpp sentinel markers in content
/// 3. XML think tags (<think></think>)
/// 4. Alternative thinking tags (<thinking>, <thought>, <reasoning>)
///
/// Returns separated thinking and final answer content.
pub fn extract_thinking(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> ThinkingExtraction {
    let content_str = content.unwrap_or("");
    let reasoning_field = extract_from_field(reasoning_content);
    
    // Check for llama.cpp sentinel markers first
    let (content_no_sentinel, sentinel_thinking) = if content_str.contains(LLAMA_REASONING_START) {
        let (plain, thinking) = extract_llama_sentinel_reasoning(content_str);
        (plain, thinking)
    } else {
        (content_str.to_string(), None)
    };
    
    // Check for <think></think> tags
    let (content_no_think, think_tag_thinking) = if content_no_sentinel.contains(THINK_OPEN) {
        extract_xml_thinking(&content_no_sentinel, THINK_OPEN, THINK_CLOSE)
    } else {
        (content_no_sentinel, None)
    };
    
    // Check for alternative thinking tags
    let (content_final, alt_thinking) = check_alternative_tags(&content_no_think);
    
    // Combine all thinking sources
    let mut all_thinking_parts: Vec<String> = Vec::new();
    let mut source = ThinkingSource::None;
    let mut has_markers = false;
    
    if let Some(t) = reasoning_field {
        all_thinking_parts.push(t);
        source = ThinkingSource::ReasoningField;
        has_markers = true;
    }
    if let Some(t) = sentinel_thinking {
        all_thinking_parts.push(t);
        source = if source == ThinkingSource::ReasoningField {
            ThinkingSource::Combined
        } else {
            ThinkingSource::LlamaSentinel
        };
        has_markers = true;
    }
    if let Some(t) = think_tag_thinking {
        all_thinking_parts.push(t);
        source = if has_markers {
            ThinkingSource::Combined
        } else {
            ThinkingSource::ThinkTag
        };
        has_markers = true;
    }
    if let Some(t) = alt_thinking {
        all_thinking_parts.push(t);
        if !has_markers {
            source = match () {
                _ if content_final.contains(THINKING_OPEN) => ThinkingSource::ThinkingTag,
                _ if content_final.contains(THOUGHT_OPEN) => ThinkingSource::ThoughtTag,
                _ if content_final.contains(REASONING_OPEN) => ThinkingSource::ReasoningTag,
                _ => ThinkingSource::None,
            };
        }
        has_markers = true;
    }
    
    let thinking = if all_thinking_parts.is_empty() {
        None
    } else {
        Some(all_thinking_parts.join("\n\n"))
    };
    
    let final_answer = strip_think_tags(&content_final);
    
    ThinkingExtraction {
        thinking,
        final_answer,
        has_thinking_markers: has_markers,
        source,
    }
}

/// Check for alternative thinking tags
fn check_alternative_tags(content: &str) -> (String, Option<String>) {
    // Check <thinking> tags
    if content.contains(THINKING_OPEN) {
        return extract_xml_thinking(content, THINKING_OPEN, THINKING_CLOSE);
    }
    // Check <thought> tags
    if content.contains(THOUGHT_OPEN) {
        return extract_xml_thinking(content, THOUGHT_OPEN, THOUGHT_CLOSE);
    }
    // Check <reasoning> tags
    if content.contains(REASONING_OPEN) {
        return extract_xml_thinking(content, REASONING_OPEN, REASONING_CLOSE);
    }
    (content.to_string(), None)
}

/// Strip think tags and their content from text
pub fn strip_think_tags(content: &str) -> String {
    let mut result = content.to_string();
    
    // Strip <think></think> tags
    result = strip_xml_tags(&result, THINK_OPEN, THINK_CLOSE);
    
    // Strip <thinking> tags
    result = strip_xml_tags(&result, THINKING_OPEN, THINKING_CLOSE);
    
    // Strip <thought> tags
    result = strip_xml_tags(&result, THOUGHT_OPEN, THOUGHT_CLOSE);
    
    // Strip <reasoning> tags
    result = strip_xml_tags(&result, REASONING_OPEN, REASONING_CLOSE);
    
    // Strip llama.cpp sentinels
    result = strip_llama_sentinels(&result);
    
    result.trim().to_string()
}

/// Strip XML-style tags and their content
fn strip_xml_tags(content: &str, open: &str, close: &str) -> String {
    let mut result = String::new();
    let mut rest = content;
    
    while let Some(open_idx) = rest.find(open) {
        result.push_str(&rest[..open_idx]);
        let after_open = &rest[open_idx + open.len()..];
        
        if let Some(close_idx) = after_open.find(close) {
            rest = &after_open[close_idx + close.len()..];
        } else {
            // Unclosed tag - skip rest
            rest = "";
            break;
        }
    }
    result.push_str(rest);
    result
}

/// Strip llama.cpp sentinel markers and content
fn strip_llama_sentinels(content: &str) -> String {
    let (plain, _) = extract_llama_sentinel_reasoning(content);
    plain
}

/// Detect if a model is a thinking model based on response characteristics
pub fn is_thinking_model(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> bool {
    // Check reasoning_content field
    if let Some(rc) = reasoning_content {
        if !rc.trim().is_empty() {
            return true;
        }
    }
    
    // Check content for thinking markers
    if let Some(c) = content {
        if c.contains(LLAMA_REASONING_START) {
            return true;
        }
        if c.contains(THINK_OPEN) {
            return true;
        }
        if c.contains(THINKING_OPEN) {
            return true;
        }
        if c.contains(THOUGHT_OPEN) {
            return true;
        }
        if c.contains(REASONING_OPEN) {
            return true;
        }
    }
    
    false
}

/// Get a description of the thinking source for logging
pub fn thinking_source_description(source: &ThinkingSource) -> &'static str {
    match source {
        ThinkingSource::None => "no thinking detected",
        ThinkingSource::LlamaSentinel => "llama.cpp sentinel markers",
        ThinkingSource::ThinkTag => "think tags",
        ThinkingSource::ThinkingTag => "thinking tags",
        ThinkingSource::ThoughtTag => "thought tags",
        ThinkingSource::ReasoningTag => "reasoning tags",
        ThinkingSource::ReasoningField => "reasoning_content field",
        ThinkingSource::Combined => "multiple sources (field + markers)",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_llama_sentinel_reasoning() {
        let content = "<<<reasoning_content_start>>>step by step thinking<<<reasoning_content_end>>>final answer";
        let (final_answer, thinking) = extract_llama_sentinel_reasoning(content);
        assert_eq!(final_answer, "final answer");
        assert_eq!(thinking, Some("step by step thinking".to_string()));
    }

    #[test]
    fn extracts_think_tag_reasoning() {
        let content = "<think>let me think about this</think>The answer is 42";
        let extraction = extract_thinking(Some(content), None);
        assert_eq!(extraction.thinking, Some("let me think about this".to_string()));
        assert_eq!(extraction.final_answer, "The answer is 42");
        assert!(extraction.has_thinking_markers);
    }

    #[test]
    fn extracts_from_reasoning_field() {
        let extraction = extract_thinking(Some("answer"), Some("thinking process"));
        assert_eq!(extraction.thinking, Some("thinking process".to_string()));
        assert_eq!(extraction.final_answer, "answer");
        assert_eq!(extraction.source, ThinkingSource::ReasoningField);
    }

    #[test]
    fn handles_no_thinking() {
        let extraction = extract_thinking(Some("plain answer"), None);
        assert!(extraction.thinking.is_none());
        assert_eq!(extraction.final_answer, "plain answer");
        assert!(!extraction.has_thinking_markers);
        assert_eq!(extraction.source, ThinkingSource::None);
    }

    #[test]
    fn handles_unterminated_think_tag() {
        let content = "<think>this is unclosed thinking";
        let extraction = extract_thinking(Some(content), None);
        assert_eq!(extraction.thinking, Some("this is unclosed thinking".to_string()));
        assert_eq!(extraction.final_answer, "");
    }

    #[test]
    fn strips_think_tags() {
        let content = "<think>thinking</think>answer";
        let stripped = strip_think_tags(content);
        assert_eq!(stripped, "answer");
    }

    #[test]
    fn detects_thinking_model() {
        assert!(is_thinking_model(Some("<think>think</think>answer"), None));
        assert!(is_thinking_model(Some("answer"), Some("thinking")));
        assert!(is_thinking_model(Some("<<<reasoning_content_start>>>think<<<reasoning_content_end>>>"), None));
        assert!(!is_thinking_model(Some("plain answer"), None));
    }

    #[test]
    fn handles_multiple_thinking_blocks() {
        let content = "<think>first thought</think>middle<think>second thought</think>end";
        let extraction = extract_thinking(Some(content), None);
        assert_eq!(extraction.thinking, Some("first thought\n\nsecond thought".to_string()));
        assert_eq!(extraction.final_answer, "middleend");
    }
}
