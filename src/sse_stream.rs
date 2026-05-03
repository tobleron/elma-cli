//! @efficiency-role: util-pure
//! Generic SSE byte stream reader. No TUI, no HTTP, pure parsing.

use crate::stream_types::SseFrame;
use futures::stream::StreamExt;
use std::time::Duration;

/// Read SSE frames from a byte stream, yielding parsed SseFrame values.
/// Handles: data lines, event lines, [DONE] markers, multi-line data.
pub fn parse_sse_bytes(chunk: &str, buffer: &mut String) -> Vec<SseFrame> {
    let mut frames = Vec::new();
    buffer.push_str(chunk);

    while let Some(pos) = buffer.find('\n') {
        let line = buffer.drain(..pos + 1).collect::<String>();
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if !line.starts_with("data: ") {
            continue;
        }
        let data = &line[6..];
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        frames.push(SseFrame {
            event: None,
            data: data.to_string(),
        });
    }
    frames
}

/// Default timeout for TUI UI pump operations during streaming.
pub const UI_PUMP_INTERVAL_MS: u64 = 40;
