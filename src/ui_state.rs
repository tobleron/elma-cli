//! @efficiency-role: util-pure
//!
//! UI - State Management

use crate::*;

static TRACE_LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static REASONING_DISPLAY: OnceLock<Mutex<(bool, bool)>> = OnceLock::new();
static JSON_OUTPUTTER_PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static FINAL_ANSWER_EXTRACTOR_PROFILE: OnceLock<Mutex<Option<Profile>>> = OnceLock::new();
static MODEL_BEHAVIOR_PROFILE: OnceLock<Mutex<Option<ModelBehaviorProfile>>> = OnceLock::new();

pub(crate) fn trace_log_state() -> &'static Mutex<Option<PathBuf>> {
    TRACE_LOG_PATH.get_or_init(|| Mutex::new(None))
}

pub(crate) fn reasoning_display_state() -> &'static Mutex<(bool, bool)> {
    REASONING_DISPLAY.get_or_init(|| Mutex::new((false, false)))
}

pub(crate) fn json_outputter_state() -> &'static Mutex<Option<Profile>> {
    JSON_OUTPUTTER_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn final_answer_extractor_state() -> &'static Mutex<Option<Profile>> {
    FINAL_ANSWER_EXTRACTOR_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn model_behavior_state() -> &'static Mutex<Option<ModelBehaviorProfile>> {
    MODEL_BEHAVIOR_PROFILE.get_or_init(|| Mutex::new(None))
}

pub(crate) fn set_trace_log_path(path: Option<PathBuf>) {
    if let Ok(mut slot) = trace_log_state().lock() {
        *slot = path;
    }
}

pub(crate) fn set_reasoning_display(show_terminal: bool, no_color: bool) {
    if let Ok(mut slot) = reasoning_display_state().lock() {
        *slot = (show_terminal, no_color);
    }
}

pub(crate) fn set_json_outputter_profile(profile: Option<Profile>) {
    if let Ok(mut slot) = json_outputter_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn set_final_answer_extractor_profile(profile: Option<Profile>) {
    if let Ok(mut slot) = final_answer_extractor_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn set_model_behavior_profile(profile: Option<ModelBehaviorProfile>) {
    if let Ok(mut slot) = model_behavior_state().lock() {
        *slot = profile;
    }
}

pub(crate) fn current_model_behavior_profile() -> Option<ModelBehaviorProfile> {
    model_behavior_state().lock().ok()?.clone()
}

pub(crate) fn json_outputter_profile() -> Option<Profile> {
    json_outputter_state().lock().ok()?.clone()
}

pub(crate) fn final_answer_extractor_profile() -> Option<Profile> {
    final_answer_extractor_state().lock().ok()?.clone()
}
