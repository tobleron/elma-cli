//! @efficiency-role: ignored
//!
//! Tool Orchestration Coverage Certification
//!
//! These tests verify that every tool exposed to the model (via default_tools,
//! DSL commands, or the XML adapter) has the required orchestration coverage.
//! They serve as a regression gate against accidental tool exposure without
//! corresponding DSL support, execution adapters, or safety policy.

use crate::tool_registry;
use crate::*;
use elma_tools::registry::ToolExecutorState;

/// The canonical set of tools returned by default_tools().
/// Every tool here must have either a DSL command or a provider executor.
const CANONICAL_DEFAULT_TOOLS: &[&str] = &[
    "read",
    "respond",
    "search",
    "shell",
    "summary",
    "tool_search",
    "update_todo_list",
];

/// Tools registered as DeclarationOnly that DO have DSL coverage.
/// These are intentionally hidden from provider tool calls but accessible via DSL.
const DSL_COVERED_DECLARATION_ONLY: &[&str] = &["edit", "ls"];

/// Tools registered as DeclarationOnly WITHOUT DSL or provider coverage.
/// These are inert declarations — blueprints for future implementation.
const INERT_DECLARATION_ONLY: &[&str] = &["write", "glob", "fetch", "patch"];

/// All 13 tools registered in the registry.
const ALL_REGISTERED_TOOLS: &[&str] = &[
    "edit",
    "fetch",
    "glob",
    "ls",
    "patch",
    "read",
    "respond",
    "search",
    "shell",
    "summary",
    "tool_search",
    "update_todo_list",
    "write",
];

/// DSL commands that have corresponding AgentAction variants and execution adapters.
const DSL_COMMANDS: &[&str] = &["R", "L", "S", "Y", "E", "X", "ASK", "DONE"];

/// Provider executor names in tool_calling.rs::execute_tool_call().
const PROVIDER_EXECUTORS: &[&str] = &[
    "read",
    "search",
    "shell",
    "respond",
    "summary",
    "tool_search",
    "update_todo_list",
];

// ── Registry Inventory Tests ──

#[test]
fn cert_default_tools_inventory() {
    // The exact set of default tools must match CANONICAL_DEFAULT_TOOLS.
    // If this test fails, a tool was added or removed from default_tools()
    // without updating the orchestration matrix.
    let tools = tool_registry::build_current_tools();
    let mut names: Vec<String> = tools.iter().map(|t| t.function.name.clone()).collect();
    names.sort();

    let mut expected: Vec<String> = CANONICAL_DEFAULT_TOOLS
        .iter()
        .map(|s| s.to_string())
        .collect();
    expected.sort();

    assert_eq!(
        names, expected,
        "default_tools() composition changed. Expected {:?}, got {:?}. \
         If this is intentional, update CANONICAL_DEFAULT_TOOLS in \
         tool_orchestration_certification.rs and the coverage matrix in Task 397.",
        expected, names
    );
}

#[test]
fn cert_every_default_tool_has_coverage() {
    let tools = tool_registry::build_current_tools();
    for tool in &tools {
        let name = &tool.function.name;
        let has_provider_exec = PROVIDER_EXECUTORS.contains(&name.as_str());
        // DSL coverage is indirect — check if the tool maps to an AgentAction
        // via the XML adapter (tool_call_xml.rs)
        let has_dsl = matches!(
            name.as_str(),
            "read" | "search" | "shell" | "respond" | "summary"
        );
        assert!(
            has_provider_exec || has_dsl,
            "Tool '{}' is in default_tools() but has neither a provider executor \
             nor DSL coverage. Either add coverage or mark it DeclarationOnly.",
            name
        );
    }
}

#[test]
fn cert_declaration_only_tools_not_exposed() {
    // DeclarationOnly tools must NOT appear in default_tools().
    let tools = tool_registry::build_current_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();

    for name in INERT_DECLARATION_ONLY {
        assert!(
            !names.contains(name),
            "Inert declaration-only tool '{}' leaked into default_tools()! \
             Remove `.not_deferred()` or add DSL/provider coverage.",
            name
        );
    }
    for name in DSL_COVERED_DECLARATION_ONLY {
        assert!(
            !names.contains(name),
            "DSL-covered declaration-only tool '{}' leaked into default_tools()! \
             It should remain DeclarationOnly and accessed via DSL only.",
            name
        );
    }
}

