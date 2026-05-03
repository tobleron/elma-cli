pub mod ui_autocomplete;
pub mod ui_chat;
pub mod ui_colors;
pub mod ui_context_bar;
pub mod ui_coordinator_status;
pub mod ui_diff;
pub mod ui_effort;
pub mod ui_input;
pub mod ui_interact;
pub mod ui_layout;
pub mod ui_markdown;
pub mod ui_modal;
pub mod ui_modal_search;
pub mod ui_model_picker;
pub mod ui_progress;
pub mod ui_spinner;
pub mod ui_state;
pub mod ui_syntax;
pub mod ui_terminal;
pub mod ui_theme;
pub mod ui_trace;
pub mod ui_wrap;

// Re-export key functions so they're available via `use crate::*;`
pub(crate) use ui_chat::*;
pub(crate) use ui_state::*;
pub(crate) use ui_theme::*;
pub(crate) use ui_trace::*;
