//! @efficiency-role: data-model
//! Fault injection and stream error recovery for LLM providers.
//!
//! Enables deterministic testing of provider failure modes by injecting
//! controlled faults into streaming responses and providing classification
//! and recovery strategies for real-world stream errors.

/// Probability of fault injection (0.0 to 1.0) and which fault types to use.
#[derive(Debug, Clone)]
pub(crate) struct FaultConfig {
    pub(crate) inject_rate: f64,
    pub(crate) fault_types: Vec<FaultType>,
    pub(crate) seed: Option<u64>,
}

/// Types of faults that can be injected into a provider stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FaultType {
    /// Simulate a network-level connection drop mid-stream.
    ConnectionDrop,
    /// Simulate a timeout occurring during streaming.
    StreamTimeout,
    /// Inject a malformed JSON chunk into the stream data.
    MalformedJson,
    /// Inject an empty content delta event.
    EmptyDelta,
    /// Inject tool call deltas with mismatched call IDs.
    ToolCallMismatch,
    /// Simulate a 429 Too Many Requests response.
    RateLimit,
    /// Simulate a 500 Internal Server Error response.
    ServerError,
}

/// Action to take after processing a stream delta through the fault injector.
#[derive(Debug, Clone)]
pub(crate) enum StreamAction {
    /// Continue streaming with the provided (possibly modified) content.
    Continue(String),
    /// A fault was triggered; the stream should handle the given fault type.
    Fault(FaultType),
    /// The stream should terminate immediately.
    End,
}

/// Minimal deterministic PRNG (LCG) for reproducible fault injection.
/// Uses the classic constants from Numerical Recipes (a=1664525, c=1013904223, m=2^32).
#[derive(Debug, Clone)]
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generate next u32 and advance state.
    fn next_u32(&mut self) -> u32 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    /// Generate f64 in [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u32() as f64) / (u32::MAX as f64 + 1.0)
    }

    /// Generate usize in [0, max).
    fn next_usize(&mut self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }
        (self.next_u32() as usize) % max
    }
}

/// Injects controlled faults into streaming LLM responses for testing.
#[derive(Debug, Clone)]
pub(crate) struct FaultInjector {
    config: FaultConfig,
    rng: Option<SimpleRng>,
}

impl FaultInjector {
    /// Create a new fault injector from the given configuration.
    pub(crate) fn new(config: FaultConfig) -> Self {
        let rng = config.seed.map(SimpleRng::new);
        Self { config, rng }
    }

    /// Check whether a fault should be injected based on the configured rate.
    fn should_inject(&mut self) -> bool {
        match &mut self.rng {
            Some(rng) => rng.next_f64() < self.config.inject_rate,
            None => false,
        }
    }

    /// Pick a random fault type from the configured list.
    fn pick_fault(&mut self) -> Option<FaultType> {
        if self.config.fault_types.is_empty() {
            return None;
        }
        let idx = match &mut self.rng {
            Some(rng) => rng.next_usize(self.config.fault_types.len()),
            None => return None,
        };
        Some(self.config.fault_types[idx])
    }

    /// Process a stream delta and return the appropriate action.
    ///
    /// Returns [`StreamAction::Continue`] with the original delta when no fault
    /// is triggered. When a fault is triggered, returns [`StreamAction::Fault`]
    /// with the chosen fault type.
    pub(crate) fn maybe_inject_stream(&mut self, delta: &str) -> StreamAction {
        if !self.should_inject() {
            return StreamAction::Continue(delta.to_string());
        }
        match self.pick_fault() {
            Some(FaultType::EmptyDelta) => StreamAction::Continue(String::new()),
            Some(FaultType::MalformedJson) => {
                StreamAction::Continue(r#"{"invalid": "truncated "#.to_string())
            }
            Some(fault) => StreamAction::Fault(fault),
            None => StreamAction::Continue(delta.to_string()),
        }
    }

    /// Inject a connection drop fault.
    pub(crate) fn inject_connection_drop() -> StreamAction {
        StreamAction::Fault(FaultType::ConnectionDrop)
    }

    /// Inject a stream timeout fault.
    pub(crate) fn inject_timeout() -> StreamAction {
        StreamAction::Fault(FaultType::StreamTimeout)
    }
}

/// Classification of a stream error for recovery decision-making.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StreamErrorClass {
    /// Retryable transient error (connection reset, temporary outage).
    Transient,
    /// Non-retryable permanent error (invalid request, bad data).
    Permanent,
    /// Rate-limited; requires backoff before retry.
    RateLimited,
    /// Authentication failure; needs reconfiguration.
    AuthFailure,
}

