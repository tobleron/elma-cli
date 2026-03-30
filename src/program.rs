//! @efficiency-role: domain-logic
//!
//! Program Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - program_steps: Step helpers and JSON serialization
//! - program_policy: Command policy and evaluation
//! - program_utils: Utilities for command execution

pub use crate::program_policy::*;
pub use crate::program_steps::*;
pub use crate::program_utils::*;
