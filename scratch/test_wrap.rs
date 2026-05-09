fn word_wrap_lines(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.split('\n') {
        let mut remaining = para;
        while remaining.chars().count() > width {
            // Find the byte index for the 'width' character safely
            let byte_pos = remaining
                .char_indices()
                .nth(width)
                .map(|(i, _)| i)
                .unwrap_or(remaining.len());

            let mut split_at_byte = byte_pos;

            // Try to break at last space within the char width
            if let Some(pos) = remaining[..byte_pos].rfind(' ') {
                split_at_byte = pos;
            }

            // But if no space found, break at byte_pos
            if split_at_byte == 0 {
                split_at_byte = byte_pos;
            }

            let left = &remaining[..split_at_byte];
            lines.push(left.trim_end().to_string());
            remaining = remaining[split_at_byte..].trim_start();
        }
        if !remaining.is_empty() {
            lines.push(remaining.to_string());
        }
    }
    lines
}

fn main() {
    let text = "suited for small models. The evidence ledger ensures every claim traces back to collected data — this is the foundation of truth-grounded answers.";
    let width = 96;
    let lines = word_wrap_lines(text, width);
    for line in lines {
        println!("LINE ({} chars): {}", line.chars().count(), line);
    }
    
    // Test with multi-byte char exactly at split
    let text2 = "This is a test with a multi-byte character — right here.";
    let width2 = 43; // '—' is at char 43
    let lines2 = word_wrap_lines(text2, width2);
    for line in lines2 {
        println!("LINE2 ({} chars): {}", line.chars().count(), line);
    }
}
