# T210: Thinking Auto-Collapse Timer and Visual Indicators

## Status
`pending`

## Problem
The thinking stream currently has no visual animation during streaming and no word-count-based auto-collapse timer. When thinking is collapsed, users see a static label with no indication of collapse timing. Long thinking processes should take proportionally longer to collapse.

## Two Issues To Fix

### Issue 1: Visual Indicators
The thinking row needs distinct visual states:
- **Streaming (expanded)**: Box rotation animation (like a spinner) on the left of the thinking content
- **Post-stream collapsed**: `> Thinking..` prefix (right arrow, implies expandable)
- **Post-stream expanded**: `▸ (ctrl+o to collapse)` prefix (down arrow, implies collapsible)

The animation should use a Unicode spinner (available as `SPINNER_FRAMES` in `ui_theme.rs`).

### Issue 2: Word-Count-Based Auto-Collapse
Current auto-collapse uses a fixed short timeout (`thinking_collapse_deadline`). This should be replaced with a word-count calculation:
- Reading speed: 300 WPM (fast reader)
- Collapse delay = (word_count / 300) * 60 seconds
- Example: 150 words → 30 seconds, 60 words → 12 seconds, 15 words → 3 seconds (minimum)

The system should accumulate thinking content word count in real-time as chunks arrive, and set the collapse deadline based on the final count when streaming finishes.

## Implementation Boundary

### Visual Indicators (Issue 1)
1. In `ClaudeMessage::Thinking` rendering (`claude_state.rs`, `to_lines` method):
   - Add a `streaming` parameter to `to_lines` or check a new `is_streaming` flag
   - When `streaming == true`: show spinner animation frame instead of content
   - When `expanded == true && streaming == false`: show `▸ (ctrl+o to collapse)` prefix
   - When `expanded == false`: show `> Thinking..` prefix

2. Track streaming state per thinking message:
   - Add `is_streaming: bool` field to `ClaudeMessage::Thinking`
   - Set `is_streaming = true` when `start_thinking()` is called
   - Set `is_streaming = false` when `ThinkingFinished` event fires

3. Spinner animation in the legacy renderer:
   - Use existing `SPINNER_FRAMES` from `ui_theme.rs`
   - Advance frame on each pump/render cycle during streaming

### Word-Count Auto-Collapse (Issue 2)
1. In `claude_state.rs`, add word counting to thinking accumulation:
   ```rust
   fn count_words(text: &str) -> usize {
       text.split_whitespace().count()
   }
   ```

2. Update `append_thinking` to track word count:
   - Add `streaming_word_count: usize` field to `ClaudeMessage::Thinking`
   - Increment word count as each chunk arrives

3. Calculate collapse deadline when `ThinkingFinished`:
   - `delay_secs = (word_count as f64 / 300.0 * 60.0).max(3.0).min(60.0)`
   - Set `thinking_collapse_deadline = Some((index, Instant::now() + delay))`

4. Update `thinking_expanded_for_index` to use the word-count-based deadline (already partially implemented — ensure it uses the calculated delay).

5. In `claude_ui_stream.rs`, call the word-count update on each thinking delta.

## Files To Change
- `src/claude_ui/claude_state.rs` — add `is_streaming`, `streaming_word_count` fields, update `to_lines`, update `thinking_expanded_for_index`
- `src/claude_ui/claude_stream.rs` — track streaming state per thinking message
- `src/claude_ui/claude_render.rs` — spinner rendering for streaming thinking
- `src/ui/ui_render_legacy.rs` — update thinking collapsed label to `> Thinking..`
- `src/ui/ui_state.rs` — already has the state machinery, may need toggle helper

## Verification
- `cargo build` passes
- Short thinking (10 words) collapses in ~2 seconds
- Long thinking (500 words) collapses in ~100 seconds
- Streaming thinking shows animated spinner
- Collapsed thinking shows `> Thinking..`
- Expanded thinking shows `▸ (ctrl+o to collapse)`
- Click on collapsed thinking row expands it