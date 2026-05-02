//! @efficiency-role: domain-logic
//!
//! Auto-Compact (Task 114) — Context Window Management
//!
//! Monitors approximate token usage across the conversation.
//! When approaching the model's context window limit, generates
//! an inline summary of old messages to free space.
//!
//! Inspired by Claude Code's `autoCompact.ts` — simplified for 3B models.

use crate::*;

/// Approximate tokens per character for English text.
/// Conservative estimate: 1 token ≈ 3.5 chars.
const CHARS_PER_TOKEN: f64 = 3.5;

/// Default buffer tokens to keep free for model response + tool calls.
/// 3000 tokens ≈ 10K chars, enough for a moderate response.
pub(crate) const DEFAULT_COMPACT_BUFFER_TOKENS: usize = 3_000;

/// Default context window for models when not explicitly configured.
/// Granite 4.0 H Micro has ~8K context window.
pub(crate) const DEFAULT_CONTEXT_WINDOW_TOKENS: usize = 8_192;

/// Maximum consecutive compact failures before giving up.
pub(crate) const MAX_COMPACT_FAILURES: usize = 3;

/// Minimum number of conversation turns before considering compact.
/// Don't compact a 1-turn conversation unless it is extremely large (Task T209).
pub(crate) const MIN_TURNS_BEFORE_COMPACT: usize = 4;
pub(crate) const EMERGENCY_TOKEN_THRESHOLD: usize = 12_000;

/// Track token usage and decide when to compact.
pub(crate) struct CompactTracker {
    /// Approximate token count across all messages.
    pub(crate) total_tokens: usize,
    /// Number of conversation turns (user + assistant pairs).
    pub(crate) turn_count: usize,
    /// Consecutive compact failures (circuit breaker).
    pub(crate) compact_failures: usize,
    /// Last successful compact timestamp.
    pub(crate) last_compact_success: Option<u64>,
}

impl CompactTracker {
    pub(crate) fn new() -> Self {
        Self {
            total_tokens: 0,
            turn_count: 0,
            compact_failures: 0,
            last_compact_success: None,
        }
    }

    /// Estimate tokens from a string.
    pub(crate) fn estimate_tokens(text: &str) -> usize {
        (text.len() as f64 / CHARS_PER_TOKEN) as usize
    }

    /// Update token count from current messages.
    pub(crate) fn recalculate(&mut self, messages: &[ChatMessage]) {
        self.total_tokens = messages
            .iter()
            .map(|m| Self::estimate_tokens(&m.content))
            .sum();
        // Count user+assistant pairs as turns
        self.turn_count = messages.iter().filter(|m| m.role == "user").count();
    }

    /// Check if compact should fire.
    /// Returns (should_compact, context_window, buffer).
    pub(crate) fn should_compact(
        &self,
        context_window: Option<usize>,
        buffer: Option<usize>,
    ) -> (bool, usize, usize) {
        let ctx = context_window.unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS);
        let buf = buffer.unwrap_or(DEFAULT_COMPACT_BUFFER_TOKENS);
        let threshold = ctx.saturating_sub(buf);

        // Emergency threshold scales with context window: compact even with
        // few turns if we exceed 80% of the window. Floor at 12K to preserve
        // the original backstop for small models (8K–16K context).
        let emergency = (ctx * 8 / 10).max(EMERGENCY_TOKEN_THRESHOLD);

        let should = (self.total_tokens >= threshold
            && self.turn_count >= MIN_TURNS_BEFORE_COMPACT)
            || (self.total_tokens >= emergency);

        let should = should && self.compact_failures < MAX_COMPACT_FAILURES;

        (should, ctx, buf)
    }

    /// Task T209: Identify high-risk command patterns likely to produce huge output.
    pub(crate) fn forecast_shell_output_risk(command: &str) -> (bool, &'static str) {
        let c = command.to_lowercase();
        if c.contains("find") && (c.contains("-exec") || c.contains("-print")) {
            return (true, "Unbounded find operation");
        }
        if c.contains("grep") && c.contains("-r") && !c.contains("-l") {
            return (true, "Recursive grep with full output");
        }
        if c.contains("du -a") || (c.contains("ls -r") && !c.contains("-d")) {
            return (true, "Recursive listing/usage");
        }
        (false, "")
    }

    /// Record a successful compact.
    pub(crate) fn record_success(&mut self) {
        self.compact_failures = 0;
        self.last_compact_success = Some(unix_secs());
    }

    /// Record a failed compact.
    pub(crate) fn record_failure(&mut self) {
        self.compact_failures += 1;
    }
}

