//! @efficiency-role: data-model
//!
//! Budget Forecaster & Context Envelope Management (Task 653)
//!
//! Forecasts token/iteration budgets based on complexity level
//! and manages context envelopes for bounded execution.

use crate::complexity_gate::ComplexityLevel;

/// Budget envelope defining resource limits for a task.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BudgetEnvelope {
    pub max_tokens: u64,
    pub max_iterations: u32,
    pub max_context_window: u64,
    pub current_usage: u64,
    pub current_iteration: u32,
}

/// Forecasts token/iteration budgets and manages context envelopes.
///
/// Tracks cumulative token consumption and iteration count against
/// a configured context window maximum. Provides static methods to
/// produce envelopes appropriate for each complexity level.
#[derive(Debug, Clone)]
pub(crate) struct BudgetForecaster {
    context_max: u64,
    current_usage: u64,
    current_iteration: u32,
    max_iterations: u32,
}

impl BudgetEnvelope {
    pub(crate) fn remaining_tokens(&self) -> u64 {
        self.max_tokens.saturating_sub(self.current_usage)
    }

    pub(crate) fn remaining_iterations(&self) -> u32 {
        self.max_iterations.saturating_sub(self.current_iteration)
    }

    pub(crate) fn is_exhausted(&self) -> bool {
        self.current_usage >= self.max_tokens || self.current_iteration >= self.max_iterations
    }
}

impl BudgetForecaster {
    pub(crate) fn new(context_max: u64) -> Self {
        Self {
            context_max,
            current_usage: 0,
            current_iteration: 0,
            max_iterations: u32::MAX,
        }
    }

    pub(crate) fn forecast_for_complexity(level: &ComplexityLevel) -> BudgetEnvelope {
        let (max_tokens, max_iterations, context_window_ratio) = match level {
            ComplexityLevel::Direct => (1000, 3, 8),
            ComplexityLevel::Investigate => (4000, 8, 16),
            ComplexityLevel::Multistep => (8000, 15, 32),
            ComplexityLevel::OpenEnded => (16000, 30, 64),
        };
        BudgetEnvelope {
            max_tokens,
            max_iterations,
            max_context_window: max_tokens * context_window_ratio,
            current_usage: 0,
            current_iteration: 0,
        }
    }

    pub(crate) fn track_usage(&mut self, tokens: u64) {
        self.current_usage += tokens;
        self.current_iteration += 1;
    }

    pub(crate) fn is_exhausted(&self) -> bool {
        self.current_usage >= self.context_max || self.current_iteration >= self.max_iterations
    }

    pub(crate) fn remaining_tokens(&self) -> u64 {
        self.context_max.saturating_sub(self.current_usage)
    }

    pub(crate) fn remaining_iterations(&self) -> u32 {
        self.max_iterations.saturating_sub(self.current_iteration)
    }

    pub(crate) fn envelope_for_type(work_type: &str) -> BudgetEnvelope {
        match work_type.to_uppercase().as_str() {
            "DIRECT" => Self::forecast_for_complexity(&ComplexityLevel::Direct),
            "INVESTIGATE" => Self::forecast_for_complexity(&ComplexityLevel::Investigate),
            "MULTISTEP" => Self::forecast_for_complexity(&ComplexityLevel::Multistep),
            "OPEN_ENDED" => Self::forecast_for_complexity(&ComplexityLevel::OpenEnded),
            _ => Self::forecast_for_complexity(&ComplexityLevel::Investigate),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_direct_envelope() {
        let envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Direct);
        assert_eq!(envelope.max_tokens, 1000);
        assert_eq!(envelope.max_iterations, 3);
        assert_eq!(envelope.current_usage, 0);
        assert_eq!(envelope.current_iteration, 0);
    }

    #[test]
    fn test_investigate_envelope() {
        let envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Investigate);
        assert_eq!(envelope.max_tokens, 4000);
        assert_eq!(envelope.max_iterations, 8);
    }

