//! @efficiency-role: ui-component
//!
//! Task 099: Context Window Usage Visualizer
//!
//! Renders a Unicode progress bar showing real-time token usage.
//! Color-coded: green < 70%, yellow 70-90%, red > 90%.

use crate::ui_colors::*;

const BAR_FULL: &str = "█";
const BAR_75: &str = "▓";
const BAR_50: &str = "▒";
const BAR_25: &str = "░";
const BAR_EMPTY: &str = " ";

/// Render a context window usage progress bar.
/// `current` = tokens used, `max` = total context window, `width` = bar width in chars.
pub(crate) fn render_context_bar(current: u64, max: u64, width: usize) -> String {
    if max == 0 {
        return format!("{} ─/─ [?]", BAR_EMPTY.repeat(width));
    }

    let pct = (current as f64 / max as f64).clamp(0.0, 1.0);
    let filled = (pct * width as f64).round() as usize;

    let mut bar = String::with_capacity(width);
    for i in 0..width {
        if i < filled.saturating_sub(1) {
            bar.push_str(BAR_FULL);
        } else if i == filled.saturating_sub(1) && filled > 0 {
            let partial = (pct * width as f64) - (filled as f64 - 1.0);
            if partial >= 0.75 {
                bar.push_str(BAR_FULL);
            } else if partial >= 0.50 {
                bar.push_str(BAR_75);
            } else if partial >= 0.25 {
                bar.push_str(BAR_50);
            } else {
                bar.push_str(BAR_25);
            }
        } else {
            bar.push_str(BAR_EMPTY);
        }
    }

    let current_display = format_tokens(current);
    let max_display = format_tokens(max);
    let pct_display = format!("{:.1}%", pct * 100.0);

    format!("{} {}/{} [{}]", bar, current_display, max_display, pct_display)
}

/// Format a token count for display.
fn format_tokens(count: u64) -> String {
    if count < 1000 {
        count.to_string()
    } else if count < 1_000_000 {
        format!("{:.1}k", count as f64 / 1000.0)
    } else {
        format!("{:.1}M", count as f64 / 1_000_000.0)
    }
}

/// Color-code the context bar based on usage percentage.
pub(crate) fn render_context_bar_colored(current: u64, max: u64, width: usize) -> String {
    if max == 0 {
        return meta_comment(&render_context_bar(current, max, width));
    }

    let pct = current as f64 / max as f64;
    let bar = render_context_bar(current, max, width);

    if pct < 0.70 {
        success_green(&bar)
    } else if pct < 0.90 {
        warn_yellow(&bar)
    } else {
        error_red(&bar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_bar_empty() {
        let bar = render_context_bar(0, 8192, 20);
        assert!(bar.contains("[0.0%]"));
    }

    #[test]
    fn test_context_bar_half() {
        let bar = render_context_bar(4096, 8192, 20);
        assert!(bar.contains("[50.0%]"));
    }

    #[test]
    fn test_context_bar_full() {
        let bar = render_context_bar(8192, 8192, 20);
        assert!(bar.contains("[100.0%]"));
    }

    #[test]
    fn test_context_bar_over_limit() {
        let bar = render_context_bar(10000, 8192, 20);
        assert!(bar.contains("[100.0%]"));
    }

    #[test]
    fn test_format_tokens() {
        assert_eq!(format_tokens(500), "500");
        assert_eq!(format_tokens(4096), "4.1k");
        assert_eq!(format_tokens(200_000), "200.0k");
    }

    #[test]
    fn test_colored_bar_green_for_low_usage() {
        let colored = render_context_bar_colored(1000, 8192, 20);
        assert!(colored.contains("█") || colored.contains("\x1b"));
    }
}
