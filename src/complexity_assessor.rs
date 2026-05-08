//! @efficiency-role: domain-logic
//! Lightweight heuristic-based complexity assessment.
//!
//! Analyzes the user message for signals that indicate task complexity:
//! multi-step operations, file references, action verbs, etc.
//! This runs before the tool loop to determine iteration budget.

/// Complexity level for a user request.
/// Two gates: Direct (no graph) or Multistep (Objective → SubGoal → Instruction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Complexity {
    Direct,
    Multistep,
}

impl Complexity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Complexity::Direct => "DIRECT",
            Complexity::Multistep => "MULTISTEP",
        }
    }

    pub fn max_iterations(&self) -> usize {
        match self {
            Complexity::Direct => 3,
            Complexity::Multistep => 12,
        }
    }
}

/// Assess complexity of a user request using lightweight heuristics.
/// Returns a Complexity level based on structural signals in the text.
pub fn assess_complexity(user_message: &str) -> Complexity {
    let lower = user_message.to_lowercase();
    let word_count = lower.split_whitespace().count();

    // Multi-doc / multi-file signals → OPEN_ENDED or MULTISTEP
    let multi_edit_signals = [
        "all docs",
        "every file",
        "all files",
        "compare with",
        "throughout",
        "across the",
        "entire project",
        "whole codebase",
        "everywhere",
        "all occurrences",
        "all references",
        "multiple files",
        "many files",
        "each file",
    ];
    let has_multi_signal = multi_edit_signals.iter().any(|s| lower.contains(s));

    // Code modification signals → MULTISTEP
    let code_change_signals = [
        "refactor",
        "implement",
        "create",
        "write a",
        "add a",
        "change",
        "modify",
        "update all",
        "migrate",
        "convert",
        "rename",
        "move",
        "extract",
    ];
    let has_code_signal = code_change_signals.iter().any(|s| lower.contains(s));

    // Investigation / read signals → MULTISTEP
    let investigate_signals = [
        "find",
        "search",
        "look for",
        "check",
        "examine",
        "read",
        "show me",
        "tell me about",
        "what is",
        "how does",
        "where is",
        "why does",
        "analyze",
        "list",
        "get",
        "look at",
    ];
    let has_investigate_signal = investigate_signals.iter().any(|s| lower.contains(s));

    // Long message heuristic
    let is_long = word_count > 10;

    // Greeting / simple chat — minimum complexity
    let greeting_signals = ["hi", "hello", "hey", "thanks", "ok", "yes", "no"];
    let is_greeting =
        greeting_signals.iter().any(|s| s == &lower.trim()) || lower.trim().len() < 10;

    // Classification logic (two gates)
    // Multi-doc/file signals => MULTISTEP (scale is implied)
    if has_multi_signal || has_code_signal {
        Complexity::Multistep
    } else if has_investigate_signal || is_long {
        Complexity::Multistep
    } else if is_greeting {
        Complexity::Direct
    } else {
        Complexity::Multistep // default: assume needs investigation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting() {
        assert_eq!(assess_complexity("hi"), Complexity::Direct);
        assert_eq!(assess_complexity("hello"), Complexity::Direct);
        assert_eq!(assess_complexity("thanks"), Complexity::Direct);
    }

    #[test]
    fn test_direct_question() {
        let result = assess_complexity("what time is it");
        // Short, no multi-file signals → INVESTIGATE (default)
        assert_eq!(result, Complexity::Multistep);
    }

    #[test]
    fn test_read_single_file() {
        let result = assess_complexity("read src/main.rs");
        assert_eq!(result, Complexity::Multistep);
    }

    #[test]
    fn test_read_all_docs() {
        let result = assess_complexity("read all docs and compare with source code");
        assert_eq!(result, Complexity::Multistep);
    }

    #[test]
    fn test_multi_file_refactor() {
        let result = assess_complexity(
            "rename the function getCwd to getCurrentWorkingDirectory across the entire project",
        );
        assert_eq!(result, Complexity::Multistep);
    }

    #[test]
    fn test_investigate() {
        let result = assess_complexity("find all places where the database connection is created");
        assert_eq!(result, Complexity::Multistep);
    }

    #[test]
    fn test_multistep_implementation() {
        let result =
            assess_complexity("add a new endpoint for user registration with validation and tests");
        assert_eq!(result, Complexity::Multistep);
    }

    #[test]
    fn test_complexity_as_str() {
        assert_eq!(Complexity::Direct.as_str(), "DIRECT");
        assert_eq!(Complexity::Multistep.as_str(), "MULTISTEP");
    }

    #[test]
    fn test_max_iterations_scaling() {
        assert_eq!(Complexity::Direct.max_iterations(), 3);
        assert_eq!(Complexity::Multistep.max_iterations(), 12);
    }
}
