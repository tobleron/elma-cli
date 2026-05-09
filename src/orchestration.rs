//! @efficiency-role: orchestrator
//!
//! Orchestration Module (De-bloated)
//!
//! This module now re-exports orchestration functions from specialized sub-modules:
//! - orchestration_core: Core orchestration functions

// Re-export all orchestration functions
pub use crate::orchestration_core::*;
