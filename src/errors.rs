//! @efficiency-role: data-model
//! Structured error types for Elma CLI.
//!
//! Provides typed error enums for each subsystem, enabling
//! structured recovery, model-facing guidance, and stop-policy
//! classification without string matching.

use thiserror::Error;

/// Top-level error for recoverable failures across all subsystems.
#[derive(Error, Debug)]
pub enum ElmaError {
    #[error("Tool execution failed: {0}")]
    Tool(#[from] ToolError),

    #[error("Model response error: {0}")]
    Model(#[from] ModelError),

    #[error("JSON parse error: {0}")]
    JsonParse(#[from] JsonParseError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),
}

/// Recoverable tool execution errors.
#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Invalid arguments: {field}: {reason}")]
    InvalidArgs { field: String, reason: String },

    #[error("Command blocked by preflight: {reason}")]
    PreflightBlocked { reason: String },

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },

    #[error("Execution failed: exit_code={exit_code:?}, timed_out={timed_out}")]
    ExecutionFailed { exit_code: Option<i32>, timed_out: bool },
}

/// Model response errors.
#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Empty response: model returned no content")]
    EmptyResponse,

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Timeout after {duration}s")]
    Timeout { duration: u64 },

    #[error("API error: {status} {body}")]
    ApiError { status: u16, body: String },
}

/// JSON parsing errors from model output.
#[derive(Error, Debug)]
pub enum JsonParseError {
    #[error("Not valid JSON after repair pipeline")]
    UnableToParse,

    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Type mismatch for field '{field}': expected {expected}")]
    TypeMismatch { field: String, expected: String },
}

/// Validation errors.
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Path traversal detected: {path}")]
    PathTraversal { path: String },

    #[error("Required configuration missing: {field}")]
    MissingConfig { field: String },
}
