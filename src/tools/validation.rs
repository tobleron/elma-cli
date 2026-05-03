//! @efficiency-role: domain-logic
//! Tool argument schema validation.
//!
//! Provides structured validation of model-generated tool arguments
//! before they reach tool executors. Each tool defines an `ArgSchema`
//! specifying required and optional fields, types, and constraints.

use crate::tools::ToolExecutionResult;

/// Argument type for tool parameter validation.
#[derive(Debug, Clone)]
pub enum ArgType {
    /// Workspace-relative path, no `..`, no absolute prefix
    RelPath,
    /// Relative path or glob pattern
    RelPathOrGlob,
    /// Non-empty string
    String_,
    /// Optional string (allows null/empty)
    OptionalString,
    /// Positive integer
    UInt,
    /// Bounded positive integer (min, max)
    UIntRange(u64, u64),
    /// Boolean
    Bool,
    /// Array of typed elements
    ArrayOf(Box<ArgType>),
    /// Any valid JSON value
    JsonValue,
    /// Shell command string (basic safety check)
    Command,
    /// File content with size cap
    FileContent,
}

impl ArgType {
    pub fn label(&self) -> &str {
        match self {
            ArgType::RelPath => "path (string)",
            ArgType::RelPathOrGlob => "path-or-glob (string)",
            ArgType::String_ => "string",
            ArgType::OptionalString => "optional-string",
            ArgType::UInt => "non-negative-integer",
            ArgType::UIntRange(_, _) => "integer (bounded range)",
            ArgType::Bool => "boolean",
            ArgType::ArrayOf(_) => "array",
            ArgType::JsonValue => "json-value",
            ArgType::Command => "shell-command (string)",
            ArgType::FileContent => "file-content (string)",
        }
    }
}

/// Result of validating tool arguments against a schema.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub ok: bool,
    pub field_errors: Vec<FieldError>,
}

#[derive(Debug, Clone)]
pub struct FieldError {
    pub field: String,
    pub error: String,
    pub value: Option<String>,
}

impl ValidationResult {
    pub fn ok() -> Self {
        Self {
            ok: true,
            field_errors: Vec::new(),
        }
    }

    pub fn error(field: &str, error: &str) -> Self {
        Self {
            ok: false,
            field_errors: vec![FieldError {
                field: field.to_string(),
                error: error.to_string(),
                value: None,
            }],
        }
    }
}

/// Schema definition for a tool's arguments.
pub struct ToolArgSchema {
    pub tool_name: String,
    pub required: Vec<(String, ArgType)>,
    pub optional: Vec<(String, ArgType)>,
    pub allow_extra_fields: bool,
    /// Human-readable description of each required field (e.g. "path to file")
    pub help_text: Option<String>,
    /// Example of a valid tool call JSON
    pub usage_example: Option<String>,
}

impl ToolArgSchema {
    pub fn new(name: &str) -> Self {
        Self {
            tool_name: name.to_string(),
            required: Vec::new(),
            optional: Vec::new(),
            allow_extra_fields: false,
            help_text: None,
            usage_example: None,
        }
    }

    pub fn required(mut self, field: &str, arg_type: ArgType) -> Self {
        self.required.push((field.to_string(), arg_type));
        self
    }

    pub fn optional(mut self, field: &str, arg_type: ArgType) -> Self {
        self.optional.push((field.to_string(), arg_type));
        self
    }

    pub fn no_extra_fields(mut self) -> Self {
        self.allow_extra_fields = false;
        self
    }

    pub fn help(mut self, text: &str) -> Self {
        self.help_text = Some(text.to_string());
        self
    }

    pub fn example(mut self, json: &str) -> Self {
        self.usage_example = Some(json.to_string());
        self
    }

    /// Format a human-readable narrative of the schema for error messages.
    pub fn format_schema_narrative(&self) -> String {
        let mut parts = Vec::new();
        for (field, at) in &self.required {
            parts.push(format!("{} ({}) [required]", field, at.label()));
        }
        for (field, at) in &self.optional {
            parts.push(format!("{} ({}) [optional]", field, at.label()));
        }
        let schema = parts.join(", ");
        let mut msg = format!("Tool '{}' expects: {}", self.tool_name, schema);
        if let Some(ref ex) = self.usage_example {
            msg.push_str(&format!(". Example: {}", ex));
        }
        msg
    }

    /// Format a validation error with schema context for the model.
    pub fn format_error_with_schema(&self, error_msg: &str) -> String {
        let mut msg = format!("Argument validation failed: {}", error_msg);
        msg.push_str(&format!("\n\n{}", self.format_schema_narrative()));
        if let Some(ref help) = self.help_text {
            msg.push_str(&format!("\n\nHint: {}", help));
        }
        msg
    }

    pub fn validate(&self, args: &serde_json::Value) -> ValidationResult {
        for (field, arg_type) in &self.required {
            match args.get(field) {
                None | Some(serde_json::Value::Null) => {
                    return ValidationResult {
                        ok: false,
                        field_errors: vec![FieldError {
                            field: field.clone(),
                            error: format!("required field '{}' is missing", field),
                            value: None,
                        }],
                    };
                }
                Some(val) => {
                    if let Err(e) = check_type(val, arg_type) {
                        return ValidationResult {
                            ok: false,
                            field_errors: vec![FieldError {
                                field: field.clone(),
                                error: e,
                                value: Some(val.to_string()),
                            }],
                        };
                    }
                }
            }
        }
        ValidationResult::ok()
    }
}

