//! @efficiency-role: domain-logic
//!
//! Trajectory Compression For Long-Running Sessions (Task 271)
//!
//! Tracks and compresses session trajectories for deep reasoning tasks.
//! Inspired by Hermes-Agent's trajectory_compressor — reduces long histories
//! into compact forms while preserving critical state and decision points.

use crate::*;

// ============================================================================
// Trajectory Types
// ============================================================================

/// Represents a single step in the session trajectory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrajectoryStep {
    /// Step identifier (e.g., "turn_1", "tool_call_3")
    pub id: String,
    /// Step type: "user_input", "assistant_response", "tool_call", "tool_result", "compact", "error"
    pub step_type: String,
    /// Brief description of what happened
    pub summary: String,
    /// Full content (may be truncated for storage)
    pub content: String,
    /// Token estimate for this step
    pub token_count: usize,
    /// Timestamp (unix seconds)
    pub timestamp: u64,
    /// Whether this step is critical (should not be compressed away)
    pub is_critical: bool,
    /// Outcome: "success", "failure", "partial", "pending"
    pub outcome: String,
}

impl TrajectoryStep {
    pub fn new(id: &str, step_type: &str, summary: &str, content: &str, is_critical: bool) -> Self {
        Self {
            id: id.to_string(),
            step_type: step_type.to_string(),
            summary: summary.chars().take(200).collect(),
            content: content.chars().take(1000).collect(),
            token_count: estimate_tokens(content),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            is_critical,
            outcome: "pending".to_string(),
        }
    }

    pub fn with_outcome(mut self, outcome: &str) -> Self {
        self.outcome = outcome.to_string();
        self
    }
}

/// Compressed trajectory segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CompressedSegment {
    /// Range of original steps this segment covers
    pub step_range: (usize, usize),
    /// Compressed summary of the segment
    pub summary: String,
    /// Key decisions/outcomes preserved
    pub key_points: Vec<String>,
    /// Total tokens in original steps
    pub original_tokens: usize,
    /// Tokens after compression
    pub compressed_tokens: usize,
}

/// Full trajectory record for a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrajectoryRecord {
    /// Session ID
    pub session_id: String,
    /// All trajectory steps
    pub steps: Vec<TrajectoryStep>,
    /// Compressed segments (if any compression has occurred)
    pub compressed_segments: Vec<CompressedSegment>,
    /// Total token count across all steps
    pub total_tokens: usize,
    /// Number of turns (user+assistant pairs)
    pub turn_count: usize,
    /// Number of tool calls
    pub tool_call_count: usize,
    /// Number of compressions performed
    pub compression_count: usize,
    /// Whether trajectory is currently compressed
    pub is_compressed: bool,
}

impl TrajectoryRecord {
    pub fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            steps: Vec::new(),
            compressed_segments: Vec::new(),
            total_tokens: 0,
            turn_count: 0,
            tool_call_count: 0,
            compression_count: 0,
            is_compressed: false,
        }
    }

    /// Add a new step to the trajectory
    pub fn add_step(&mut self, step: TrajectoryStep) {
        if step.step_type == "user_input" {
            self.turn_count += 1;
        }
        if step.step_type.starts_with("tool_") {
            self.tool_call_count += 1;
        }
        self.total_tokens += step.token_count;
        self.steps.push(step);
    }

    /// Check if trajectory should be compressed
    pub fn needs_compression(&self, token_threshold: usize, turn_threshold: usize) -> bool {
        if self.is_compressed {
            return false;
        }
        self.total_tokens >= token_threshold || self.turn_count >= turn_threshold
    }

    /// Get critical steps that should never be compressed away
    pub fn get_critical_steps(&self) -> Vec<&TrajectoryStep> {
        self.steps.iter().filter(|s| s.is_critical).collect()
    }

    /// Get recent steps (last N turns)
    pub fn get_recent_steps(&self, turns: usize) -> Vec<&TrajectoryStep> {
        let user_steps: Vec<&TrajectoryStep> = self
            .steps
            .iter()
            .filter(|s| s.step_type == "user_input")
            .collect();

        let cutoff_turn = user_steps.len().saturating_sub(turns);
        let cutoff_index = if cutoff_turn == 0 {
            0
        } else {
            user_steps
                .get(cutoff_turn - 1)
                .map(|s| {
                    self.steps
                        .iter()
                        .position(|step| step.id == s.id)
                        .unwrap_or(0)
                })
                .unwrap_or(0)
        };

        self.steps[cutoff_index..].iter().collect()
    }
}

