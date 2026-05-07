//! @efficiency-role: domain-logic
//!
//! Evidence Grounded Finalization Honesty — Task 690.
//!
//! Validates final-answer claims against gathered evidence.
//! Extracts atomic claims about files created, edits applied,
//! online verification, and command success, then checks each
//! against the evidence ledger and artifact existence.

use crate::*;
use std::collections::HashSet;

/// Pattern types for claims extracted from final answers.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ClaimKind {
    FileCreated(String),
    FileEdited(String),
    FileVerified(String),
    OnlineVerified(String),
    CommandSuccess(String),
    Completion,
}

/// A single claim extracted from a final answer.
#[derive(Debug, Clone)]
pub(crate) struct FinalClaim {
    pub kind: ClaimKind,
    pub text: String,
    pub supported: bool,
    pub note: String,
}

/// Extract file-creation and verification claims from a final answer.
pub(crate) fn extract_claims(final_answer: &str) -> Vec<FinalClaim> {
    let mut claims = Vec::new();

    for line in final_answer.lines() {
        let lower = line.to_lowercase();

        // Created: pattern
        if lower.contains("created:") || lower.contains("created ") {
            if let Some(path) = extract_path_from_claim(line) {
                claims.push(FinalClaim {
                    kind: ClaimKind::FileCreated(path),
                    text: line.trim().to_string(),
                    supported: false,
                    note: String::new(),
                });
            }
        }

        // Wrote to / saved to pattern
        if (lower.contains("wrote ") || lower.contains("saved "))
            && (lower.contains(".md") || lower.contains(".rs") || lower.contains(".txt")
                || lower.contains(".json") || lower.contains(".toml"))
        {
            if let Some(path) = extract_path_from_claim(line) {
                claims.push(FinalClaim {
                    kind: ClaimKind::FileCreated(path),
                    text: line.trim().to_string(),
                    supported: false,
                    note: String::new(),
                });
            }
        }

        // "verified" or "confirmed" patterns — online verification claims
        if (lower.contains("verified") || lower.contains("confirmed"))
            && (lower.contains("online") || lower.contains("url") || lower.contains("http")
                || lower.contains("current") || lower.contains("up-to-date")
                || lower.contains("latest"))
        {
            claims.push(FinalClaim {
                kind: ClaimKind::OnlineVerified(line.trim().to_string()),
                text: line.trim().to_string(),
                supported: false,
                note: String::new(),
            });
        }
    }

    claims
}

/// Extract a file path from a claim line using heuristics.
fn extract_path_from_claim(line: &str) -> Option<String> {
    let candidates: Vec<&str> = line
        .split_whitespace()
        .filter(|w| w.contains('/') || w.contains(".md") || w.contains(".rs")
            || w.contains(".txt") || w.contains(".json") || w.contains(".toml"))
        .collect();

    for c in candidates {
        let clean = c
            .trim_start_matches('`')
            .trim_end_matches('`')
            .trim_end_matches(|ch: char| ch == '.' || ch == ',' || ch == ')' || ch == ']');
        if !clean.is_empty() && clean.contains(|ch: char| ch.is_alphanumeric()) {
            return Some(clean.to_string());
        }
    }
    None
}

