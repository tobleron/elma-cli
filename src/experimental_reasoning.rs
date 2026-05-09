//! @efficiency-role: domain-logic
//!
//! Experimental reasoning tuning (Task 685).
//! Provides configurable reasoning parameters, creative recovery strategies,
//! and task-type-based tuning defaults.

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReasoningTuningConfig {
    pub(crate) temperature: f64,
    pub(crate) top_p: f64,
    pub(crate) max_reasoning_tokens: u64,
    pub(crate) creative_mode: bool,
}

impl Default for ReasoningTuningConfig {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            top_p: 0.9,
            max_reasoning_tokens: 2048,
            creative_mode: false,
        }
    }
}

pub(crate) struct CreativeRecovery;

impl CreativeRecovery {
    pub(crate) fn attempt_recovery(
        failed_approach: &str,
        config: &ReasoningTuningConfig,
    ) -> String {
        let base = format!(
            "Recovery from failed approach '{}' using temperature={} creative_mode={}",
            failed_approach, config.temperature, config.creative_mode
        );
        if config.creative_mode {
            format!("{} - try radically different decomposition", base)
        } else {
            format!("{} - try narrower refinement", base)
        }
    }

    pub(crate) fn vary_temperature(base: f64) -> f64 {
        let jitter = (base * 0.1).max(0.01);
        let delta = (randish() * 2.0 - 1.0) * jitter;
        (base + delta).clamp(0.0, 2.0)
    }
}

fn randish() -> f64 {
    // Deterministic pseudo-random for reproducibility.
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let val = COUNTER.fetch_add(1, Ordering::Relaxed);
    (val as f64) / (u64::MAX as f64)
}

pub(crate) struct ReasoningTuner;

impl ReasoningTuner {
    pub(crate) fn tune_for_task(task_type: &str) -> ReasoningTuningConfig {
        match task_type {
            "analysis" => ReasoningTuningConfig {
                temperature: 0.3,
                top_p: 0.8,
                max_reasoning_tokens: 4096,
                creative_mode: false,
            },
            "creative" => ReasoningTuningConfig {
                temperature: 1.2,
                top_p: 0.95,
                max_reasoning_tokens: 8192,
                creative_mode: true,
            },
            "debug" => ReasoningTuningConfig {
                temperature: 0.2,
                top_p: 0.7,
                max_reasoning_tokens: 2048,
                creative_mode: false,
            },
            "planning" => ReasoningTuningConfig {
                temperature: 0.6,
                top_p: 0.85,
                max_reasoning_tokens: 4096,
                creative_mode: false,
            },
            _ => ReasoningTuningConfig::default(),
        }
    }

    pub(crate) fn apply_config(config: &ReasoningTuningConfig) -> String {
        format!(
            "Reason with temperature={}, top_p={}, max_tokens={}, creative={}",
            config.temperature, config.top_p, config.max_reasoning_tokens, config.creative_mode
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tune_for_analysis() {
        let config = ReasoningTuner::tune_for_task("analysis");
        assert_eq!(config.temperature, 0.3);
        assert_eq!(config.top_p, 0.8);
        assert_eq!(config.max_reasoning_tokens, 4096);
        assert!(!config.creative_mode);
    }

    #[test]
    fn test_tune_for_creative() {
        let config = ReasoningTuner::tune_for_task("creative");
        assert_eq!(config.temperature, 1.2);
        assert!(config.creative_mode);
        assert_eq!(config.max_reasoning_tokens, 8192);
    }

    #[test]
    fn test_tune_for_debug() {
        let config = ReasoningTuner::tune_for_task("debug");
        assert_eq!(config.temperature, 0.2);
        assert_eq!(config.top_p, 0.7);
        assert_eq!(config.max_reasoning_tokens, 2048);
        assert!(!config.creative_mode);
    }

    #[test]
    fn test_tune_for_planning() {
        let config = ReasoningTuner::tune_for_task("planning");
        assert_eq!(config.temperature, 0.6);
        assert_eq!(config.top_p, 0.85);
        assert_eq!(config.max_reasoning_tokens, 4096);
        assert!(!config.creative_mode);
    }

    #[test]
    fn test_tune_for_unknown_task_defaults() {
        let config = ReasoningTuner::tune_for_task("unknown");
        assert_eq!(config, ReasoningTuningConfig::default());
    }

    #[test]
    fn test_apply_config() {
        let config = ReasoningTuningConfig {
            temperature: 0.5,
            top_p: 0.8,
            max_reasoning_tokens: 1024,
            creative_mode: true,
        };
        let formatted = ReasoningTuner::apply_config(&config);
        assert!(formatted.contains("temperature=0.5"));
        assert!(formatted.contains("top_p=0.8"));
        assert!(formatted.contains("max_tokens=1024"));
        assert!(formatted.contains("creative=true"));
    }

    #[test]
    fn test_creative_recovery_attempt() {
        let config = ReasoningTuningConfig {
            temperature: 1.0,
            creative_mode: true,
            ..Default::default()
        };
        let recovery = CreativeRecovery::attempt_recovery("test_fail", &config);
        assert!(recovery.contains("test_fail"));
        assert!(recovery.contains("radically different"));
    }

    #[test]
    fn test_non_creative_recovery() {
        let config = ReasoningTuningConfig {
            creative_mode: false,
            ..Default::default()
        };
        let recovery = CreativeRecovery::attempt_recovery("fail", &config);
        assert!(recovery.contains("narrower refinement"));
    }

    #[test]
    fn test_vary_temperature_stays_in_bounds() {
        for _ in 0..100 {
            let t = CreativeRecovery::vary_temperature(0.7);
            assert!((0.0..=2.0).contains(&t));
        }
    }

    #[test]
    fn test_vary_temperature_zero_base() {
        let t = CreativeRecovery::vary_temperature(0.0);
        assert!((0.0..=2.0).contains(&t));
    }

    #[test]
    fn test_reasoning_tuning_config_default() {
        let config = ReasoningTuningConfig::default();
        assert_eq!(config.temperature, 0.7);
        assert_eq!(config.top_p, 0.9);
        assert_eq!(config.max_reasoning_tokens, 2048);
        assert!(!config.creative_mode);
    }

    #[test]
    fn test_attempt_recovery_different_approaches() {
        let creative = ReasoningTuningConfig {
            creative_mode: true,
            ..Default::default()
        };
        let analytic = ReasoningTuningConfig {
            creative_mode: false,
            ..Default::default()
        };
        let c = CreativeRecovery::attempt_recovery("same", &creative);
        let a = CreativeRecovery::attempt_recovery("same", &analytic);
        assert_ne!(c, a);
    }
}