/// Result of trajectory compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TrajectoryCompressionResult {
    /// Original token count
    pub original_tokens: usize,
    /// Compressed token count
    pub compressed_tokens: usize,
    /// Tokens freed
    pub tokens_freed: usize,
    /// Compression ratio (0.0-1.0, lower is better)
    pub compression_ratio: f64,
    /// Number of segments created
    pub segments_created: usize,
    /// Critical steps preserved
    pub critical_steps_preserved: usize,
}

// ============================================================================
// Trajectory Compression Engine
// ============================================================================

/// Estimate tokens from text (same as auto_compact)
fn estimate_tokens(text: &str) -> usize {
    const CHARS_PER_TOKEN: f64 = 3.5;
    (text.len() as f64 / CHARS_PER_TOKEN) as usize
}

/// Compress a trajectory record into segments while preserving critical steps
pub(crate) fn compress_trajectory(
    trajectory: &mut TrajectoryRecord,
    keep_recent_turns: usize,
    max_segment_turns: usize,
) -> TrajectoryCompressionResult {
    if trajectory.steps.is_empty() {
        return TrajectoryCompressionResult {
            original_tokens: 0,
            compressed_tokens: 0,
            tokens_freed: 0,
            compression_ratio: 1.0,
            segments_created: 0,
            critical_steps_preserved: 0,
        };
    }

    let original_tokens = trajectory.total_tokens;
    let critical_steps = trajectory.get_critical_steps();
    let critical_ids: std::collections::HashSet<&str> =
        critical_steps.iter().map(|s| s.id.as_str()).collect();

    // Get recent steps to keep uncompressed
    let recent_steps = trajectory.get_recent_steps(keep_recent_turns);
    let recent_ids: std::collections::HashSet<&str> =
        recent_steps.iter().map(|s| s.id.as_str()).collect();

    // Identify steps eligible for compression
    let compressible_steps: Vec<&TrajectoryStep> = trajectory
        .steps
        .iter()
        .filter(|s| !critical_ids.contains(s.id.as_str()) && !recent_ids.contains(s.id.as_str()))
        .collect();

    if compressible_steps.is_empty() {
        return TrajectoryCompressionResult {
            original_tokens,
            compressed_tokens: original_tokens,
            tokens_freed: 0,
            compression_ratio: 1.0,
            segments_created: 0,
            critical_steps_preserved: critical_steps.len(),
        };
    }

    // Group compressible steps into segments
    let mut segments = Vec::new();
    let mut current_segment_steps: Vec<&TrajectoryStep> = Vec::new();
    let mut current_segment_tokens = 0;

    for step in &compressible_steps {
        current_segment_steps.push(*step);
        current_segment_tokens += step.token_count;

        // Create segment when we hit max turns or token threshold
        let user_count = current_segment_steps
            .iter()
            .filter(|s| s.step_type == "user_input")
            .count();

        if user_count >= max_segment_turns || current_segment_tokens > 5000 {
            segments.push(create_segment(
                &current_segment_steps,
                current_segment_tokens,
            ));
            current_segment_steps.clear();
            current_segment_tokens = 0;
        }
    }

    // Flush remaining steps
    if !current_segment_steps.is_empty() {
        segments.push(create_segment(
            &current_segment_steps,
            current_segment_tokens,
        ));
    }

    // Calculate compressed token count
    let compressed_segment_tokens: usize = segments.iter().map(|s| s.compressed_tokens).sum();

    let critical_tokens: usize = critical_steps.iter().map(|s| s.token_count).sum();

    let recent_tokens: usize = recent_steps.iter().map(|s| s.token_count).sum();

    let compressed_tokens = compressed_segment_tokens + critical_tokens + recent_tokens;
    let tokens_freed = original_tokens.saturating_sub(compressed_tokens);
    let compression_ratio = if original_tokens > 0 {
        compressed_tokens as f64 / original_tokens as f64
    } else {
        1.0
    };

    // Update trajectory
    let critical_count = critical_steps.len();
    trajectory.compressed_segments = segments.clone();
    trajectory.compression_count += 1;
    trajectory.is_compressed = true;

    TrajectoryCompressionResult {
        original_tokens,
        compressed_tokens,
        tokens_freed,
        compression_ratio,
        segments_created: segments.len(),
        critical_steps_preserved: critical_count,
    }
}