/// Validate file-creation claims against the evidence ledger and filesystem.
pub(crate) fn validate_file_claims(
    claims: &[FinalClaim],
    messages: &[ChatMessage],
    workspace_root: &Path,
) -> Vec<FinalClaim> {
    // Build set of files that were successfully written (from tool messages)
    let written_files: HashSet<String> = messages
        .iter()
        .filter(|m| m.role == "tool" && m.name.as_deref() == Some("write"))
        .map(|m| {
            // Extract path from the assistant message's tool_call
            String::new()
        })
        .collect();

    // Check from tool call assistant messages
    let mut written_from_calls: HashSet<String> = HashSet::new();
    for msg in messages {
        if let Some(calls) = &msg.tool_calls {
            for call in calls {
                if call.function.name == "write" {
                    if let Ok(args) = serde_json::from_str::<serde_json::Value>(
                        &call.function.arguments,
                    ) {
                        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                            written_from_calls.insert(path.to_string());
                        }
                    }
                }
            }
        }
    }

    // Check backup tool messages for manifest paths
    let backup_manifest_basenames: Vec<String> = messages
        .iter()
        .filter(|m| m.role == "tool" && m.name.as_deref() == Some("backup"))
        .filter_map(|m| {
            m.content.lines().find_map(|l| {
                let trimmed = l.trim();
                if trimmed.starts_with("Manifest:") {
                    trimmed.split(':').nth(1).map(|p| {
                        std::path::Path::new(p.trim())
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("backup_manifest.txt")
                            .to_string()
                    })
                } else {
                    None
                }
            })
        })
        .collect();

    let mut validated = Vec::new();
    for claim in claims {
        match &claim.kind {
            ClaimKind::FileCreated(path) => {
                let exists = workspace_root.join(path).exists();
                let in_writes = written_from_calls.contains(path);
                let in_backup_manifest = backup_manifest_basenames.iter().any(|b| path.ends_with(b));
                if exists && (in_writes || in_backup_manifest) {
                    validated.push(FinalClaim {
                        supported: true,
                        note: format!("confirmed: {} exists on disk", path),
                        ..claim.clone()
                    });
                } else if exists {
                    validated.push(FinalClaim {
                        supported: true,
                        note: format!("file exists but no explicit write event found for {}", path),
                        ..claim.clone()
                    });
                } else if in_backup_manifest {
                    validated.push(FinalClaim {
                        supported: true,
                        note: format!("confirmed: {} created by backup tool", path),
                        ..claim.clone()
                    });
                } else {
                    validated.push(FinalClaim {
                        supported: false,
                        note: format!("UNVERIFIED: {} does not exist on disk", path),
                        ..claim.clone()
                    });
                }
            }
            ClaimKind::OnlineVerified(text) => {
                // Check if any fetch tool event exists
                let has_fetch = messages.iter().any(|m| {
                    m.role == "tool" && m.name.as_deref() == Some("fetch")
                });
                if has_fetch {
                    validated.push(FinalClaim {
                        supported: true,
                        note: "verified by fetch tool evidence".to_string(),
                        ..claim.clone()
                    });
                } else {
                    validated.push(FinalClaim {
                        supported: false,
                        note: "no network/fetch tool evidence found".to_string(),
                        ..claim.clone()
                    });
                }
            }
            _ => {
                validated.push(claim.clone());
            }
        }
    }

    validated
}

/// Build a correction appendix for unsupported claims in the final answer.
pub(crate) fn build_unsupported_claims_appendix(
    validated_claims: &[FinalClaim],
) -> Option<String> {
    let unsupported: Vec<&FinalClaim> = validated_claims
        .iter()
        .filter(|c| !c.supported)
        .collect();

    if unsupported.is_empty() {
        return None;
    }

    let items: Vec<String> = unsupported
        .iter()
        .map(|c| match &c.kind {
            ClaimKind::FileCreated(path) => {
                format!("- File '{}' was claimed but could not be verified on disk", path)
            }
            ClaimKind::OnlineVerified(text) => {
                format!("- Online verification claim: '{}' — no network evidence found", text)
            }
            _ => format!("- {}", c.text),
        })
        .collect();

    Some(format!(
        "\n\n**Note:** The following claims in the answer above could not be \
         verified against available evidence:\n{}\n\
         These statements should be treated with caution.",
        items.join("\n")
    ))
}

