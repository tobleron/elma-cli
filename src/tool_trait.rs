//! @efficiency-role: infra-adapter
//!
//! Unified Tool trait and executor for all tools (Task 655).
//! Defines the core abstractions for tool registration, validation, and execution.

use std::collections::HashMap;
use std::time::Instant;

/// Result of a single tool execution.
#[derive(Debug, Clone)]
pub(crate) struct ToolResult {
    pub(crate) success: bool,
    pub(crate) output: String,
    pub(crate) error: Option<String>,
    pub(crate) duration_ms: u64,
    pub(crate) tool_name: String,
}

/// Policy governing how a tool may be used.
#[derive(Debug, Clone)]
pub(crate) struct ToolPolicy {
    pub(crate) requires_permission: bool,
    pub(crate) allow_in_background: bool,
    pub(crate) timeout_seconds: u64,
    pub(crate) max_output_bytes: u64,
}

/// The canonical trait for all tools in the system.
pub(crate) trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn execute(&self, args: &HashMap<String, String>) -> ToolResult;
    fn policy(&self) -> ToolPolicy;
    fn validate_args(&self, args: &HashMap<String, String>) -> Result<(), Vec<String>>;
}

/// Registry and executor for registered tools.
pub(crate) struct ToolExecutor {
    tools: HashMap<&'static str, Box<dyn Tool>>,
}

impl ToolExecutor {
    pub(crate) fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub(crate) fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name();
        self.tools.insert(name, tool);
    }

    pub(crate) fn execute(&self, name: &str, args: &HashMap<String, String>) -> Option<ToolResult> {
        self.tools.get(name).map(|tool| tool.execute(args))
    }

    pub(crate) fn available_tools(&self) -> Vec<(&str, &str)> {
        self.tools
            .iter()
            .map(|(name, tool)| (*name, tool.description()))
            .collect()
    }

    pub(crate) fn is_available(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

// ── Test implementations ────────────────────────────────────────────────────

pub(crate) struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read the contents of a file"
    }

    fn execute(&self, args: &HashMap<String, String>) -> ToolResult {
        let start = Instant::now();
        let path = args.get("path").cloned().unwrap_or_default();

        match std::fs::read_to_string(&path) {
            Ok(content) => ToolResult {
                success: true,
                output: content,
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
                tool_name: self.name().to_string(),
            },
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
                tool_name: self.name().to_string(),
            },
        }
    }

    fn policy(&self) -> ToolPolicy {
        ToolPolicy {
            requires_permission: false,
            allow_in_background: true,
            timeout_seconds: 30,
            max_output_bytes: 10_485_760,
        }
    }

    fn validate_args(&self, args: &HashMap<String, String>) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !args.contains_key("path") {
            errors.push("Missing required argument: path".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

pub(crate) struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute a shell command"
    }

    fn execute(&self, args: &HashMap<String, String>) -> ToolResult {
        let start = Instant::now();
        let command = args.get("command").cloned().unwrap_or_default();

        let policy = self.policy();
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(&command)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                let truncated = stdout.len() > policy.max_output_bytes as usize;

                let mut output = stdout;
                if truncated {
                    output.truncate(policy.max_output_bytes as usize);
                    output.push_str("\n--- output truncated ---");
                }

                ToolResult {
                    success: out.status.success(),
                    output,
                    error: if stderr.is_empty() { None } else { Some(stderr) },
                    duration_ms: start.elapsed().as_millis() as u64,
                    tool_name: self.name().to_string(),
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e.to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
                tool_name: self.name().to_string(),
            },
        }
    }

    fn policy(&self) -> ToolPolicy {
        ToolPolicy {
            requires_permission: true,
            allow_in_background: false,
            timeout_seconds: 120,
            max_output_bytes: 1_048_576,
        }
    }

    fn validate_args(&self, args: &HashMap<String, String>) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if !args.contains_key("command") {
            errors.push("Missing required argument: command".to_string());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_tool_name() {
        let tool = ReadTool;
        assert_eq!(tool.name(), "read");
    }

    #[test]
    fn test_read_tool_description() {
        let tool = ReadTool;
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_read_tool_validate_missing_path() {
        let tool = ReadTool;
        let args = HashMap::new();
        assert!(tool.validate_args(&args).is_err());
    }

    #[test]
    fn test_read_tool_validate_with_path() {
        let tool = ReadTool;
        let mut args = HashMap::new();
        args.insert("path".to_string(), "/tmp/test.txt".to_string());
        assert!(tool.validate_args(&args).is_ok());
    }

    #[test]
    fn test_bash_tool_name() {
        let tool = BashTool;
        assert_eq!(tool.name(), "bash");
    }

    #[test]
    fn test_bash_tool_policy() {
        let tool = BashTool;
        let policy = tool.policy();
        assert!(policy.requires_permission);
        assert!(!policy.allow_in_background);
        assert_eq!(policy.timeout_seconds, 120);
    }

    #[test]
    fn test_read_tool_policy() {
        let tool = ReadTool;
        let policy = tool.policy();
        assert!(!policy.requires_permission);
        assert!(policy.allow_in_background);
        assert_eq!(policy.timeout_seconds, 30);
    }

    #[test]
    fn test_bash_tool_validate_missing_command() {
        let tool = BashTool;
        let args = HashMap::new();
        assert!(tool.validate_args(&args).is_err());
    }

    #[test]
    fn test_tool_executor_register_and_execute() {
        let mut executor = ToolExecutor::new();
        executor.register(Box::new(ReadTool));
        executor.register(Box::new(BashTool));

        assert!(executor.is_available("read"));
        assert!(executor.is_available("bash"));
        assert!(!executor.is_available("nonexistent"));
    }

    #[test]
    fn test_tool_executor_available_tools() {
        let mut executor = ToolExecutor::new();
        executor.register(Box::new(ReadTool));

        let available = executor.available_tools();
        assert!(available.contains(&("read", "Read the contents of a file")));
    }

    #[test]
    fn test_tool_executor_execute_unknown() {
        let executor = ToolExecutor::new();
        let args = HashMap::new();
        assert!(executor.execute("unknown", &args).is_none());
    }

    #[test]
    fn test_tool_result_fields() {
        let result = ToolResult {
            success: true,
            output: "hello".to_string(),
            error: None,
            duration_ms: 42,
            tool_name: "test".to_string(),
        };
        assert!(result.success);
        assert_eq!(result.output, "hello");
        assert_eq!(result.duration_ms, 42);
        assert_eq!(result.tool_name, "test");
    }

    #[test]
    fn test_tool_policy_fields() {
        let policy = ToolPolicy {
            requires_permission: true,
            allow_in_background: false,
            timeout_seconds: 60,
            max_output_bytes: 1024,
        };
        assert!(policy.requires_permission);
        assert!(!policy.allow_in_background);
        assert_eq!(policy.timeout_seconds, 60);
        assert_eq!(policy.max_output_bytes, 1024);
    }
}
