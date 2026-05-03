//! @efficiency-role: data-model
//! Context window budget tracking with user visibility.
//!
//! Tracks token usage breakdown and provides warnings when approaching limits.

use crate::token_counter::count_tokens;

/// Breakdown of context window usage.
#[derive(Debug, Clone, Default)]
pub struct ContextBudget {
    pub system_prompt_tokens: u64,
    pub conversation_tokens: u64,
    pub tool_results_tokens: u64,
    pub max_tokens: u64,
    pub current_turn: u32,
}

impl ContextBudget {
    pub fn new(max_tokens: u64) -> Self {
        Self {
            max_tokens,
            ..Default::default()
        }
    }

    pub fn total_used(&self) -> u64 {
        self.system_prompt_tokens + self.conversation_tokens + self.tool_results_tokens
    }

    pub fn usage_pct(&self) -> f64 {
        if self.max_tokens == 0 {
            return 0.0;
        }
        (self.total_used() as f64 / self.max_tokens as f64) * 100.0
    }

    pub fn is_near_limit(&self, threshold_pct: f64) -> bool {
        self.usage_pct() >= threshold_pct
    }

    pub fn record_system_prompt(&mut self, text: &str) {
        self.system_prompt_tokens = count_tokens(text) as u64;
    }

    pub fn record_conversation(&mut self, text: &str) {
        self.conversation_tokens = count_tokens(text) as u64;
    }

    pub fn record_tool_result(&mut self, text: &str) {
        self.tool_results_tokens = count_tokens(text) as u64;
    }

    pub fn summary(&self) -> String {
        format!(
            "Context: {}/{} tokens ({:.0}%) — {} turns",
            self.total_used(),
            self.max_tokens,
            self.usage_pct(),
            self.current_turn,
        )
    }
}
