//! @efficiency-role: util-pure
//! Retry policy with exponential backoff for LLM API calls.

use std::time::Duration;

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
    pub jitter: bool,
}

impl RetryPolicy {
    pub fn for_streaming() -> Self {
        Self { max_retries: 3, base_delay_ms: 1000, max_delay_ms: 30_000, backoff_multiplier: 2.0, jitter: true }
    }

    pub fn for_one_shot() -> Self {
        Self { max_retries: 2, base_delay_ms: 500, max_delay_ms: 10_000, backoff_multiplier: 2.0, jitter: true }
    }

    pub fn get_delay(&self, attempt: u32) -> Duration {
        let delay = self.base_delay_ms as f64 * self.backoff_multiplier.powi(attempt as i32);
        let delay = delay.min(self.max_delay_ms as f64);
        let delay = if self.jitter {
            // Use a simple deterministic jitter based on attempt number
            let jitter_factor = 0.5 + ((attempt as f64 * 0.317).fract() * 0.5);
            delay * jitter_factor
        } else {
            delay
        };
        Duration::from_millis(delay as u64)
    }
}
