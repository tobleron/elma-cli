//! @efficiency-role: service-orchestrator
//!
//! Approach branch rehydration — converts failed approaches into new sibling
//! branches with a rehydration plan. Provides failure taxonomy classification
//! based on error messages and tool context.

use crate::work_graph::ApproachId;
use crate::*;

/// Taxonomy of approach failure reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FailureTaxonomy {
    ToolError,
    ModelError,
    PermissionDenied,
    Timeout,
    Stagnation,
    ContextLimit,
    UserInterrupt,
    Unknown,
}

impl FailureTaxonomy {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            FailureTaxonomy::ToolError => "tool_error",
            FailureTaxonomy::ModelError => "model_error",
            FailureTaxonomy::PermissionDenied => "permission_denied",
            FailureTaxonomy::Timeout => "timeout",
            FailureTaxonomy::Stagnation => "stagnation",
            FailureTaxonomy::ContextLimit => "context_limit",
            FailureTaxonomy::UserInterrupt => "user_interrupt",
            FailureTaxonomy::Unknown => "unknown",
        }
    }
}

/// Record of a failed approach with classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FailedApproach {
    pub(crate) approach_id: String,
    pub(crate) goal: String,
    pub(crate) failure: FailureTaxonomy,
    pub(crate) evidence: String,
    pub(crate) timestamp: u64,
}

/// Plan for rehydrating a failed approach into a new sibling branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RehydrationPlan {
    pub(crate) new_approach_id: String,
    pub(crate) sibling_goal: String,
    pub(crate) avoid_patterns: Vec<String>,
    pub(crate) retry_strategy: String,
}

/// Converts failed approaches into rehydration plans with failure-aware strategies.
pub(crate) struct ApproachRehydrator;

impl ApproachRehydrator {
    /// Generate a rehydration plan from a failed approach.
    ///
    /// The plan includes a new approach ID, a modified sibling goal that
    /// embeds the failure context, patterns to avoid, and a retry strategy.
    pub(crate) fn rehydrate(failed: &FailedApproach) -> RehydrationPlan {
        let new_approach_id = ApproachId::new().0;

        let (sibling_goal, avoid_patterns, retry_strategy) = match failed.failure {
            FailureTaxonomy::ToolError => (
                format!("{} (alt-tool)", failed.goal),
                vec!["use different tool".to_string()],
                "retry with alternative tool or approach".to_string(),
            ),
            FailureTaxonomy::ModelError => (
                format!("{} (retry)", failed.goal),
                vec!["avoid malformed output".to_string()],
                "retry with lower temperature and stricter output format".to_string(),
            ),
            FailureTaxonomy::PermissionDenied => (
                format!("{} (escalated)", failed.goal),
                vec!["avoid denied paths/commands".to_string()],
                "request permission or use alternative path".to_string(),
            ),
            FailureTaxonomy::Timeout => (
                format!("{} (simplified)", failed.goal),
                vec!["reduce scope".to_string()],
                "decompose into smaller steps with shorter timeouts".to_string(),
            ),
            FailureTaxonomy::Stagnation => (
                format!("{} (reframed)", failed.goal),
                vec![failed.evidence.clone()],
                "change approach strategy to avoid repetition".to_string(),
            ),
            FailureTaxonomy::ContextLimit => (
                format!("{} (compact)", failed.goal),
                vec!["reduce context".to_string()],
                "compact context, trim history, retry with minimal context".to_string(),
            ),
            FailureTaxonomy::UserInterrupt => (
                failed.goal.clone(),
                vec!["user interruption logged".to_string()],
                "resume on user request with fresh context".to_string(),
            ),
            FailureTaxonomy::Unknown => (
                format!("{} (investigate)", failed.goal),
                vec!["undiagnosed failure".to_string()],
                "retry with enhanced diagnostics and error capture".to_string(),
            ),
        };

        RehydrationPlan {
            new_approach_id,
            sibling_goal,
            avoid_patterns,
            retry_strategy,
        }
    }