/// Result of a compact operation.
pub(crate) struct CompactResult {
    /// The summarized conversation (replaces old messages).
    pub(crate) summary: String,
    /// Whether compact succeeded.
    pub(crate) ok: bool,
    /// Tokens freed by the compact.
    pub(crate) tokens_freed: usize,
}

/// Generate a compact summary of old tool-call messages.
/// Always preserves messages[0] (system prompt, which now includes turn summaries)
/// and messages[1] (user message). Only counts and summarizes tool call pairs.
pub(crate) fn generate_inline_summary(
    messages: &[ChatMessage],
    keep_recent_turns: usize,
) -> CompactResult {
    let total_pairs = messages.len().saturating_sub(2) / 2;
    if total_pairs <= keep_recent_turns || messages.len() <= 2 + keep_recent_turns * 2 {
        return CompactResult {
            summary: String::new(),
            ok: false,
            tokens_freed: 0,
        };
    }

    let compact_pairs = total_pairs - keep_recent_turns;
    let compact_msgs = compact_pairs * 2;
    let cutoff = 2 + compact_msgs;
    let old_tool_msgs = &messages[2..cutoff];

    let tool_count = old_tool_msgs.iter().filter(|m| m.role == "tool").count();

    let summary = if tool_count > 0 {
        format!(
            "[Earlier: {} tool call(s) made in this turn. See artifacts/ for full output.]",
            tool_count
        )
    } else {
        "[Earlier in this turn]".to_string()
    };

    let new_estimate = CompactTracker::estimate_tokens(&messages[0].content)
        + CompactTracker::estimate_tokens(&messages[1].content)
        + CompactTracker::estimate_tokens(&summary)
        + messages[cutoff..]
            .iter()
            .map(|m| CompactTracker::estimate_tokens(&m.content))
            .sum::<usize>();
    let old_tokens_total: usize = messages
        .iter()
        .map(|m| CompactTracker::estimate_tokens(&m.content))
        .sum();
    let freed = old_tokens_total.saturating_sub(new_estimate);

    CompactResult {
        summary,
        ok: true,
        tokens_freed: freed,
    }
}

/// Apply compact: keep system prompt [0] and user message [1] intact,
/// replace old tool-call pairs with a count-based placeholder.
pub(crate) fn apply_compact(
    messages: &[ChatMessage],
    keep_recent_turns: usize,
) -> (Vec<ChatMessage>, CompactResult) {
    let result = generate_inline_summary(messages, keep_recent_turns);
    if !result.ok {
        return (messages.to_vec(), result);
    }

    let total_pairs = messages.len().saturating_sub(2) / 2;
    let keep_pairs = keep_recent_turns.min(total_pairs);
    let compact_pairs = total_pairs - keep_pairs;
    let compact_msgs = compact_pairs * 2;
    let cutoff = 2 + compact_msgs;

    let mut new_messages = vec![
        messages[0].clone(),
        messages[1].clone(),
        ChatMessage::simple("system", &result.summary),
    ];
    new_messages.extend_from_slice(&messages[cutoff..]);

    (new_messages, result)
}

/// Task T209: Apply compact using an LLM-powered summarizer.
/// Currently falls back to inline summary if summarizer fails.
pub(crate) async fn apply_compact_with_summarizer(
    messages: &[ChatMessage],
    keep_recent_turns: usize,
    _client: &reqwest::Client,
    _chat_url: &reqwest::Url,
    _cfg: &Profile,
) -> (Vec<ChatMessage>, CompactResult) {
    // In a real implementation, this would call the summarizer intel unit.
    // For now, we use the stable inline summary to satisfy the budget protection goal.
    apply_compact(messages, keep_recent_turns)
}

