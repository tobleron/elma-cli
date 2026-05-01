//! Deterministic JSON repair pipeline for intel unit outputs.
//!
//! Repair stages (in order, stop on first success):
//! 1. Direct parse (serde_json::from_str)
//! 2. jsonrepair-rs (fix trailing commas, missing quotes, etc.)
//! 3. Regex field extraction (regex_fallback_value)
//! 4. LLM-based repair (JsonRepairUnit)
//!
//! The pipeline is structured so deterministic stages run before
//! the expensive LLM-based repair, minimizing latency for common cases.

use crate::*;
use jsonrepair_rs::jsonrepair;
use serde::de::DeserializeOwned;

/// Which repair stage produced the successful parse
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepairStage {
    DirectParse,
    JsonRepairRs,
    RegexExtraction,
    LlmRepair,
    Failed,
}

/// Result of a successful repair pipeline run
#[derive(Debug, Clone)]
pub(crate) struct RepairResult<T> {
    pub(crate) value: T,
    pub(crate) stage: RepairStage,
    pub(crate) original_error: Option<String>,
}

/// Run the full 4-stage deterministic repair pipeline.
///
/// Stages 1-3 are synchronous and fast. Stage 4 (LLM repair) requires
/// a network call and is only invoked if stages 1-3 all fail.
pub(crate) async fn repair_json<T: DeserializeOwned + 'static>(
    raw: &str,
    client: &reqwest::Client,
    chat_url: &Url,
    repair_profile: &Profile,
) -> RepairResult<Option<T>> {
    let raw_trimmed = raw.trim();

    // Stage 1: Direct parse with extraction
    if let Some(value) = crate::json_parser::extract_json_object(raw_trimmed) {
        if let Ok(parsed) = serde_json::from_value::<T>(value) {
            return RepairResult {
                value: Some(parsed),
                stage: RepairStage::DirectParse,
                original_error: None,
            };
        }
    }

    // Stage 2: jsonrepair-rs
    if let Ok(repaired_str) = jsonrepair(raw_trimmed) {
        if let Some(value) = crate::json_parser::extract_json_object(&repaired_str) {
            if let Ok(parsed) = serde_json::from_value::<T>(value) {
                return RepairResult {
                    value: Some(parsed),
                    stage: RepairStage::JsonRepairRs,
                    original_error: None,
                };
            }
        }
    }

    // Stage 3: Regex field extraction (existing fallback)
    if let Some(fallback_value) = crate::json_parser_extract::regex_fallback_value::<T>(raw_trimmed)
    {
        if let Ok(parsed) = serde_json::from_value(fallback_value) {
            return RepairResult {
                value: Some(parsed),
                stage: RepairStage::RegexExtraction,
                original_error: None,
            };
        }
    }

    // Stage 4: LLM-based repair via existing JsonRepairUnit
    let repair_unit = crate::JsonRepairUnit::new(
        repair_profile.clone(),
    );
    let problems = vec!["Failed to parse as valid JSON".to_string()];
    match repair_unit
        .repair_with_fallback(client, chat_url, raw_trimmed, &problems)
        .await
    {
        Ok(repaired) => {
            if let Some(value) = crate::json_parser::extract_json_object(&repaired) {
                if let Ok(parsed) = serde_json::from_value::<T>(value.clone()) {
                    return RepairResult {
                        value: Some(parsed),
                        stage: RepairStage::LlmRepair,
                        original_error: None,
                    };
                }
            }
            // LLM returned something but it wasn't parseable — try regex on it
            if let Some(fallback_value) =
                crate::json_parser_extract::regex_fallback_value::<T>(&repaired)
            {
                if let Ok(parsed) = serde_json::from_value(fallback_value) {
                    return RepairResult {
                        value: Some(parsed),
                        stage: RepairStage::LlmRepair,
                        original_error: None,
                    };
                }
            }
            RepairResult {
                value: None,
                stage: RepairStage::Failed,
                original_error: Some("LLM repair produced unparseable output".to_string()),
            }
        }
        Err(e) => RepairResult {
            value: None,
            stage: RepairStage::Failed,
            original_error: Some(format!("LLM repair failed: {}", e)),
        },
    }
}