    /// Classify a failure based on the error message and tool name.
    ///
    /// Uses substring matching on lowercased inputs to classify into one of
    /// the FailureTaxonomy variants. Falls back to Unknown when no pattern matches.
    pub(crate) fn classify_failure(error: &str, tool: &str) -> FailureTaxonomy {
        let error_lower = error.to_lowercase();
        let tool_lower = tool.to_lowercase();

        if error_lower.contains("permission")
            || error_lower.contains("denied")
            || error_lower.contains("not allowed")
            || error_lower.contains("forbidden")
        {
            return FailureTaxonomy::PermissionDenied;
        }

        if error_lower.contains("timeout")
            || error_lower.contains("timed out")
            || error_lower.contains("deadline")
        {
            return FailureTaxonomy::Timeout;
        }

        if error_lower.contains("stagnat")
            || error_lower.contains("no progress")
            || error_lower.contains("repetition")
            || error_lower.contains("loop")
        {
            return FailureTaxonomy::Stagnation;
        }

        if error_lower.contains("context")
            && (error_lower.contains("limit")
                || error_lower.contains("overflow")
                || error_lower.contains("exceeded"))
        {
            return FailureTaxonomy::ContextLimit;
        }

        if error_lower.contains("interrupt") || error_lower.contains("cancelled") {
            return FailureTaxonomy::UserInterrupt;
        }

        if tool_lower.contains("model")
            || tool_lower.contains("llm")
            || tool_lower.contains("completion")
            || error_lower.contains("json parse")
            || error_lower.contains("invalid response")
        {
            return FailureTaxonomy::ModelError;
        }

        if tool_lower.contains("shell")
            || tool_lower.contains("bash")
            || tool_lower.contains("exec")
            || error_lower.contains("exit code")
            || error_lower.contains("non-zero")
        {
            return FailureTaxonomy::ToolError;
        }

        FailureTaxonomy::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_failed(
        approach_id: &str,
        goal: &str,
        failure: FailureTaxonomy,
        evidence: &str,
    ) -> FailedApproach {
        FailedApproach {
            approach_id: approach_id.to_string(),
            goal: goal.to_string(),
            failure,
            evidence: evidence.to_string(),
            timestamp: 1000000,
        }
    }

    #[test]
    fn test_failure_taxonomy_labels() {
        assert_eq!(FailureTaxonomy::ToolError.label(), "tool_error");
        assert_eq!(FailureTaxonomy::ModelError.label(), "model_error");
        assert_eq!(
            FailureTaxonomy::PermissionDenied.label(),
            "permission_denied"
        );
        assert_eq!(FailureTaxonomy::Timeout.label(), "timeout");
        assert_eq!(FailureTaxonomy::Stagnation.label(), "stagnation");
        assert_eq!(FailureTaxonomy::ContextLimit.label(), "context_limit");
        assert_eq!(FailureTaxonomy::UserInterrupt.label(), "user_interrupt");
        assert_eq!(FailureTaxonomy::Unknown.label(), "unknown");
    }

    #[test]
    fn test_classify_tool_error_exit_code() {
        let result = ApproachRehydrator::classify_failure("exit code 1", "shell");
        assert_eq!(result, FailureTaxonomy::ToolError);
    }

    #[test]
    fn test_classify_tool_error_non_zero() {
        let result = ApproachRehydrator::classify_failure("non-zero return", "bash");
        assert_eq!(result, FailureTaxonomy::ToolError);
    }

    #[test]
    fn test_classify_model_error() {
        let result =
            ApproachRehydrator::classify_failure("invalid json response from llm", "completion");
        assert_eq!(result, FailureTaxonomy::ModelError);
    }

    #[test]
    fn test_classify_permission_denied() {
        let result = ApproachRehydrator::classify_failure("permission denied: /etc/shadow", "bash");
        assert_eq!(result, FailureTaxonomy::PermissionDenied);
    }

    #[test]
    fn test_classify_not_allowed() {
        let result = ApproachRehydrator::classify_failure("operation not allowed", "tool");
        assert_eq!(result, FailureTaxonomy::PermissionDenied);
    }

    #[test]
    fn test_classify_forbidden() {
        let result = ApproachRehydrator::classify_failure("forbidden path", "read");
        assert_eq!(result, FailureTaxonomy::PermissionDenied);
    }

    #[test]
    fn test_classify_timeout() {
        let result = ApproachRehydrator::classify_failure("operation timed out after 30s", "shell");
        assert_eq!(result, FailureTaxonomy::Timeout);
    }

    #[test]
    fn test_classify_deadline() {
        let result = ApproachRehydrator::classify_failure("deadline exceeded", "exec");
        assert_eq!(result, FailureTaxonomy::Timeout);
    }

    #[test]
    fn test_classify_stagnation() {
        let result =
            ApproachRehydrator::classify_failure("no progress detected after 5 attempts", "tool");
        assert_eq!(result, FailureTaxonomy::Stagnation);
    }

    #[test]
    fn test_classify_repetition() {
        let result = ApproachRehydrator::classify_failure("repetition detected", "agent");
        assert_eq!(result, FailureTaxonomy::Stagnation);
    }

    #[test]
    fn test_classify_context_limit() {
        let result = ApproachRehydrator::classify_failure("context limit exceeded", "model");
        assert_eq!(result, FailureTaxonomy::ContextLimit);
    }

    #[test]
    fn test_classify_context_overflow() {
        let result = ApproachRehydrator::classify_failure("context window overflow", "llm");
        assert_eq!(result, FailureTaxonomy::ContextLimit);
    }

    #[test]
    fn test_classify_user_interrupt() {
        let result = ApproachRehydrator::classify_failure("user interrupt received", "session");
        assert_eq!(result, FailureTaxonomy::UserInterrupt);
    }

    #[test]
    fn test_classify_cancelled() {
        let result = ApproachRehydrator::classify_failure("operation cancelled", "tool");
        assert_eq!(result, FailureTaxonomy::UserInterrupt);
    }

    #[test]
    fn test_classify_unknown() {
        let result = ApproachRehydrator::classify_failure("unexpected error: disk full", "storage");
        assert_eq!(result, FailureTaxonomy::Unknown);
    }

    #[test]
    fn test_classify_insensitive_case() {
        let result = ApproachRehydrator::classify_failure("PERMISSION DENIED", "TOOL");
        assert_eq!(result, FailureTaxonomy::PermissionDenied);
    }

    #[test]
    fn test_rehydrate_tool_error() {
        let failed = make_failed(
            "a_1",
            "Install deps",
            FailureTaxonomy::ToolError,
            "npm error",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("alt-tool"));
        assert!(plan.retry_strategy.contains("alternative"));
        assert!(plan.new_approach_id.starts_with("a_"));
    }

    #[test]
    fn test_rehydrate_stagnation() {
        let failed = make_failed(
            "a_2",
            "Fix bug",
            FailureTaxonomy::Stagnation,
            "same error repeated",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("reframed"));
        assert!(plan
            .avoid_patterns
            .contains(&"same error repeated".to_string()));
    }

    #[test]
    fn test_rehydrate_user_interrupt_copies_goal() {
        let failed = make_failed(
            "a_3",
            "Write tests",
            FailureTaxonomy::UserInterrupt,
            "ctrl-c",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert_eq!(plan.sibling_goal, "Write tests");
    }

    #[test]
    fn test_rehydrate_timeout_simplifies_goal() {
        let failed = make_failed(
            "a_4",
            "Build project",
            FailureTaxonomy::Timeout,
            "30s timeout",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("simplified"));
        assert!(plan.retry_strategy.contains("shorter"));
    }

    #[test]
    fn test_rehydrate_model_error() {
        let failed = make_failed(
            "a_5",
            "Parse config",
            FailureTaxonomy::ModelError,
            "bad json",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("retry"));
        assert!(plan.retry_strategy.contains("temperature"));
    }

    #[test]
    fn test_rehydrate_permission_denied() {
        let failed = make_failed(
            "a_6",
            "Read /etc",
            FailureTaxonomy::PermissionDenied,
            "denied",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("escalated"));
        assert!(plan.retry_strategy.contains("permission"));
    }

    #[test]
    fn test_rehydrate_context_limit() {
        let failed = make_failed(
            "a_7",
            "Analyze codebase",
            FailureTaxonomy::ContextLimit,
            "too long",
        );
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("compact"));
        assert!(plan.retry_strategy.contains("compact context"));
    }

    #[test]
    fn test_rehydrate_unknown() {
        let failed = make_failed("a_8", "Do thing", FailureTaxonomy::Unknown, "mystery error");
        let plan = ApproachRehydrator::rehydrate(&failed);
        assert!(plan.sibling_goal.contains("investigate"));
        assert!(plan.retry_strategy.contains("diagnostics"));
    }

    #[test]
    fn test_rehydrate_unique_approach_ids() {
        let f1 = make_failed("a1", "x", FailureTaxonomy::ToolError, "err");
        let f2 = make_failed("a2", "x", FailureTaxonomy::ToolError, "err");
        let p1 = ApproachRehydrator::rehydrate(&f1);
        let p2 = ApproachRehydrator::rehydrate(&f2);
        assert_ne!(p1.new_approach_id, p2.new_approach_id);
    }
}
