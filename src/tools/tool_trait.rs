use crate::tools::types::ToolExecutionResult;
use std::fmt::Debug;

/// Metadata for a tool, used for discovery and prompt generation.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub parameters_schema: String, // JSON Schema
}

/// The canonical interface for all tools in Elma.
#[async_trait::async_trait]
pub trait Tool: Send + Sync + Debug {
    /// Returns the metadata for this tool.
    fn info(&self) -> ToolInfo;

    /// Executes the tool.
    ///
    /// # Arguments
    /// * `call_id` - Unique identifier for this specific tool call.
    /// * `arguments` - JSON string of arguments.
    /// * `context` - The current execution context (can include user info, session, etc.)
    async fn run(
        &self,
        call_id: &str,
        arguments: &str,
        context: &serde_json::Value,
    ) -> ToolExecutionResult;
}