/// Validate a final answer against all available evidence and append corrections.
/// Returns the original answer with corrections appended if needed.
pub(crate) fn verify_final_answer(
    final_answer: &str,
    messages: &[ChatMessage],
    workspace_root: &Path,
) -> String {
    let claims = extract_claims(final_answer);
    if claims.is_empty() {
        return final_answer.to_string();
    }

    let validated = validate_file_claims(&claims, messages, workspace_root);
    if let Some(appendix) = build_unsupported_claims_appendix(&validated) {
        format!("{}{}", final_answer.trim(), appendix)
    } else {
        final_answer.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tool_msg(name: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: "tool".to_string(),
            content: content.to_string(),
            name: Some(name.to_string()),
            tool_calls: None,
            tool_call_id: Some("t1".to_string()),
            reasoning_content: None,
            summarized: false,
        }
    }

    fn make_assistant_with_tool_call(name: &str, args: &str) -> ChatMessage {
        ChatMessage {
            role: "assistant".to_string(),
            content: String::new(),
            name: None,
            tool_calls: Some(vec![ToolCall {
                id: "c1".to_string(),
                call_type: "function".to_string(),
                function: ToolFunctionCall {
                    name: name.to_string(),
                    arguments: args.to_string(),
                },
            }]),
            tool_call_id: None,
            reasoning_content: None,
            summarized: false,
        }
    }

    #[test]
    fn test_extract_file_created_claim() {
        let answer = "Created: project_tmp/security_report.md with findings.";
        let claims = extract_claims(answer);
        assert_eq!(claims.len(), 1);
        assert!(matches!(&claims[0].kind, ClaimKind::FileCreated(p) if p == "project_tmp/security_report.md"));
    }

    #[test]
    fn test_extract_online_verification_claim() {
        let answer = "Verified that all API references are current and up-to-date.";
        let claims = extract_claims(answer);
        assert!(!claims.is_empty());
        assert!(claims.iter().any(|c| matches!(c.kind, ClaimKind::OnlineVerified(_))));
    }

    #[test]
    fn test_extract_no_claims_from_clean_answer() {
        let answer = "I found 3 TODO comments in the source code.";
        let claims = extract_claims(answer);
        assert!(claims.is_empty());
    }

    #[test]
    fn test_validate_file_claim_exists() {
        let claims = vec![FinalClaim {
            kind: ClaimKind::FileCreated("Cargo.toml".to_string()),
            text: "Created Cargo.toml".to_string(),
            supported: false,
            note: String::new(),
        }];
        let msg = make_assistant_with_tool_call("write", r#"{"path": "Cargo.toml", "content": "data"}"#);
        let validated = validate_file_claims(&claims, &[msg], Path::new("."));
        assert!(validated[0].supported);
    }

    #[test]
    fn test_validate_file_claim_missing() {
        let claims = vec![FinalClaim {
            kind: ClaimKind::FileCreated("nonexistent_98765.md".to_string()),
            text: "Created nonexistent_98765.md".to_string(),
            supported: false,
            note: String::new(),
        }];
        let validated = validate_file_claims(&claims, &[], Path::new("/tmp"));
        assert!(!validated[0].supported);
    }

    #[test]
    fn test_verify_clean_answer_no_changes() {
        let answer = "The project has 42 source files.";
        let result = verify_final_answer(answer, &[], Path::new("."));
        assert_eq!(result, answer);
    }

    #[test]
    fn test_build_unsupported_appendix() {
        let claims = vec![
            FinalClaim {
                kind: ClaimKind::FileCreated("missing.md".to_string()),
                text: "Created missing.md".to_string(),
                supported: false,
                note: "does not exist".to_string(),
            },
            FinalClaim {
                kind: ClaimKind::FileCreated("exists.md".to_string()),
                text: "Created exists.md".to_string(),
                supported: true,
                note: "confirmed".to_string(),
            },
        ];
        let appendix = build_unsupported_claims_appendix(&claims);
        assert!(appendix.is_some());
        let appendix = appendix.unwrap();
        assert!(appendix.contains("missing.md"));
        assert!(!appendix.contains("exists.md"));
    }

    #[test]
    fn test_no_appendix_when_all_supported() {
        let claims = vec![FinalClaim {
            kind: ClaimKind::FileCreated("exists.md".to_string()),
            text: "Created exists.md".to_string(),
            supported: true,
            note: "confirmed".to_string(),
        }];
        assert!(build_unsupported_claims_appendix(&claims).is_none());
    }

    #[test]
    fn test_extract_path_from_claim_md_file() {
        let result = extract_path_from_claim("I wrote the report to project_tmp/findings.md");
        assert_eq!(result, Some("project_tmp/findings.md".to_string()));
    }

    #[test]
    fn test_extract_path_no_path() {
        let result = extract_path_from_claim("Completed all tasks successfully.");
        assert!(result.is_none());
    }

    #[test]
    fn test_online_verification_no_network_evidence() {
        let answer = "Verified that all dependencies are up-to-date via online check.";
        let claims = extract_claims(answer);
        // No fetch tool evidence in messages
        let validated = validate_file_claims(&claims, &[], Path::new("."));
        let online_claims: Vec<_> = validated.iter().filter(|c| matches!(c.kind, ClaimKind::OnlineVerified(_))).collect();
        assert!(!online_claims.is_empty());
        assert!(!online_claims[0].supported);
        assert!(online_claims[0].note.contains("no network"));
    }
}