/// Recovery strategy derived from a stream error classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoveryStrategy {
    pub(crate) should_retry: bool,
    pub(crate) backoff_ms: u64,
    pub(crate) reset_connection: bool,
    pub(crate) trim_context: bool,
}

impl RecoveryStrategy {
    /// No retry — give up immediately.
    pub(crate) fn no_retry() -> Self {
        Self {
            should_retry: false,
            backoff_ms: 0,
            reset_connection: false,
            trim_context: false,
        }
    }

    /// Retry with the given backoff, keeping connection and context intact.
    pub(crate) fn retry_with_backoff(backoff_ms: u64) -> Self {
        Self {
            should_retry: true,
            backoff_ms,
            reset_connection: false,
            trim_context: false,
        }
    }

    /// Retry after resetting the connection.
    pub(crate) fn retry_with_reset(backoff_ms: u64) -> Self {
        Self {
            should_retry: true,
            backoff_ms,
            reset_connection: true,
            trim_context: false,
        }
    }

    /// Rate-limited: long backoff, no connection reset, no context trim.
    pub(crate) fn rate_limited(backoff_ms: u64) -> Self {
        Self {
            should_retry: true,
            backoff_ms,
            reset_connection: false,
            trim_context: false,
        }
    }
}

/// Classifies provider stream errors and maps them to recovery strategies.
#[derive(Debug, Clone)]
pub(crate) struct StreamErrorRecovery;

impl StreamErrorRecovery {
    /// Classify an error string into a [`StreamErrorClass`].
    pub(crate) fn classify_error(error: &str) -> StreamErrorClass {
        let lower = error.to_lowercase();
        if lower.contains("rate limit")
            || lower.contains("429")
            || lower.contains("too many requests")
            || lower.contains("rate_limit")
            || lower.contains("quota exceeded")
        {
            return StreamErrorClass::RateLimited;
        }
        if lower.contains("auth")
            || lower.contains("unauthorized")
            || lower.contains("401")
            || lower.contains("403")
            || lower.contains("forbidden")
            || lower.contains("invalid api key")
            || lower.contains("authentication")
        {
            return StreamErrorClass::AuthFailure;
        }
        if lower.contains("timeout")
            || lower.contains("timed out")
            || lower.contains("connection reset")
            || lower.contains("connection refused")
            || lower.contains("eof")
            || lower.contains("5")
            || lower.contains("server error")
            || lower.contains("temporarily")
            || lower.contains("try again")
            || lower.contains("service unavailable")
            || lower.contains("503")
            || lower.contains("502")
            || lower.contains("504")
            || lower.contains("internal server error")
            || lower.contains("500")
        {
            return StreamErrorClass::Transient;
        }
        StreamErrorClass::Permanent
    }