#[test]
fn cert_registry_contains_all_tools() {
    let registry = tool_registry::get_registry();
    for name in ALL_REGISTERED_TOOLS {
        assert!(
            registry.get(name).is_some(),
            "Expected tool '{}' to be registered in the global registry.",
            name
        );
    }
}

#[test]
fn cert_declaration_only_state_correct() {
    let registry = tool_registry::get_registry();
    for name in INERT_DECLARATION_ONLY
        .iter()
        .chain(DSL_COVERED_DECLARATION_ONLY)
    {
        let def = registry.get(name).expect("tool should be registered");
        assert_eq!(
            def.executor_state,
            ToolExecutorState::DeclarationOnly,
            "Tool '{}' should be DeclarationOnly but is {:?}",
            name,
            def.executor_state
        );
    }
}

#[test]
fn cert_default_tools_are_executable() {
    let registry = tool_registry::get_registry();
    for name in CANONICAL_DEFAULT_TOOLS {
        let def = registry.get(name).expect("tool should be registered");
        assert_eq!(
            def.executor_state,
            ToolExecutorState::Executable,
            "Default tool '{}' should be Executable but is {:?}",
            name,
            def.executor_state
        );
    }
}

// ── Search & Discovery Tests ──

#[test]
fn cert_tool_search_excludes_declaration_only() {
    // tool_search's search_and_convert() must not return DeclarationOnly tools.
    let registry = tool_registry::get_registry();

    for name in INERT_DECLARATION_ONLY {
        // Search by the tool's own name
        let results = registry.search_and_convert(name);
        let found = results.iter().any(|t| t.function.name == *name);
        assert!(
            !found,
            "Inert DeclarationOnly tool '{}' should not be discoverable via \
             tool_search. search_and_convert() returned it.",
            name
        );
    }

    for name in DSL_COVERED_DECLARATION_ONLY {
        let results = registry.search_and_convert(name);
        let found = results.iter().any(|t| t.function.name == *name);
        assert!(
            !found,
            "DSL-covered DeclarationOnly tool '{}' should not be discoverable \
             via tool_search. search_and_convert() returned it.",
            name
        );
    }
}

#[test]
fn cert_search_returns_core_tools() {
    let registry = tool_registry::get_registry();
    let results = registry.search_and_convert("read file");
    let found_read = results.iter().any(|t| t.function.name == "read");
    assert!(
        found_read,
        "search_and_convert('read file') should return the 'read' tool"
    );
}

// ── Provider Executor Mapping Tests ──

#[test]
fn cert_provider_executors_match_default_tools() {
    // Every provider executor name should correspond to a default tool.
    // This catches drift between tool_calling.rs and the registry.
    let tools = tool_registry::build_current_tools();
    let default_names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();

    for executor_name in PROVIDER_EXECUTORS {
        assert!(
            default_names.contains(executor_name),
            "Provider executor '{}' exists in tool_calling.rs but the tool is \
             not in default_tools(). Either add it to the registry or remove \
             the dead executor code.",
            executor_name
        );
    }
}

// ── DSL & Adapter Coverage Tests ──

#[test]
fn cert_dsl_commands_have_action_variants() {
    // Verify that all DSL commands have corresponding AgentAction variants.
    // This is a compile-time-ish check that the DSL grammar and execution
    // paths are in sync.
    use crate::AgentAction;

    // These should all compile and be constructible:
    let _read = AgentAction::ReadFile {
        path: "test".into(),
    };
    let _list = AgentAction::ListFiles {
        path: "test".into(),
        depth: 1,
    };
    let _search = AgentAction::SearchText {
        q: "test".into(),
        path: "test".into(),
    };
    let _sym = AgentAction::SearchSymbol {
        q: "test".into(),
        path: "test".into(),
    };
    let _edit = AgentAction::EditFile {
        path: "test".into(),
        old: "a".into(),
        new: "b".into(),
    };
    let _cmd = AgentAction::RunCommand {
        command: "ls".into(),
    };
    let _ask = AgentAction::Ask {
        question: "?".into(),
    };
    let _done = AgentAction::Done {
        summary: "ok".into(),
    };
}

