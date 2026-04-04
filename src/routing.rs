//! @efficiency-role: orchestrator
//!
//! Routing Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - routing_parse: JSON and markdown parsing
//! - routing_calc: Routing calculations and distributions
//! - routing_infer: Router inference functions

pub use crate::routing_calc::*;
pub use crate::routing_infer::*;
pub use crate::routing_parse::*;