    /// Map a [`StreamErrorClass`] to a [`RecoveryStrategy`].
    pub(crate) fn recovery_strategy(class: &StreamErrorClass) -> RecoveryStrategy {
        match class {
            StreamErrorClass::Transient => RecoveryStrategy::retry_with_reset(1_000),
            StreamErrorClass::Permanent => RecoveryStrategy::no_retry(),
            StreamErrorClass::RateLimited => RecoveryStrategy::rate_limited(5_000),
            StreamErrorClass::AuthFailure => RecoveryStrategy::no_retry(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── FaultType assertions ──────────────────────────────────────────────

    #[test]
    fn test_fault_type_variants() {
        let variants = [
            FaultType::ConnectionDrop,
            FaultType::StreamTimeout,
            FaultType::MalformedJson,
            FaultType::EmptyDelta,
            FaultType::ToolCallMismatch,
            FaultType::RateLimit,
            FaultType::ServerError,
        ];
        assert_eq!(variants.len(), 7);
    }

    // ── FaultConfig / FaultInjector construction ─────────────────────────

    #[test]
    fn test_fault_injector_never_injects_when_rate_zero() {
        let config = FaultConfig {
            inject_rate: 0.0,
            fault_types: vec![FaultType::ConnectionDrop],
            seed: Some(42),
        };
        let mut injector = FaultInjector::new(config);
        for _ in 0..100 {
            let action = injector.maybe_inject_stream("hello");
            assert!(matches!(action, StreamAction::Continue(ref s) if s == "hello"));
        }
    }

    #[test]
    fn test_fault_injector_always_injects_when_rate_one() {
        let config = FaultConfig {
            inject_rate: 1.0,
            fault_types: vec![FaultType::ConnectionDrop],
            seed: Some(42),
        };
        let mut injector = FaultInjector::new(config);
        let injected = (0..100).any(|_| {
            matches!(
                injector.maybe_inject_stream("hello"),
                StreamAction::Fault(FaultType::ConnectionDrop)
            )
        });
        assert!(injected, "expected at least one injection at rate=1.0");
    }

    #[test]
    fn test_empty_delta_injection_returns_empty_string() {
        let config = FaultConfig {
            inject_rate: 1.0,
            fault_types: vec![FaultType::EmptyDelta],
            seed: Some(1),
        };
        let mut injector = FaultInjector::new(config);
        let action = injector.maybe_inject_stream("content");
        match action {
            StreamAction::Continue(ref s) => assert!(s.is_empty()),
            other => panic!("expected Continue(empty), got {other:?}"),
        }
    }

    #[test]
    fn test_malformed_json_injection() {
        let config = FaultConfig {
            inject_rate: 1.0,
            fault_types: vec![FaultType::MalformedJson],
            seed: Some(1),
        };
        let mut injector = FaultInjector::new(config);
        let action = injector.maybe_inject_stream("valid");
        match action {
            StreamAction::Continue(ref s) => {
                assert!(
                    s.starts_with(r#"{"invalid""#),
                    "expected malformed json, got: {s}"
                );
                assert!(
                    s.ends_with("truncated "),
                    "expected truncated json, got: {s}"
                );
            }
            other => panic!("expected Continue(malformed), got {other:?}"),
        }
    }

    #[test]
    fn test_inject_connection_drop() {
        let action = FaultInjector::inject_connection_drop();
        assert!(matches!(
            action,
            StreamAction::Fault(FaultType::ConnectionDrop)
        ));
    }

    #[test]
    fn test_inject_timeout() {
        let action = FaultInjector::inject_timeout();
        assert!(matches!(
            action,
            StreamAction::Fault(FaultType::StreamTimeout)
        ));
    }

    // ── StreamAction patterns ─────────────────────────────────────────────

    #[test]
    fn test_stream_action_continue_carries_content() {
        let action = StreamAction::Continue("hello world".to_string());
        if let StreamAction::Continue(s) = &action {
            assert_eq!(s, "hello world");
        } else {
            panic!("expected Continue variant");
        }
    }

    #[test]
    fn test_stream_action_fault_carries_type() {
        let action = StreamAction::Fault(FaultType::ServerError);
        assert!(matches!(
            action,
            StreamAction::Fault(FaultType::ServerError)
        ));
    }

    #[test]
    fn test_stream_action_end() {
        let action = StreamAction::End;
        assert!(matches!(action, StreamAction::End));
    }

    // ── StreamErrorClass classification ───────────────────────────────────

    #[test]
    fn test_classify_rate_limit_text() {
        let cases = [
            "rate limit exceeded",
            "429 Too Many Requests",
            "too many requests",
            "rate_limit_exceeded",
            "quota exceeded",
        ];
        for case in &cases {
            assert_eq!(
                StreamErrorRecovery::classify_error(case),
                StreamErrorClass::RateLimited,
                "failed for: {case}"
            );
        }
    }

    #[test]
    fn test_classify_auth_failure() {
        let cases = [
            "unauthorized",
            "401 Unauthorized",
            "403 Forbidden",
            "invalid API key",
            "authentication failed",
            "auth error",
        ];
        for case in &cases {
            assert_eq!(
                StreamErrorRecovery::classify_error(case),
                StreamErrorClass::AuthFailure,
                "failed for: {case}"
            );
        }
    }

    #[test]
    fn test_classify_transient() {
        let cases = [
            "connection reset",
            "connection refused",
            "timeout",
            "timed out",
            "EOF occurred",
            "500 Internal Server Error",
            "503 Service Unavailable",
            "502 Bad Gateway",
            "504 Gateway Timeout",
            "server error",
            "temporarily unavailable",
            "try again later",
        ];
        for case in &cases {
            assert_eq!(
                StreamErrorRecovery::classify_error(case),
                StreamErrorClass::Transient,
                "failed for: {case}"
            );
        }
    }

    #[test]
    fn test_classify_permanent() {
        let cases = [
            "invalid request body",
            "bad request",
            "model not found",
            "unsupported parameter",
            "",
        ];
        for case in &cases {
            assert_eq!(
                StreamErrorRecovery::classify_error(case),
                StreamErrorClass::Permanent,
                "failed for: {case}"
            );
        }
    }

    #[test]
    fn test_classify_partial_overlap_prefers_rate_limit() {
        // "rate limit" should match RateLimited even if other keywords present.
        assert_eq!(
            StreamErrorRecovery::classify_error("rate limit timeout error"),
            StreamErrorClass::RateLimited,
        );
    }

    #[test]
    fn test_classify_auth_over_transient() {
        // auth keywords checked before transient keywords.
        assert_eq!(
            StreamErrorRecovery::classify_error("authentication timeout"),
            StreamErrorClass::AuthFailure,
        );
    }

    // ── RecoveryStrategy mapping ──────────────────────────────────────────

    #[test]
    fn test_recovery_strategy_for_transient() {
        let strat = StreamErrorRecovery::recovery_strategy(&StreamErrorClass::Transient);
        assert!(strat.should_retry);
        assert!(strat.reset_connection);
        assert!(!strat.trim_context);
        assert_eq!(strat.backoff_ms, 1_000);
    }

    #[test]
    fn test_recovery_strategy_for_permanent() {
        let strat = StreamErrorRecovery::recovery_strategy(&StreamErrorClass::Permanent);
        assert!(!strat.should_retry);
        assert_eq!(strat.backoff_ms, 0);
    }

    #[test]
    fn test_recovery_strategy_for_rate_limited() {
        let strat = StreamErrorRecovery::recovery_strategy(&StreamErrorClass::RateLimited);
        assert!(strat.should_retry);
        assert!(!strat.reset_connection);
        assert!(!strat.trim_context);
        assert_eq!(strat.backoff_ms, 5_000);
    }

    #[test]
    fn test_recovery_strategy_for_auth_failure() {
        let strat = StreamErrorRecovery::recovery_strategy(&StreamErrorClass::AuthFailure);
        assert!(!strat.should_retry);
        assert_eq!(strat.backoff_ms, 0);
    }

    // ── RecoveryStrategy constructors ─────────────────────────────────────

    #[test]
    fn test_recovery_strategy_no_retry() {
        let s = RecoveryStrategy::no_retry();
        assert!(!s.should_retry);
        assert_eq!(s.backoff_ms, 0);
        assert!(!s.reset_connection);
        assert!(!s.trim_context);
    }

    #[test]
    fn test_recovery_strategy_retry_with_backoff() {
        let s = RecoveryStrategy::retry_with_backoff(250);
        assert!(s.should_retry);
        assert_eq!(s.backoff_ms, 250);
        assert!(!s.reset_connection);
        assert!(!s.trim_context);
    }

    #[test]
    fn test_recovery_strategy_retry_with_reset() {
        let s = RecoveryStrategy::retry_with_reset(2000);
        assert!(s.should_retry);
        assert_eq!(s.backoff_ms, 2000);
        assert!(s.reset_connection);
        assert!(!s.trim_context);
    }

    #[test]
    fn test_recovery_strategy_rate_limited() {
        let s = RecoveryStrategy::rate_limited(10_000);
        assert!(s.should_retry);
        assert_eq!(s.backoff_ms, 10_000);
        assert!(!s.reset_connection);
        assert!(!s.trim_context);
    }

    // ── Deterministic seed reproducibility ────────────────────────────────

    #[test]
    fn test_deterministic_seed_produces_same_sequence() {
        let config_a = FaultConfig {
            inject_rate: 0.5,
            fault_types: vec![FaultType::ConnectionDrop, FaultType::ServerError],
            seed: Some(999),
        };
        let config_b = FaultConfig {
            inject_rate: 0.5,
            fault_types: vec![FaultType::ConnectionDrop, FaultType::ServerError],
            seed: Some(999),
        };
        let mut injector_a = FaultInjector::new(config_a);
        let mut injector_b = FaultInjector::new(config_b);

        let seq_a: Vec<StreamAction> = (0..20)
            .map(|_| injector_a.maybe_inject_stream("x"))
            .collect();
        let seq_b: Vec<StreamAction> = (0..20)
            .map(|_| injector_b.maybe_inject_stream("x"))
            .collect();

        for (i, (a, b)) in seq_a.iter().zip(seq_b.iter()).enumerate() {
            assert_eq!(
                format!("{a:?}"),
                format!("{b:?}"),
                "diverged at position {i}"
            );
        }
    }

    // ── Edge cases ────────────────────────────────────────────────────────

    #[test]
    fn test_no_fault_types_injects_nothing() {
        let config = FaultConfig {
            inject_rate: 1.0,
            fault_types: vec![],
            seed: Some(42),
        };
        let mut injector = FaultInjector::new(config);
        let action = injector.maybe_inject_stream("hello");
        assert!(matches!(action, StreamAction::Continue(ref s) if s == "hello"));
    }

    #[test]
    fn test_no_seed_never_injects() {
        let config = FaultConfig {
            inject_rate: 1.0,
            fault_types: vec![FaultType::ConnectionDrop],
            seed: None,
        };
        let mut injector = FaultInjector::new(config);
        for _ in 0..10 {
            let action = injector.maybe_inject_stream("x");
            assert!(matches!(action, StreamAction::Continue(ref s) if s == "x"));
        }
    }
}
