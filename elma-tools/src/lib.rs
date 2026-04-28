//! Elma Tools — standalone tool definition and discovery crate.
//!
//! Provides `DynamicToolRegistry`, tool definitions, capability search,
//! and prerequisite checking (via `check_fn`). Extracted from the main
//! `elma-cli` binary for independent testing, extensibility, and sharing.

pub(crate) mod tools;
pub mod registry;
pub mod types;

// Re-export commonly used types
pub use registry::{
    build_current_tools, build_tools_for_context, get_discovered, mark_discovered,
    mark_discovered_filtered, DynamicToolRegistry, RegistryBuilder, ToolDefinitionExt,
};
pub use types::{ToolDefinition, ToolFunction};