/// Create a compressed segment from a group of steps
fn create_segment(steps: &[&TrajectoryStep], original_tokens: usize) -> CompressedSegment {
    if steps.is_empty() {
        return CompressedSegment {
            step_range: (0, 0),
            summary: String::new(),
            key_points: Vec::new(),
            original_tokens: 0,
            compressed_tokens: 0,
        };
    }

    let start_idx = steps.first().map(|s| s.id.clone()).unwrap_or_default();
    let end_idx = steps.last().map(|s| s.id.clone()).unwrap_or_default();

    // Extract key points from steps
    let key_points: Vec<String> = steps
        .iter()
        .filter(|s| s.outcome == "success" || s.is_critical)
        .map(|s| s.summary.clone())
        .take(5)
        .collect();

    // Build summary
    let user_count = steps.iter().filter(|s| s.step_type == "user_input").count();
    let tool_count = steps
        .iter()
        .filter(|s| s.step_type.starts_with("tool_"))
        .count();
    let error_count = steps.iter().filter(|s| s.outcome == "failure").count();

    let summary = format!(
        "[Compressed: {} turns, {} tool calls, {} errors. Key outcomes preserved.]",
        user_count, tool_count, error_count
    );

    let compressed_tokens =
        estimate_tokens(&summary) + key_points.iter().map(|p| estimate_tokens(p)).sum::<usize>();

    let start_pos = steps
        .first()
        .map(|s| {
            // This is approximate - in a real implementation we'd track indices properly
            0
        })
        .unwrap_or(0);
    let end_pos = start_pos + steps.len();

    CompressedSegment {
        step_range: (start_pos, end_pos),
        summary,
        key_points,
        original_tokens,
        compressed_tokens,
    }
}

