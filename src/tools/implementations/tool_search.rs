use crate::tools::types::{ToolExecutionResult};

pub fn exec_tool_search(
    av: &serde_json::Value,
    call_id: &str,
    _tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let query = av["query"].as_str().unwrap_or("").to_string();
    if query.is_empty() {
        let error_msg = "Error: query is required".to_string();
        return ToolExecutionResult::new_failed(call_id, "tool_search", &error_msg);
    }

    let registry = crate::tool_registry::get_registry();
    let tools = registry.search_and_convert(&query);
    
    if tools.is_empty() {
        let content = format!("No tools found matching: '{}'", query);
        return ToolExecutionResult::new_ok(call_id, "tool_search", &content);
    }

    // Mark tools as discovered so they become available in future requests
    let tool_names = registry.get_tool_names(&query);
    crate::tool_registry::mark_discovered(&tool_names);

    // Format tool definitions as JSON for the model
    let tools_json = serde_json::to_string_pretty(&tools).unwrap_or_default();
    let content = format!(
        "Found {} tool(s) matching '{}':\n\n{}\n\nThese tools are now loaded and available for use. You can call them directly in your next response.",
        tools.len(),
        query,
        tools_json
    );

    ToolExecutionResult::new_ok(call_id, "tool_search", &content)
}
