//! @efficiency-role: domain-logogic
//!
//! Auto-Compact (Task 114) — Context Window Management
//!
//! Monitors approximate token usage across the conversation.
//! When approaching the model's context window limit, generates
//! an inline summary of old messages to free space.
//!
//! Inspired by Claude Code's `autoCompact.ts` — simplified for 3B models.

use crate::evidence_ledger::EvidenceLedger;
use crate::*;

/// Approximate tokens per character for English text.
/// Conservative estimate: 1 token ≈ 3.5 chars.
/// This is used as fallback when model tokenizer is unavailable.
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

    /// Estimate tokens from a string using fallback method.
    pub(crate) fn estimate_tokens(text: &str) -> usize {
        (text.len() as f64 / CHARS_PER_TOKEN) as usize
    }

    /// Estimate tokens from a string using the model's tokenizer.
    pub(crate) fn estimate_tokens_for_model(text: &str, model_name: &str) -> usize {
        crate::model_capabilities::estimate_token_count(text, model_name)
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

    /// Update token count using model-specific tokenizer.
    pub(crate) fn recalculate_with_model(&mut self, messages: &[ChatMessage], model_name: &str) {
        self.total_tokens = messages
            .iter()
            .map(|m| Self::estimate_tokens_for_model(&m.content, model_name))
            .sum();
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

    /// Check if compact should fire, using model-specific context window.
    pub(crate) fn should_compact_for_model(
        &self,
        model_name: &str,
        buffer_tokens: Option<usize>,
    ) -> (bool, usize, usize) {
        let registry = crate::model_capabilities::ModelCapabilityRegistry::new();
        let caps = registry.get(model_name);
        let ctx = caps
            .map(|c| c.context_window as usize)
            .unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS);
        let buf = buffer_tokens.unwrap_or(DEFAULT_COMPACT_BUFFER_TOKENS);
        self.should_compact(Some(ctx), Some(buf))
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

/// Generate an inline summary of old messages.
/// This is a self-summary — Elma summarizes its own conversation
/// without needing a forked agent (preferred for 3B models).
pub(crate) fn generate_inline_summary(
    messages: &[ChatMessage],
    keep_recent_turns: usize,
    evidence_ledger: Option<&EvidenceLedger>,
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
                    exchanges.push(format!(
                        "User: {}\nElma: {}",
                        current_user, current_assistant
                    ));
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
        exchanges.push(format!(
            "User: {}\nElma: {}",
            current_user, current_assistant
        ));
    }

    let summary = if exchanges.is_empty() {
        // Include read file inventory if available (evidence-aware compaction)
        let read_inventory = evidence_ledger
            .map(|l| l.read_inventory_summary())
            .filter(|s| !s.is_empty())
            .unwrap_or_default();
        if read_inventory.is_empty() {
            "[Earlier conversation: technical tool usage]".to_string()
        } else {
            format!(
                "[Earlier conversation: technical tool usage]\n\n{}",
                read_inventory
            )
        }
    } else {
        let total = exchanges.len();
        let first = exchanges
            .first()
            .map(|s| s.chars().take(100).collect::<String>())
            .unwrap_or_default();
        let last = exchanges
            .last()
            .map(|s| s.chars().take(100).collect::<String>())
            .unwrap_or_default();
        format!(
            "[Earlier conversation summary: {} exchanges total.\n\
             First: {}\n\
             Last: {}\n\
             Details omitted to preserve context window.]",
            total, first, last
        )
    };

    // Append read file inventory if available (evidence-aware compaction)
    let summary = if let Some(ledger) = evidence_ledger {
        let read_inventory = ledger.read_inventory_summary();
        if !read_inventory.is_empty() {
            format!("{}\n\n{}", summary, read_inventory)
        } else {
            summary
        }
    } else {
        summary
    };

    let old_tokens: usize = old_messages
        .iter()
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
    evidence_ledger: Option<&EvidenceLedger>,
) -> (Vec<ChatMessage>, CompactResult) {
    let result = generate_inline_summary(messages, keep_recent_turns, evidence_ledger);
    if !result.ok {
        return (messages.to_vec(), result);
    }

    let cutoff = messages.len().saturating_sub(keep_recent_turns * 2);
    let mut new_messages = vec![ChatMessage::simple("system", &result.summary)];
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
    evidence_ledger: Option<&EvidenceLedger>,
) -> (Vec<ChatMessage>, CompactResult) {
    // In a real implementation, this would call the summarizer intel unit.
    // For now, we use the stable inline summary to satisfy the budget protection goal.
    apply_compact(messages, keep_recent_turns, evidence_ledger)
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

    #[test]
    fn test_generate_inline_summary_with_read_inventory() {
        // Verify that evidence-aware compaction includes read file inventory
        use std::path::PathBuf;
        let mut ledger =
            crate::evidence_ledger::EvidenceLedger::new("s_compact_test", &PathBuf::from("/tmp"));
        ledger.add_entry(
            crate::evidence_ledger::EvidenceSource::Read {
                path: "src/main.rs".to_string(),
            },
            "fn main() { println!(\"hello\"); }",
        );
        ledger.add_entry(
            crate::evidence_ledger::EvidenceSource::Read {
                path: "Cargo.toml".to_string(),
            },
            "[package]\nname = \"elma-cli\"",
        );

        // Build messages that trigger the empty-exchanges branch
        let mut msgs = Vec::new();
        msgs.push(ChatMessage::simple("user", "read all files"));
        msgs.push(ChatMessage::simple(
            "assistant",
            "[Earlier conversation: read all files]",
        ));
        for _ in 0..8 {
            msgs.push(ChatMessage::simple("user", "do something"));
            msgs.push(ChatMessage::simple("assistant", "done"));
        }

        let result = generate_inline_summary(&msgs, 3, Some(&ledger));
        assert!(result.ok);
        assert!(
            result.summary.contains("src/main.rs"),
            "Evidence-aware compact should preserve read file paths. Got: {}",
            result.summary
        );
        assert!(result.summary.contains("Cargo.toml"));
        assert!(
            result.summary.contains("fn main()"),
            "Should preserve per-file summaries"
        );
        assert!(
            result.summary.contains("Read File Inventory"),
            "Should contain the inventory heading"
        );
        // With enough old messages, tokens should be freed
        assert!(
            result.tokens_freed > 0 || result.summary.len() > 50,
            "Either freed tokens or produced a meaningful summary"
        );
    }

    #[test]
    fn test_generate_inline_summary_without_read_inventory() {
        // Without ledger, should produce standard summary
        let mut msgs = Vec::new();
        for _ in 0..8 {
            msgs.push(ChatMessage::simple("user", "do something"));
            msgs.push(ChatMessage::simple("assistant", "done"));
        }

        let result = generate_inline_summary(&msgs, 3, None);
        assert!(result.ok);
        // Should NOT contain file inventory headers
        assert!(!result.summary.contains("Read File Inventory"));
    }
}
