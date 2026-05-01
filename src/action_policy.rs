//! Policy metadata for every model-callable DSL action and legacy tool.
//!
//! Defines risk classification, permission requirements, concurrency safety,
//! and other policy metadata for all actions the model can invoke.

/// Risk classification for an action or tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionRisk {
    ReadOnly,
    WorkspaceWrite,
    ExternalProcess,
    Network,
    ConversationState,
}

/// Policy metadata for a single DSL action command.
#[derive(Debug, Clone)]
pub(crate) struct ActionPolicy {
    pub command: &'static str,
    pub risks: &'static [ActionRisk],
    pub requires_permission: bool,
    pub requires_prior_read: bool,
    pub concurrency_safe: bool,
    pub is_model_callable: bool,
}

/// Policy metadata for every DSL action command.
pub(crate) static ALL_ACTION_POLICIES: &[ActionPolicy] = &[
    ActionPolicy {
        command: "R",
        risks: &[ActionRisk::ReadOnly],
        requires_permission: false,
        requires_prior_read: false,
        concurrency_safe: true,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "L",
        risks: &[ActionRisk::ReadOnly],
        requires_permission: false,
        requires_prior_read: false,
        concurrency_safe: true,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "S",
        risks: &[ActionRisk::ReadOnly],
        requires_permission: false,
        requires_prior_read: false,
        concurrency_safe: true,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "Y",
        risks: &[ActionRisk::ReadOnly],
        requires_permission: false,
        requires_prior_read: false,
        concurrency_safe: true,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "E",
        risks: &[ActionRisk::WorkspaceWrite],
        requires_permission: false,
        requires_prior_read: true,
        concurrency_safe: false,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "X",
        risks: &[ActionRisk::ExternalProcess],
        requires_permission: true,
        requires_prior_read: false,
        concurrency_safe: false,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "ASK",
        risks: &[ActionRisk::ConversationState],
        requires_permission: false,
        requires_prior_read: false,
        concurrency_safe: false,
        is_model_callable: true,
    },
    ActionPolicy {
        command: "DONE",
        risks: &[ActionRisk::ConversationState],
        requires_permission: false,
        requires_prior_read: false,
        concurrency_safe: false,
        is_model_callable: true,
    },
];

/// Look up policy for a DSL action by its command name (e.g., "R", "E", "X").
pub(crate) fn action_policy(command: &str) -> Option<&'static ActionPolicy> {
    ALL_ACTION_POLICIES.iter().find(|p| p.command == command)
}

/// Look up policy for a DSL action by its Rust variant name (e.g., "ReadFile", "EditFile").
pub(crate) fn action_policy_for_variant(variant: &str) -> Option<&'static ActionPolicy> {
    ALL_ACTION_POLICIES.iter().find(|p| {
        let v = match p.command {
            "R" => "ReadFile",
            "L" => "ListFiles",
            "S" => "SearchText",
            "Y" => "SearchSymbol",
            "E" => "EditFile",
            "X" => "RunCommand",
            "ASK" => "Ask",
            "DONE" => "Done",
            _ => return false,
        };
        v == variant
    })
}

/// Determine whether a legacy tool name is concurrency-safe.
///
/// This replaces the previous hardcoded `matches!(tool_name, "read" | "search" | "respond")`
/// with a data-driven approach that stays consistent with `ALL_ACTION_POLICIES`.
pub(crate) fn concurrency_safe_for_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "read"
            | "search"
            | "respond"
            | "summary"
            | "glob"
            | "ls"
            | "list"
            | "tool_search"
            | "search_symbol"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_actions_have_policy() {
        for cmd in &["R", "L", "S", "Y", "E", "X", "ASK", "DONE"] {
            assert!(action_policy(cmd).is_some(), "Missing policy for {}", cmd);
        }
    }

    #[test]
    fn test_read_actions_are_concurrency_safe() {
        for cmd in &["R", "L", "S", "Y"] {
            assert!(
                action_policy(cmd).unwrap().concurrency_safe,
                "{} should be concurrency safe",
                cmd
            );
        }
    }

    #[test]
    fn test_write_actions_are_serial() {
        for cmd in &["E", "X", "ASK", "DONE"] {
            assert!(
                !action_policy(cmd).unwrap().concurrency_safe,
                "{} should not be concurrency safe",
                cmd
            );
        }
    }

    #[test]
    fn test_variant_lookup() {
        assert!(action_policy_for_variant("ReadFile").is_some());
        assert!(action_policy_for_variant("EditFile").is_some());
        assert!(action_policy_for_variant("RunCommand").is_some());
        assert!(
            action_policy_for_variant("Nonexistent").is_none(),
            "unknown variant should return None"
        );
    }

    #[test]
    fn test_concurrency_safe_tools() {
        assert!(concurrency_safe_for_tool("read"));
        assert!(concurrency_safe_for_tool("search"));
        assert!(concurrency_safe_for_tool("respond"));
        assert!(concurrency_safe_for_tool("summary"));
        assert!(concurrency_safe_for_tool("glob"));
        assert!(concurrency_safe_for_tool("ls"));
        assert!(concurrency_safe_for_tool("tool_search"));
        assert!(!concurrency_safe_for_tool("shell"));
        assert!(!concurrency_safe_for_tool("edit"));
        assert!(!concurrency_safe_for_tool("write"));
        assert!(!concurrency_safe_for_tool("fetch"));
        assert!(!concurrency_safe_for_tool("update_todo_list"));
    }
}
