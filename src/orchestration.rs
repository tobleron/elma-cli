//! @efficiency-role: service-orchestrator
//!
//! Orchestration Module (De-bloated)
//!
//! This module now re-exports orchestration functions from specialized sub-modules:
//! - orchestration_core: Core orchestration functions
//! - orchestration_loop: Autonomous loop execution
//! - orchestration_retry: Retry orchestration and meta-review
//! - orchestration_planning: Planning prior and hierarchical decomposition

// Re-export all orchestration functions
pub use crate::orchestration_core::*;
pub use crate::orchestration_loop::*;
pub use crate::orchestration_planning::*;
pub use crate::orchestration_retry::*;