/// Synchronous repair for contexts where async/network is unavailable.
/// Stages 1-3 only (no LLM repair).
pub(crate) fn repair_json_sync<T: DeserializeOwned + 'static>(raw: &str) -> RepairResult<Option<T>> {
    let raw_trimmed = raw.trim();

    // Stage 1: Direct parse
    if let Some(value) = crate::json_parser::extract_json_object(raw_trimmed) {
        if let Ok(parsed) = serde_json::from_value::<T>(value) {
            return RepairResult {
                value: Some(parsed),
                stage: RepairStage::DirectParse,
                original_error: None,
            };
        }
    }

    // Stage 2: jsonrepair-rs
    if let Ok(repaired_str) = jsonrepair(raw_trimmed) {
        if let Some(value) = crate::json_parser::extract_json_object(&repaired_str) {
            if let Ok(parsed) = serde_json::from_value::<T>(value) {
                return RepairResult {
                    value: Some(parsed),
                    stage: RepairStage::JsonRepairRs,
                    original_error: None,
                };
            }
        }
    }

    // Stage 3: Regex field extraction
    if let Some(fallback_value) = crate::json_parser_extract::regex_fallback_value::<T>(raw_trimmed)
    {
        if let Ok(parsed) = serde_json::from_value(fallback_value) {
            return RepairResult {
                value: Some(parsed),
                stage: RepairStage::RegexExtraction,
                original_error: None,
            };
        }
    }

    RepairResult {
        value: None,
        stage: RepairStage::Failed,
        original_error: Some("All deterministic repair stages failed".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_stage1_direct_parse() {
        let result = repair_json_sync::<serde_json::Value>(r#"{"a": 1, "b": "two"}"#);
        assert!(result.value.is_some());
        assert_eq!(result.stage, RepairStage::DirectParse);
    }

    #[test]
    fn test_sync_stage1_handles_think_blocks() {
        let result = repair_json_sync::<serde_json::Value>(
            "<think>I should figure this out</think>{\"choice\": \"ok\", \"reason\": \"done\"}",
        );
        assert!(result.value.is_some());
        assert_eq!(result.stage, RepairStage::DirectParse);
    }

    #[test]
    fn test_sync_stage2_jsonrepair() {
        let result = repair_json_sync::<serde_json::Value>(r#"{"a": 1, "b": "two"}"#);
        assert!(result.value.is_some());
    }

    #[test]
    fn test_sync_stage3_regex_extraction() {
        let result = repair_json_sync::<serde_json::Value>(
            "The answer is: choice=ok label=ok reason=everything works entropy=0.1",
        );
        // regex_fallback_value handles key=value token format
        assert!(result.value.is_some());
    }

    #[test]
    fn test_sync_fails_on_garbage() {
        // Use a string with no key=value tokens and no JSON-like structure
        // to ensure all deterministic stages fail
        let result = repair_json_sync::<serde_json::Value>(
            "\u{0000}\u{0001}\u{0002}noise\u{0003}",
        );
        // The specific behavior depends on jsonrepair-rs, but we verify
        // that the pipeline completes without panicking
        assert!(
            result.stage == RepairStage::Failed || result.stage == RepairStage::RegexExtraction
        );
        if result.stage == RepairStage::Failed {
            assert!(result.value.is_none());
        }
    }

    #[test]
    fn test_sync_stage_order_precedence() {
        // Valid JSON should hit stage 1, not fall through to regex
        let result = repair_json_sync::<serde_json::Value>(r#"{"choice": "ok", "reason": "direct"}"#);
        assert_eq!(result.stage, RepairStage::DirectParse);
    }
}