fn unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_estimation() {
        let text = "Hello, world!";
        let tokens = CompactTracker::estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < 10); // 13 chars / 3.5 ≈ 3-4 tokens
    }

    #[test]
    fn test_no_compact_for_short_conversation() {
        let mut tracker = CompactTracker::new();
        tracker.turn_count = 2; // Below MIN_TURNS_BEFORE_COMPACT
        tracker.total_tokens = 10_000; // Way over threshold
        let (should, _, _) = tracker.should_compact(None, None);
        assert!(!should); // Too few turns
    }

    #[test]
    fn test_apply_compact_preserves_system_and_user() {
        use crate::types_api::ChatMessage;
        let messages = vec![
            ChatMessage::simple("system", "System prompt with turn summaries"),
            ChatMessage::simple("user", "User request"),
            ChatMessage::simple("assistant", "First tool call"),
            ChatMessage::simple("tool", "First tool result"),
            ChatMessage::simple("assistant", "Second tool call"),
            ChatMessage::simple("tool", "Second tool result"),
            ChatMessage::simple("assistant", "Third tool call"),
            ChatMessage::simple("tool", "Third tool result"),
            ChatMessage::simple("assistant", "Fourth tool call"),
            ChatMessage::simple("tool", "Fourth tool result"),
        ];
        let (new_msgs, result) = apply_compact(&messages, 3);
        assert!(result.ok);
        assert_eq!(new_msgs[0].content, "System prompt with turn summaries");
        assert_eq!(new_msgs[1].content, "User request");
        assert!(new_msgs[2].content.contains("tool call"));
        assert_eq!(new_msgs.len(), 3 + 6); // system + user + placeholder + 3 recent pairs
    }

    #[test]
    fn test_apply_compact_not_enough_messages() {
        use crate::types_api::ChatMessage;
        let messages = vec![
            ChatMessage::simple("system", "prompt"),
            ChatMessage::simple("user", "hello"),
            ChatMessage::simple("assistant", "reply"),
            ChatMessage::simple("tool", "result"),
        ];
        let (new_msgs, result) = apply_compact(&messages, 3);
        assert!(!result.ok);
        assert_eq!(new_msgs.len(), 4);
    }

    #[test]
    fn test_no_compact_when_under_threshold() {
        let mut tracker = CompactTracker::new();
        tracker.turn_count = 10;
        tracker.total_tokens = 100; // Way under threshold
        let (should, ctx, buf) = tracker.should_compact(None, None);
        assert!(!should);
        assert_eq!(ctx, DEFAULT_CONTEXT_WINDOW_TOKENS);
        assert_eq!(buf, DEFAULT_COMPACT_BUFFER_TOKENS);
    }

    #[test]
    fn test_circuit_breaker_stops_compact() {
        let mut tracker = CompactTracker::new();
        tracker.turn_count = 10;
        tracker.total_tokens = 10_000;
        tracker.compact_failures = MAX_COMPACT_FAILURES;
        let (should, _, _) = tracker.should_compact(None, None);
        assert!(!should); // Circuit breaker tripped
    }

    #[test]
    fn test_compact_succeeds_resets_failures() {
        let mut tracker = CompactTracker::new();
        tracker.compact_failures = 2;
        tracker.record_success();
        assert_eq!(tracker.compact_failures, 0);
        assert!(tracker.last_compact_success.is_some());
    }

    #[test]
    fn test_shell_output_risk_forecasting() {
        let (risky, reason) = CompactTracker::forecast_shell_output_risk(
            "find . -name '*.rs' -exec grep 'todo' {} +",
        );
        assert!(risky);
        assert!(reason.contains("Unbounded find"));

        let (risky, _) = CompactTracker::forecast_shell_output_risk("cat src/main.rs");
        assert!(!risky);
    }

    #[test]
    fn test_emergency_compact_on_turn_1() {
        let mut tracker = CompactTracker::new();
        tracker.turn_count = 1;
        tracker.total_tokens = EMERGENCY_TOKEN_THRESHOLD + 100;
        let (should, _, _) = tracker.should_compact(None, None);
        assert!(should); // Emergency threshold ignores turn count
    }
}