#[test]
fn cert_parse_each_dsl_command() {
    use crate::parse_action_dsl;
    use crate::ParseContext;

    let ctx = ParseContext {
        dsl_variant: "action",
        line: None,
    };

    // Each DSL command must parse successfully
    assert!(parse_action_dsl(r#"R path="Cargo.toml""#, &ctx).is_ok());
    assert!(parse_action_dsl(r#"L path="src""#, &ctx).is_ok());
    assert!(parse_action_dsl(r#"S q="fn main" path="src""#, &ctx).is_ok());
    assert!(parse_action_dsl(r#"Y q="parse_action" path="src""#, &ctx).is_ok());
    assert!(parse_action_dsl("E path=\"test.txt\"\n---OLD\na\n---NEW\nb\n---END", &ctx).is_ok());
    assert!(parse_action_dsl("X\necho hi\n---END", &ctx).is_ok());
    assert!(parse_action_dsl("ASK\nwhat?\n---END", &ctx).is_ok());
    assert!(parse_action_dsl("DONE\nok\n---END", &ctx).is_ok());
    assert!(parse_action_dsl(r#"DONE summary="ok""#, &ctx).is_ok());
}

#[test]
fn cert_xml_adapter_covers_core_tools() {
    // The XML→AgentAction adapter must cover the core tools that have
    // provider-style tool call representations.
    use crate::parse_tool_call_xml;

    let read = parse_tool_call_xml(
        r#"<tool_call>{"name":"read","arguments":{"path":"Cargo.toml"}}</tool_call>"#,
    );
    assert!(read.is_some(), "XML adapter should parse 'read' tool calls");

    let shell = parse_tool_call_xml(
        r#"<tool_call>{"name":"shell","arguments":{"command":"cargo test"}}</tool_call>"#,
    );
    assert!(
        shell.is_some(),
        "XML adapter should parse 'shell' tool calls"
    );

    let search = parse_tool_call_xml(
        r#"<tool_call>{"name":"search","arguments":{"pattern":"fn main","path":"src"}}</tool_call>"#,
    );
    assert!(
        search.is_some(),
        "XML adapter should parse 'search' tool calls"
    );

    let edit = parse_tool_call_xml(
        r#"<tool_call>{"name":"edit","arguments":{"path":"file.txt","old":"foo","new":"bar"}}</tool_call>"#,
    );
    assert!(edit.is_some(), "XML adapter should parse 'edit' tool calls");

    let respond = parse_tool_call_xml(
        r#"<tool_call>{"name":"respond","arguments":{"text":"Done."}}</tool_call>"#,
    );
    assert!(
        respond.is_some(),
        "XML adapter should parse 'respond' tool calls"
    );
}

// ── XML→Action Adapter Mapping Coverage ──

#[test]
fn cert_adapter_maps_read_to_readfile() {
    use crate::parse_tool_call_xml;
    use crate::AgentAction;

    let result = parse_tool_call_xml(
        r#"<tool_call>{"name":"read","arguments":{"path":"src/main.rs"}}</tool_call>"#,
    )
    .unwrap();
    assert!(matches!(result, AgentAction::ReadFile { .. }));
}

#[test]
fn cert_adapter_maps_shell_to_runcommand() {
    use crate::parse_tool_call_xml;
    use crate::AgentAction;

    let result = parse_tool_call_xml(
        r#"<tool_call>{"name":"shell","arguments":{"command":"cargo build"}}</tool_call>"#,
    )
    .unwrap();
    assert!(matches!(result, AgentAction::RunCommand { .. }));
}

#[test]
fn cert_adapter_maps_respond_to_done() {
    use crate::parse_tool_call_xml;
    use crate::AgentAction;

    let result = parse_tool_call_xml(
        r#"<tool_call>{"name":"respond","arguments":{"text":"ok"}}</tool_call>"#,
    )
    .unwrap();
    assert!(matches!(result, AgentAction::Done { .. }));
}

#[test]
fn cert_adapter_maps_search_to_searchtext() {
    use crate::parse_tool_call_xml;
    use crate::AgentAction;

    let result = parse_tool_call_xml(
        r#"<tool_call>{"name":"search","arguments":{"pattern":"fn main","path":"src"}}</tool_call>"#,
    )
    .unwrap();
    assert!(matches!(result, AgentAction::SearchText { .. }));
}

#[test]
fn cert_adapter_maps_edit_to_editfile() {
    use crate::parse_tool_call_xml;
    use crate::AgentAction;

    let result = parse_tool_call_xml(
        r#"<tool_call>{"name":"edit","arguments":{"path":"file.txt","old":"foo","new":"bar"}}</tool_call>"#,
    )
    .unwrap();
    assert!(matches!(result, AgentAction::EditFile { .. }));
}

#[test]
fn cert_adapter_rejects_nonexistent_tools() {
    use crate::parse_tool_call_xml;

    let result =
        parse_tool_call_xml(r#"<tool_call>{"name":"nonexistent","arguments":{}}</tool_call>"#);
    assert!(
        result.is_none(),
        "XML adapter should reject unknown tool names"
    );
}

// ── Evidence & Safety Coverage Tests ──

#[test]
fn cert_shell_has_safety_policy() {
    let registry = tool_registry::get_registry();
    let shell = registry.get("shell").expect("shell should be registered");
    assert!(
        shell.requires_permission,
        "shell tool must require permission (safety policy)"
    );
    assert!(
        shell
            .risks
            .contains(&elma_tools::registry::ToolRisk::ExternalProcess),
        "shell tool must have ExternalProcess risk"
    );
    assert!(
        shell.check_fn.is_some(),
        "shell tool must have a prerequisite check function"
    );
}

#[test]
fn cert_edit_requires_prior_read() {
    let registry = tool_registry::get_registry();
    let edit = registry.get("edit").expect("edit should be registered");
    assert!(
        edit.requires_prior_read,
        "edit tool must require prior read before execution"
    );
}

#[test]
fn cert_concurrency_safe_tools_flagged() {
    let registry = tool_registry::get_registry();
    for name in &[
        "read",
        "search",
        "ls",
        "glob",
        "respond",
        "summary",
        "tool_search",
    ] {
        let tool = registry
            .get(name)
            .unwrap_or_else(|| panic!("{} should be registered", name));
        assert!(
            tool.concurrency_safe,
            "ReadOnly/conversation tool '{}' should be marked concurrency_safe",
            name
        );
    }
}

// ── Intel Unit DSL Profile Certification ──

/// Migrated intel units that must use compact DSL output format.
/// Each entry maps (profile_name, expected_dsl_command).
const MIGRATED_INTEL_DSL_UNITS: &[(&str, &str)] = &[
    ("action_selector", "SELECT"),
    ("turn_summary", "TURN"),
    ("evidence_need_assessor", "ASSESS"),
];

#[test]
fn cert_migrated_intel_profiles_contain_dsl_commands() {
    // Every migrated intel unit's default config must contain the expected DSL
    // command in its system prompt. If this test fails, a profile was regressed
    // to prose-only output, which would cause INVALID_DSL fallback at runtime.
    let config_dir = "config/defaults";
    for (profile_name, expected_dsl_command) in MIGRATED_INTEL_DSL_UNITS {
        let path = format!("{}/{}.toml", config_dir, profile_name);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path, e));
        assert!(
            content.contains(expected_dsl_command),
            "Profile '{}' ({}) system_prompt does not contain expected DSL command '{}'. \
             This profile must guide the model to produce compact DSL output, not prose. \
             Add a DSL format instruction (e.g., 'Output exactly one {} line.') to the system_prompt.",
            profile_name, path, expected_dsl_command, expected_dsl_command
        );
    }
}
