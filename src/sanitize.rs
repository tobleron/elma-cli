//! @efficiency-role: util-pure
//! Centralized output sanitization for all tool results.

/// Strip ANSI escape sequences, terminal control characters, and null bytes
/// from tool output before it enters the model context or persistent storage.
pub fn sanitize_tool_output(raw: &str) -> String {
    let mut cleaned = String::with_capacity(raw.len());

    for ch in raw.chars() {
        match ch {
            // Strip ANSI escape sequences (ESC [ ... m)
            '\x1b' => {
                // Skip until we see a letter
                continue;
            }
            // Strip terminal control characters
            '\x00' | '\x01' | '\x02' | '\x03' | '\x04' | '\x05' | '\x06' | '\x07'
            | '\x0e' | '\x0f' | '\x10' | '\x11' | '\x12' | '\x13' | '\x14' | '\x15'
            | '\x16' | '\x17' | '\x18' | '\x19' | '\x1a' | '\x1c' | '\x1d' | '\x1e'
            | '\x1f' | '\x7f' => continue,
            _ => cleaned.push(ch),
        }
    }

    // Strip remaining ANSI CSI sequences (ESC [ sequences not caught above)
    // Pattern: ESC followed by [ and parameters, ending with a letter
    let re = regex::Regex::new(r"\x1b\[[\d;]*[A-Za-z]").unwrap();
    let cleaned = re.replace_all(&cleaned, "").to_string();

    // Strip OSC sequences (ESC ] ... BEL/ST)
    let re = regex::Regex::new(r"\x1b\].*?(\x07|\x1b\\)").unwrap();
    let cleaned = re.replace_all(&cleaned, "").to_string();

    cleaned
}