    #[test]
    fn test_multistep_envelope() {
        let envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Multistep);
        assert_eq!(envelope.max_tokens, 8000);
        assert_eq!(envelope.max_iterations, 15);
    }

    #[test]
    fn test_open_ended_envelope() {
        let envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::OpenEnded);
        assert_eq!(envelope.max_tokens, 16000);
        assert_eq!(envelope.max_iterations, 30);
    }

    #[test]
    fn test_new_forecaster_starts_at_zero() {
        let f = BudgetForecaster::new(128000);
        assert_eq!(f.remaining_tokens(), 128000);
        assert_eq!(f.remaining_iterations(), u32::MAX);
        assert!(!f.is_exhausted());
    }

    #[test]
    fn test_track_usage_accumulates() {
        let mut f = BudgetForecaster::new(128000);
        f.track_usage(500);
        assert_eq!(f.remaining_tokens(), 127500);
        assert_eq!(f.remaining_iterations(), u32::MAX - 1);
        assert!(!f.is_exhausted());
    }

    #[test]
    fn test_exhaustion_by_tokens() {
        let mut f = BudgetForecaster::new(1000);
        f.track_usage(500);
        assert!(!f.is_exhausted());
        f.track_usage(500);
        assert!(f.is_exhausted());
    }

    #[test]
    fn test_exhaustion_by_iterations() {
        let mut f = BudgetForecaster::new(128000);
        f.max_iterations = 5;
        for _ in 0..5 {
            assert!(!f.is_exhausted());
            f.track_usage(100);
        }
        assert!(f.is_exhausted());
    }

    #[test]
    fn test_envelope_remaining_tokens() {
        let mut envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Direct);
        envelope.current_usage = 300;
        assert_eq!(envelope.remaining_tokens(), 700);
    }

    #[test]
    fn test_envelope_remaining_iterations() {
        let mut envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Direct);
        envelope.current_iteration = 1;
        assert_eq!(envelope.remaining_iterations(), 2);
    }

    #[test]
    fn test_envelope_is_exhausted() {
        let mut envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Direct);
        assert!(!envelope.is_exhausted());
        envelope.current_usage = 1000;
        assert!(envelope.is_exhausted());
    }

    #[test]
    fn test_envelope_is_exhausted_by_iteration() {
        let mut envelope = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Direct);
        envelope.current_iteration = 3;
        assert!(envelope.is_exhausted());
    }

    #[test]
    fn test_envelope_for_type() {
        assert_eq!(
            BudgetForecaster::envelope_for_type("DIRECT").max_tokens,
            1000
        );
        assert_eq!(
            BudgetForecaster::envelope_for_type("INVESTIGATE").max_tokens,
            4000
        );
        assert_eq!(
            BudgetForecaster::envelope_for_type("MULTISTEP").max_tokens,
            8000
        );
        assert_eq!(
            BudgetForecaster::envelope_for_type("OPEN_ENDED").max_tokens,
            16000
        );
    }

    #[test]
    fn test_envelope_for_type_case_insensitive() {
        assert_eq!(
            BudgetForecaster::envelope_for_type("direct").max_tokens,
            1000
        );
        assert_eq!(
            BudgetForecaster::envelope_for_type("Direct").max_tokens,
            1000
        );
    }

    #[test]
    fn test_envelope_for_type_unknown_defaults_to_investigate() {
        assert_eq!(
            BudgetForecaster::envelope_for_type("unknown_type").max_tokens,
            4000
        );
    }

    #[test]
    fn test_envelope_max_context_window_scales_with_complexity() {
        let direct = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::Direct);
        let open = BudgetForecaster::forecast_for_complexity(&ComplexityLevel::OpenEnded);
        assert!(direct.max_context_window < open.max_context_window);
    }

    #[test]
    fn test_track_usage_saturating_sub() {
        let mut f = BudgetForecaster::new(100);
        f.track_usage(200);
        assert_eq!(f.remaining_tokens(), 0);
        assert!(f.is_exhausted());
    }

    #[test]
    fn test_multiple_track_calls() {
        let mut f = BudgetForecaster::new(10000);
        f.track_usage(2500);
        f.track_usage(1500);
        f.track_usage(1000);
        assert_eq!(f.remaining_tokens(), 5000);
        assert_eq!(f.remaining_iterations(), u32::MAX - 3);
    }
}
