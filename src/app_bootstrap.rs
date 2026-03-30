//! @efficiency-role: orchestrator
//!
//! App Bootstrap Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - app_bootstrap_core: Core bootstrap function
//! - app_bootstrap_profiles: Profile loading and synchronization
//! - app_bootstrap_modes: Mode handling and banners

pub(crate) use crate::app_bootstrap_core::*;
pub(crate) use crate::app_bootstrap_profiles::*;
pub(crate) use crate::app_bootstrap_modes::*;
