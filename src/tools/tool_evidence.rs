//! @efficiency-role: domain-logic
//!
//! Read Evidence Tool
//!
//! Allows the model to intentionally retrieve raw evidence content
//! when compact summaries in the narrative are insufficient.

use crate::evidence_ledger::EvidenceLedger;
use crate::*;

pub(crate) fn execute_read_evidence(ledger: &EvidenceLedger, ids: Vec<String>) -> Result<String> {
    if ids.is_empty() {
        return Ok("Error: No evidence IDs provided.".to_string());
    }

    let mut results = Vec::new();
    for id in &ids {
        match ledger.get_raw(id) {
            Ok(content) => {
                results.push(format!("=== Evidence {id} ===\n{content}"));
            }
            Err(e) => {
                results.push(format!("=== Evidence {id} ===\nError: {e}"));
            }
        }
    }

    Ok(results.join("\n\n"))
}

pub(crate) fn read_evidence_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "read_evidence",
            "description": "Retrieve full raw evidence content by evidence ID. Use when compact summaries in the narrative are insufficient. Evidence IDs look like 'e_001', 'e_002', etc.",
            "parameters": {
                "type": "object",
                "properties": {
                    "ids": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of evidence IDs to retrieve (e.g., [\"e_001\", \"e_002\"])"
                    }
                },
                "required": ["ids"]
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence_ledger::{EvidenceLedger, EvidenceSource};

    fn test_ledger() -> EvidenceLedger {
        let dir = PathBuf::from("/tmp/test_read_evidence");
        let mut ledger = EvidenceLedger::new("s_test", &dir);
        ledger.add_entry(
            EvidenceSource::Shell {
                command: "ls -la".to_string(),
                exit_code: 0,
            },
            "total 48\nAGENTS.md\nCargo.toml",
        );
        ledger
    }

    #[test]
    fn test_read_evidence_single() {
        let ledger = test_ledger();
        let result = execute_read_evidence(&ledger, vec!["e_001".to_string()]).unwrap();
        assert!(result.contains("e_001"));
        assert!(result.contains("AGENTS.md"));
    }

    #[test]
    fn test_read_evidence_empty_ids() {
        let ledger = test_ledger();
        let result = execute_read_evidence(&ledger, vec![]).unwrap();
        assert!(result.contains("No evidence IDs"));
    }

    #[test]
    fn test_read_evidence_nonexistent() {
        let ledger = test_ledger();
        let result = execute_read_evidence(&ledger, vec!["e_999".to_string()]).unwrap();
        assert!(result.contains("Error"));
    }

    #[test]
    fn test_tool_definition() {
        let def = read_evidence_tool_definition();
        assert_eq!(def["function"]["name"], "read_evidence");
        assert!(def["function"]["description"]
            .as_str()
            .unwrap()
            .contains("evidence"));
    }
}
