//! @efficiency-role: data-model
//!
//! Defaults Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - defaults_core: Core and reviewer configurations
//! - defaults_router: Router, planner, and judge configurations
//! - defaults_evidence: Evidence and tune configurations

pub use crate::defaults_core::*;
pub use crate::defaults_evidence::*;
pub use crate::defaults_router::*;