pub fn get_tool_schema(tool_name: &str) -> Option<ToolArgSchema> {
    match tool_name {
        "shell" => Some(
            ToolArgSchema::new("shell")
                .required("command", ArgType::Command)
                .help("The shell command to execute. Use shell for terminal operations like file listing, git, compilation, etc.")
                .example(r#"{"command": "ls -la src/"}""#),
        ),
        "read" => Some(
            ToolArgSchema::new("read")
                .required("filePath", ArgType::RelPath)
                .optional("offset", ArgType::UInt)
                .optional("limit", ArgType::UInt)
                .help("Read the contents of a file. Provide the filePath (relative to workspace root). Optionally specify offset (line number to start from) and limit (max lines to read).")
                .example(r#"{"filePath": "src/main.rs", "limit": 50}"#),
        ),
        "edit" => Some(
            ToolArgSchema::new("edit")
                .required("path", ArgType::RelPath)
                .required("old_string", ArgType::String_)
                .required("new_string", ArgType::String_)
                .help("Edit a file by finding old_string and replacing it with new_string. Only the first occurrence is replaced.")
                .example(r#"{"path": "src/main.rs", "old_string": "foo()", "new_string": "bar()"}"#),
        ),
        "write" => Some(
            ToolArgSchema::new("write")
                .required("path", ArgType::RelPath)
                .required("content", ArgType::FileContent)
                .help("Create or overwrite a file with the given content.")
                .example(r#"{"path": "src/hello.rs", "content": "fn main() {\n    println!(\"hi\");\n}"}"#),
        ),
        "search" => Some(
            ToolArgSchema::new("search")
                .required("pattern", ArgType::String_)
                .optional("path", ArgType::RelPath)
                .help("Search file contents using ripgrep. pattern is a regex. path narrows the search to a subdirectory.")
                .example(r#"{"pattern": "fn main", "path": "src/"}"#),
        ),
        "glob" => Some(
            ToolArgSchema::new("glob")
                .required("pattern", ArgType::String_)
                .optional("path", ArgType::RelPathOrGlob)
                .help("Find files matching a glob pattern (e.g., **/*.rs).")
                .example(r#"{"pattern": "**/*.rs", "path": "src"}"#),
        ),
        "stat" => Some(
            ToolArgSchema::new("stat")
                .required("path", ArgType::RelPath)
                .help("Get file metadata: size, permissions, modification time.")
                .example(r#"{"path": "Cargo.toml"}"#),
        ),
        "exists" => Some(
            ToolArgSchema::new("exists")
                .required("path", ArgType::RelPath)
                .optional("type", ArgType::String_)
                .help("Check if a file or directory exists. Provide path (relative to workspace root). Optionally specify type to check (file, dir, or any).")
                .example(r#"{"path": "project_tmp/GEMINI.md"}"#),
        ),
        "ls" => Some(
            ToolArgSchema::new("ls")
                .optional("path", ArgType::RelPath)
                .optional("depth", ArgType::UIntRange(1, 5))
                .help("List files and directories. Default depth is 2. Max is 5.")
                .example(r#"{"path": "src", "depth": 3}"#),
        ),
        "mkdir" => Some(
            ToolArgSchema::new("mkdir")
                .required("path", ArgType::RelPath)
                .help("Create a directory (including parent directories).")
                .example(r#"{"path": "src/foo/bar"}"#),
        ),
        "copy" => Some(
            ToolArgSchema::new("copy")
                .required("source", ArgType::RelPath)
                .required("destination", ArgType::RelPath)
                .help("Copy a file from source to destination.")
                .example(r#"{"source": "src/main.rs", "destination": "src/main.rs.bak"}"#),
        ),
        "move" => Some(
            ToolArgSchema::new("move")
                .required("source", ArgType::RelPath)
                .required("destination", ArgType::RelPath)
                .help("Move or rename a file.")
                .example(r#"{"source": "src/old.rs", "destination": "src/new.rs"}"#),
        ),
        "trash" => Some(
            ToolArgSchema::new("trash")
                .required("path", ArgType::RelPath)
                .help("Move a file to the trash. IRREVERSIBLE in some contexts.")
                .example(r#"{"path": "src/temp.rs"}"#),
        ),
        _ => None,
    }
}

fn check_type(val: &serde_json::Value, arg_type: &ArgType) -> Result<(), String> {
    match arg_type {
        ArgType::String_ => {
            val.as_str()
                .filter(|s| !s.is_empty())
                .map(|_| ())
                .ok_or_else(|| "expected a non-empty string".to_string())
        }
        ArgType::UInt => {
            val.as_u64()
                .map(|_| ())
                .ok_or_else(|| "expected a non-negative integer".to_string())
        }
        ArgType::Bool => {
            val.as_bool()
                .map(|_| ())
                .ok_or_else(|| "expected a boolean".to_string())
        }
        ArgType::RelPath => {
            let s = val.as_str().ok_or_else(|| "expected a string path".to_string())?;
            if s.starts_with('/') || s.contains("..") {
                return Err("absolute path or parent traversal not allowed".to_string());
            }
            Ok(())
        }
        ArgType::UIntRange(min, max) => {
            let n = val.as_u64().ok_or_else(|| "expected a non-negative integer".to_string())?;
            if n < *min || n > *max {
                return Err(format!("value {} out of range [{}, {}]", n, min, max));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}
