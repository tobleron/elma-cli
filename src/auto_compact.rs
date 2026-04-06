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
/// Don't compact a 1-turn conversation.
pub(crate) const MIN_TURNS_BEFORE_COMPACT: usize = 4;

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
        self.total_tokens = messages.iter()
            .map(|m| Self::estimate_tokens(&m.content))
            .sum();
        // Count user+assistant pairs as turns
        self.turn_count = messages.iter()
            .filter(|m| m.role == "user")
            .count();
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

        let should = self.total_tokens >= threshold
            && self.turn_count >= MIN_TURNS_BEFORE_COMPACT
            && self.compact_failures < MAX_COMPACT_FAILURES;

        (should, ctx, buf)
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

/// Generate an inline summary of old messages.
/// This is a self-summary — Elma summarizes its own conversation
/// without needing a forked agent (preferred for 3B models).
pub(crate) fn generate_inline_summary(
    messages: &[ChatMessage],
    keep_recent_turns: usize,
) -> CompactResult {
    if messages.len() <= keep_recent_turns * 2 {
        return CompactResult {
            summary: String::new(),
            ok: false,
            tokens_freed: 0,
        };
    }

    // Identify messages to summarize (everything except the last N turns)
    let cutoff = messages.len().saturating_sub(keep_recent_turns * 2);
    let old_messages = &messages[..cutoff];
    let recent_messages = &messages[cutoff..];

    if old_messages.is_empty() {
        return CompactResult {
            summary: String::new(),
            ok: false,
            tokens_freed: 0,
        };
    }

    // Build a summary of old messages
    let mut exchanges: Vec<String> = Vec::new();
    let mut current_user = String::new();
    let mut current_assistant = String::new();

    for msg in old_messages {
        match msg.role.as_str() {
            "user" => {
                if !current_user.is_empty() && !current_assistant.is_empty() {
                    exchanges.push(format!("User: {}\nElma: {}", current_user, current_assistant));
                }
                current_user = msg.content.chars().take(200).collect();
                current_assistant = String::new();
            }
            "assistant" | "tool" => {
                current_assistant = msg.content.chars().take(200).collect();
            }
            _ => {}
        }
    }
    // Flush last exchange
    if !current_user.is_empty() && !current_assistant.is_empty() {
        exchanges.push(format!("User: {}\nElma: {}", current_user, current_assistant));
    }

    let summary = if exchanges.is_empty() {
        "[Earlier conversation: technical tool usage]".to_string()
    } else {
        let total = exchanges.len();
        let first = exchanges.first().map(|s| s.chars().take(100).collect::<String>()).unwrap_or_default();
        let last = exchanges.last().map(|s| s.chars().take(100).collect::<String>()).unwrap_or_default();
        format!(
            "[Earlier conversation summary: {} exchanges total.\n\
             First: {}\n\
             Last: {}\n\
             Details omitted to preserve context window.]",
            total, first, last
        )
    };

    let old_tokens: usize = old_messages.iter()
        .map(|m| CompactTracker::estimate_tokens(&m.content))
        .sum();
    let summary_tokens = CompactTracker::estimate_tokens(&summary);
    let freed = old_tokens.saturating_sub(summary_tokens);

    // Build new messages: summary + recent
    // Note: caller must replace messages with this
    CompactResult {
        summary,
        ok: true,
        tokens_freed: freed,
    }
}

/// Apply compact: replace old messages with summary, keep recent intact.
/// Returns the new message list.
pub(crate) fn apply_compact(
    messages: &[ChatMessage],
    keep_recent_turns: usize,
) -> (Vec<ChatMessage>, CompactResult) {
    let result = generate_inline_summary(messages, keep_recent_turns);
    if !result.ok {
        return (messages.to_vec(), result);
    }

    let cutoff = messages.len().saturating_sub(keep_recent_turns * 2);
    let mut new_messages = vec![ChatMessage::simple("system", &result.summary)];
    new_messages.extend_from_slice(&messages[cutoff..]);

    (new_messages, result)
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
}
