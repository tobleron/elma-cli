//! @efficiency-role: infra-adapter
//!
//! JSON Grammar Module
//!
//! Loads and injects GBNF grammars into ChatCompletionRequest.
//!
//! GBNF grammars enforce 100% valid JSON output at token generation level.
//! Model literally cannot produce invalid JSON because grammar blocks invalid tokens.
//!
//! Grammar Configuration:
//! Grammars are loaded from config/grammars/ directory.
//! Profile-to-grammar mapping is stored in config/grammar_mapping.toml

use crate::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// ============================================================================
// Grammar Loading
// ============================================================================

/// Load GBNF grammar from file
///
/// Grammar files are in config/grammars/ directory.
/// Example: config/grammars/router_choice_1of5.json.gbnf
pub(crate) fn load_grammar(grammar_path: &str, config_root: &Path) -> Result<String> {
    let full_path = config_root.join(grammar_path);

    if !full_path.exists() {
        return Err(anyhow::anyhow!(
            "Grammar file not found: {}",
            full_path.display()
        ));
    }

    let grammar = fs::read_to_string(&full_path)
        .with_context(|| format!("Failed to read grammar file: {}", full_path.display()))?;

    Ok(grammar.trim().to_string())
}

/// Load grammar mapping from config file
///
/// Returns map of profile_name -> grammar_path
pub(crate) fn load_grammar_mapping(config_root: &Path) -> Result<HashMap<String, String>> {
    let mapping_path = config_root.join("grammar_mapping.toml");

    if !mapping_path.exists() {
        // Return empty mapping if file doesn't exist (optional feature)
        return Ok(HashMap::new());
    }

    let content = fs::read_to_string(&mapping_path)?;
    let mut mapping = HashMap::new();

    // Simple TOML parsing for [profile_name] grammar_path = "path"
    let mut current_profile = None;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            current_profile = Some(line[1..line.len() - 1].to_string());
        } else if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim();
            let value = line[eq_pos + 1..].trim().trim_matches('"');
            if key == "grammar_path" && current_profile.is_some() {
                let profile = current_profile.take().unwrap();
                mapping.insert(profile, value.to_string());
            }
        }
    }

    Ok(mapping)
}

/// Get grammar path for a profile by name
pub(crate) fn get_grammar_for_profile(
    profile_name: &str,
    config_root: &Path,
) -> Result<Option<String>> {
    let mapping = load_grammar_mapping(config_root)?;
    Ok(mapping.get(profile_name).cloned())
}

// ============================================================================
// Grammar Injection
// ============================================================================

/// Inject grammar into ChatCompletionRequest
///
/// Modifies request to include grammar constraint.
/// Model will be forced to output only valid JSON matching grammar.
pub(crate) fn inject_grammar(request: &mut ChatCompletionRequest, grammar: &str) -> Result<()> {
    request.grammar = Some(grammar.to_string());
    Ok(())
}

/// Inject grammar for a profile by name
///
/// Loads grammar from config and injects into request.
pub(crate) fn inject_grammar_for_profile(
    request: &mut ChatCompletionRequest,
    profile_name: &str,
    config_root: &Path,
) -> Result<bool> {
    if let Some(grammar_path) = get_grammar_for_profile(profile_name, config_root)? {
        let grammar = load_grammar(&grammar_path, config_root)?;
        inject_grammar(request, &grammar)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

// ============================================================================
// Grammar Validation
// ============================================================================

/// Validate GBNF grammar syntax
///
/// Basic validation:
/// - Has root rule
/// - Rules use ::= syntax
/// - No obvious syntax errors
///
/// Note: Does not guarantee grammar works with llama.cpp, only checks basic syntax.
pub(crate) fn validate_grammar(grammar: &str) -> Result<()> {
    let grammar_trimmed = grammar.trim();

    // Check for root rule
    if !grammar_trimmed.contains("root ::=") {
        return Err(anyhow::anyhow!("Grammar missing root rule"));
    }

    // Check for rule definitions
    let rule_count = grammar_trimmed.matches("::=").count();
    if rule_count < 1 {
        return Err(anyhow::anyhow!("Grammar has no rules"));
    }

    // Check for common syntax errors
    if grammar_trimmed.contains("::= ::=") {
        return Err(anyhow::anyhow!("Grammar has duplicate ::= operators"));
    }

    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_grammar_valid() -> Result<()> {
        let grammar = r#"
            root ::= "{" ws "\"choice\":" ws string "}"
            string ::= "\"" [a-zA-Z]* "\""
            ws ::= [ \t\n]*
        "#;

        validate_grammar(grammar)?;
        Ok(())
    }

    #[test]
    fn test_validate_grammar_missing_root() {
        let grammar = r#"
            choice ::= "\"CHAT\"" | "\"INVESTIGATE\""
        "#;

        let result = validate_grammar(grammar);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("missing root rule"));
    }

    #[test]
    fn test_load_grammar_file_not_found() {
        let result = load_grammar("nonexistent.gbnf", &std::env::temp_dir());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
