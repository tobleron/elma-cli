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
}

impl ToolArgSchema {
    pub fn new(name: &str) -> Self {
        Self {
            tool_name: name.to_string(),
            required: Vec::new(),
            optional: Vec::new(),
            allow_extra_fields: false,
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
                .required("command", ArgType::Command),
        ),
        "read" => Some(
            ToolArgSchema::new("read")
                .required("path", ArgType::RelPath),
        ),
        "edit" => Some(
            ToolArgSchema::new("edit")
                .required("path", ArgType::RelPath)
                .required("old_string", ArgType::String_)
                .required("new_string", ArgType::String_),
        ),
        "write" => Some(
            ToolArgSchema::new("write")
                .required("path", ArgType::RelPath)
                .required("content", ArgType::FileContent),
        ),
        "search" => Some(
            ToolArgSchema::new("search")
                .required("pattern", ArgType::String_)
                .optional("path", ArgType::RelPath),
        ),
        "glob" => Some(
            ToolArgSchema::new("glob")
                .required("pattern", ArgType::String_)
                .optional("path", ArgType::RelPathOrGlob),
        ),
        "stat" => Some(
            ToolArgSchema::new("stat")
                .required("path", ArgType::RelPath),
        ),
        "ls" => Some(
            ToolArgSchema::new("ls")
                .optional("path", ArgType::RelPath)
                .optional("depth", ArgType::UIntRange(1, 5)),
        ),
        "mkdir" => Some(
            ToolArgSchema::new("mkdir")
                .required("path", ArgType::RelPath),
        ),
        "copy" => Some(
            ToolArgSchema::new("copy")
                .required("source", ArgType::RelPath)
                .required("destination", ArgType::RelPath),
        ),
        "move" => Some(
            ToolArgSchema::new("move")
                .required("source", ArgType::RelPath)
                .required("destination", ArgType::RelPath),
        ),
        "trash" => Some(
            ToolArgSchema::new("trash")
                .required("path", ArgType::RelPath),
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