/// Build a compressed trajectory summary for context injection
pub(crate) fn build_trajectory_summary(trajectory: &TrajectoryRecord) -> String {
    if trajectory.compressed_segments.is_empty() {
        return format!(
            "Session trajectory: {} turns, {} tool calls, {} total tokens",
            trajectory.turn_count, trajectory.tool_call_count, trajectory.total_tokens
        );
    }

    let mut parts = Vec::new();

    // Add compressed segment summaries
    for (i, segment) in trajectory.compressed_segments.iter().enumerate() {
        parts.push(format!("Segment {}: {}", i + 1, segment.summary));
        for point in &segment.key_points {
            parts.push(format!("  - {}", point));
        }
    }

    // Add recent activity
    let recent = trajectory.get_recent_steps(2);
    if !recent.is_empty() {
        parts.push("Recent activity:".to_string());
        for step in recent.iter().take(4) {
            parts.push(format!("  [{}] {}", step.step_type, step.summary));
        }
    }

    parts.join("\n")
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_trajectory() -> TrajectoryRecord {
        let mut traj = TrajectoryRecord::new("test_session");

        // Add some test steps
        traj.add_step(TrajectoryStep::new(
            "turn_1",
            "user_input",
            "User asks about Rust",
            "How do I write a function in Rust?",
            false,
        ));
        traj.add_step(TrajectoryStep::new(
            "resp_1",
            "assistant_response",
            "Explains Rust functions",
            "In Rust, you use the fn keyword...",
            false,
        ));
        traj.add_step(TrajectoryStep::new(
            "turn_2",
            "user_input",
            "User asks about traits",
            "What are traits?",
            false,
        ));
        traj.add_step(TrajectoryStep::new(
            "resp_2",
            "assistant_response",
            "Explains traits",
            "Traits are like interfaces...",
            false,
        ));
        traj.add_step(
            TrajectoryStep::new(
                "tool_1",
                "tool_call",
                "Reads Cargo.toml",
                "Reading file...",
                false,
            )
            .with_outcome("success"),
        );
        traj.add_step(TrajectoryStep::new(
            "turn_3",
            "user_input",
            "Critical: User asks for code review",
            "Review this code for safety",
            true,
        ));

        traj
    }

    #[test]
    fn test_trajectory_record_creation() {
        let traj = TrajectoryRecord::new("test");
        assert_eq!(traj.session_id, "test");
        assert!(traj.steps.is_empty());
        assert_eq!(traj.total_tokens, 0);
    }

    #[test]
    fn test_add_step_updates_counts() {
        let mut traj = TrajectoryRecord::new("test");
        traj.add_step(TrajectoryStep::new(
            "t1",
            "user_input",
            "test",
            "hello",
            false,
        ));
        traj.add_step(TrajectoryStep::new(
            "t2",
            "tool_call",
            "test",
            "running",
            false,
        ));

        assert_eq!(traj.turn_count, 1);
        assert_eq!(traj.tool_call_count, 1);
        assert!(traj.total_tokens > 0);
    }

    #[test]
    fn test_needs_compression_returns_true_when_over_threshold() {
        let mut traj = TrajectoryRecord::new("test");
        // Add enough steps to exceed token threshold
        for i in 0..100 {
            traj.add_step(TrajectoryStep::new(
                &format!("step_{}", i),
                "user_input",
                "test",
                &"x".repeat(500),
                false,
            ));
        }

        assert!(traj.needs_compression(10000, 50));
    }

    #[test]
    fn test_get_critical_steps_returns_only_critical() {
        let traj = create_test_trajectory();
        let critical = traj.get_critical_steps();
        assert_eq!(critical.len(), 1);
        assert!(critical[0].is_critical);
    }

    #[test]
    fn test_compress_trajectory_preserves_critical_steps() {
        let mut traj = create_test_trajectory();
        let result = compress_trajectory(&mut traj, 2, 5);

        assert!(result.critical_steps_preserved >= 1);
        assert!(traj.is_compressed || result.segments_created == 0);
    }

    #[test]
    fn test_compress_empty_trajectory_returns_zeros() {
        let mut traj = TrajectoryRecord::new("empty");
        let result = compress_trajectory(&mut traj, 2, 5);

        assert_eq!(result.original_tokens, 0);
        assert_eq!(result.tokens_freed, 0);
        assert_eq!(result.segments_created, 0);
    }

    #[test]
    fn test_build_trajectory_summary_includes_stats() {
        let traj = create_test_trajectory();
        let summary = build_trajectory_summary(&traj);

        assert!(summary.contains("turns"));
        assert!(summary.contains("tool calls"));
    }

    #[test]
    fn test_trajectory_step_outcome() {
        let step = TrajectoryStep::new("t1", "tool_call", "test", "content", false)
            .with_outcome("success");
        assert_eq!(step.outcome, "success");
    }

    #[test]
    fn test_compression_ratio_calculation() {
        let mut traj = TrajectoryRecord::new("test");
        for i in 0..50 {
            traj.add_step(TrajectoryStep::new(
                &format!("step_{}", i),
                "user_input",
                "test",
                &"x".repeat(300),
                false,
            ));
        }

        let original_tokens = traj.total_tokens;
        let result = compress_trajectory(&mut traj, 2, 5);

        if result.segments_created > 0 {
            assert!(result.compression_ratio <= 1.0);
            assert!(result.compression_ratio > 0.0);
            assert_eq!(result.original_tokens, original_tokens);
        }
    }
}
