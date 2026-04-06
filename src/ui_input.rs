//! @efficiency-role: ui-component
//!
//! Multi-line input editor with full cursor-based editing, history, and navigation.
//!
//! Features:
//! - Multi-line buffer (grows from 1 to max_lines)
//! - Cursor tracking (row + byte-column)
//! - Navigation: Left, Right, Home, End, Up, Down, Ctrl+Left/Right (word jump)
//! - Editing: insert char, backspace, delete, Ctrl+W (delete word), Ctrl+U (delete to line start)
//! - History: Up/Down cycles through persisted entries, stashing current input
//! - Display-width-aware cursor positioning (Unicode-aware)

use crate::ui_wrap::display_width;
use std::path::PathBuf;

/// Input history loaded from disk.
const MAX_HISTORY_ENTRIES: usize = 500;

/// A multi-line text input buffer with cursor and history support.
#[derive(Clone, Debug)]
pub(crate) struct TextInput {
    /// Lines of text. Always has at least one entry (may be empty string).
    lines: Vec<String>,
    /// Current cursor row (0-based).
    cursor_row: usize,
    /// Current cursor column — byte offset within the current line.
    cursor_col: usize,
    /// Input history entries (oldest first).
    history: Vec<String>,
    /// Index into history for cycling. When `history_index == history.len()`, we're at the "new" position.
    history_index: usize,
    /// Stashed input — saved when entering history mode, restored when exiting.
    stashed: String,
    /// Maximum number of visible lines (input grows up to this height).
    max_lines: usize,
}

impl TextInput {
    /// Create a new empty input with the given max height.
    pub(crate) fn new(max_lines: usize) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            history: Vec::new(),
            history_index: 0,
            stashed: String::new(),
            max_lines: max_lines.max(1),
        }
    }

    // --- Accessors ---

    /// Get the full text content (lines joined with newlines).
    pub(crate) fn content(&self) -> String {
        self.lines.join("\n")
    }

    /// Get the content trimmed (for submission).
    pub(crate) fn content_trimmed(&self) -> String {
        self.content().trim().to_string()
    }

    /// Whether the input is empty (all lines empty).
    pub(crate) fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }

    /// Number of lines in the buffer.
    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Current cursor row.
    pub(crate) fn cursor_row(&self) -> usize {
        self.cursor_row
    }

    /// Current cursor column (byte offset).
    pub(crate) fn cursor_col(&self) -> usize {
        self.cursor_col
    }

    /// Display column of cursor (accounts for multi-byte characters).
    pub(crate) fn display_col(&self) -> usize {
        let line = self.current_line();
        let byte_pos = self.cursor_col.min(line.len());
        display_width(&line[..byte_pos])
    }

    /// Current line content (borrowed).
    pub(crate) fn current_line(&self) -> &str {
        self.lines
            .get(self.cursor_row)
            .map(|s| s.as_str())
            .unwrap_or("")
    }

    /// All lines (borrowed).
    pub(crate) fn lines(&self) -> &[String] {
        &self.lines
    }

    // --- History ---

    /// Load history from a file. Called at startup.
    pub(crate) fn load_history(&mut self, path: &PathBuf) {
        if let Ok(content) = std::fs::read_to_string(path) {
            let entries: Vec<String> = content
                .lines()
                .filter(|l| !l.is_empty())
                .map(|l| l.to_string())
                .take(MAX_HISTORY_ENTRIES)
                .collect();
            self.history = entries;
            self.history_index = self.history.len();
        }
    }

    /// Save history to a file. Called on exit or after adding entries.
    pub(crate) fn save_history(&self, path: &PathBuf) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let content = self.history.join("\n");
        let _ = std::fs::write(path, content);
    }

    /// Add a non-empty input to history. Called on successful submission.
    pub(crate) fn push_to_history(&mut self) {
        let content = self.content_trimmed();
        if !content.is_empty() {
            self.history.push(content);
            // Trim oldest if over limit.
            if self.history.len() > MAX_HISTORY_ENTRIES {
                self.history.remove(0);
            }
            self.history_index = self.history.len();
        }
    }

    /// Move up in history (older entry). Stash current input on first use.
    /// Returns true if the cursor moved.
    pub(crate) fn history_up(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        // Stash current input on first history navigation.
        if self.history_index == self.history.len() {
            self.stashed = self.content();
        }
        if self.history_index > 0 {
            self.history_index -= 1;
            self.load_from_history_entry();
            true
        } else {
            false
        }
    }

    /// Move down in history (newer entry).
    /// Returns true if the cursor moved.
    pub(crate) fn history_down(&mut self) -> bool {
        if self.history_index >= self.history.len() {
            return false;
        }
        self.history_index += 1;
        if self.history_index == self.history.len() {
            // Restore stashed input.
            let stashed = std::mem::take(&mut self.stashed);
            self.set_content(&stashed);
        } else {
            self.load_from_history_entry();
        }
        true
    }

    fn load_from_history_entry(&mut self) {
        if self.history_index < self.history.len() {
            let entry = self.history[self.history_index].clone();
            self.set_content(&entry);
        }
    }

    /// Check if currently viewing history (not the stashed/new entry).
    pub(crate) fn is_in_history(&self) -> bool {
        self.history_index < self.history.len()
    }

    // --- Content manipulation ---

    /// Set the full content from a string. Resets cursor to end.
    pub(crate) fn set_content(&mut self, content: &str) {
        self.lines = content.lines().map(|s| s.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        // Clamp to max_lines.
        while self.lines.len() > self.max_lines {
            // Merge extra lines into the last visible line.
            let last = self.lines.len() - 1;
            let extra = self.lines[last + 1..].join(" ");
            self.lines[last].push_str(&extra);
            self.lines.truncate(self.max_lines);
        }
        self.cursor_row = self.lines.len().saturating_sub(1);
        self.cursor_col = self.lines.last().map(|s| s.len()).unwrap_or(0);
    }

    /// Clear all input.
    pub(crate) fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.stashed.clear();
    }

    /// Insert a character at the cursor position.
    pub(crate) fn insert_char(&mut self, c: char) {
        if self.cursor_row < self.lines.len() {
            let line = &mut self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            line.insert(col, c);
            self.cursor_col = col + c.len_utf8();
        }
    }

    /// Insert a newline (split current line or add new line).
    pub(crate) fn insert_newline(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &mut self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            let remainder = line[col..].to_string();
            line.truncate(col);
            self.lines.insert(self.cursor_row + 1, remainder);
            // Clamp to max_lines.
            if self.lines.len() > self.max_lines {
                self.lines.pop();
            }
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    /// Delete character before cursor (backspace).
    /// Returns true if something was deleted.
    pub(crate) fn backspace(&mut self) -> bool {
        if self.cursor_col > 0 && self.cursor_row < self.lines.len() {
            let line = &mut self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            // Find the start of the previous character (handle multi-byte).
            let new_col = prev_char_boundary(line, col);
            line.replace_range(new_col..col, "");
            self.cursor_col = new_col;
            return true;
        }
        // Merge with previous line if at start of line (and not first line).
        if self.cursor_col == 0 && self.cursor_row > 0 && self.lines.len() > 1 {
            let prev_len = self.lines[self.cursor_row - 1].len();
            let current = self.lines.remove(self.cursor_row);
            self.lines[self.cursor_row - 1].push_str(&current);
            self.cursor_row -= 1;
            self.cursor_col = prev_len;
            return true;
        }
        false
    }

    /// Delete character at cursor (delete key).
    /// Returns true if something was deleted.
    pub(crate) fn delete(&mut self) -> bool {
        if self.cursor_row < self.lines.len() {
            let line = &mut self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            if col < line.len() {
                let new_col = next_char_boundary(line, col);
                line.replace_range(col..new_col, "");
                return true;
            }
            // Merge with next line if at end of line (and not last line).
            if col == line.len() && self.cursor_row + 1 < self.lines.len() {
                let next = self.lines.remove(self.cursor_row + 1);
                self.lines[self.cursor_row].push_str(&next);
                return true;
            }
        }
        false
    }

    /// Delete word before cursor (Ctrl+W).
    pub(crate) fn delete_word_before(&mut self) {
        if self.cursor_row >= self.lines.len() {
            return;
        }
        let line = &mut self.lines[self.cursor_row];
        let col = self.cursor_col.min(line.len());
        let word_start = find_word_start_before(line, col);
        self.lines[self.cursor_row].replace_range(word_start..col, "");
        self.cursor_col = word_start;
    }

    /// Delete to start of line (Ctrl+U).
    pub(crate) fn delete_to_line_start(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line_len = self.lines[self.cursor_row].len();
            let col = self.cursor_col.min(line_len);
            self.lines[self.cursor_row].replace_range(..col, "");
            self.cursor_col = 0;
        }
    }

    // --- Navigation ---

    /// Move cursor left by one character.
    pub(crate) fn move_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            self.cursor_col = prev_char_boundary(line, self.cursor_col.min(line.len()));
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    /// Move cursor right by one character.
    pub(crate) fn move_right(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            if col < line.len() {
                self.cursor_col = next_char_boundary(line, col);
            } else if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
        }
    }

    /// Move cursor to start of line (Home).
    pub(crate) fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line (End).
    pub(crate) fn move_end(&mut self) {
        if self.cursor_row < self.lines.len() {
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    /// Move cursor up one line.
    pub(crate) fn move_up(&mut self) {
        if self.cursor_row > 0 {
            let current_display_col = self.display_col();
            self.cursor_row -= 1;
            // Preserve display column (not byte column) for proportional positioning.
            self.cursor_col =
                byte_col_for_display_col(&self.lines[self.cursor_row], current_display_col);
        }
    }

    /// Move cursor down one line.
    pub(crate) fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            let current_display_col = self.display_col();
            self.cursor_row += 1;
            self.cursor_col =
                byte_col_for_display_col(&self.lines[self.cursor_row], current_display_col);
        }
    }

    /// Move cursor left by one word (Ctrl+Left).
    pub(crate) fn move_word_left(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            self.cursor_col = find_word_start_before(line, col);
            if self.cursor_col == 0 && self.cursor_row > 0 {
                self.cursor_row -= 1;
                self.cursor_col = self.lines[self.cursor_row].len();
            }
        }
    }

    /// Move cursor right by one word (Ctrl+Right).
    pub(crate) fn move_word_right(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            let col = self.cursor_col.min(line.len());
            let next = find_word_end(line, col);
            if next > col {
                self.cursor_col = next;
            } else if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
        }
    }
}

// --- Character boundary helpers ---

/// Find the byte offset of the character before the given position.
fn prev_char_boundary(s: &str, byte_pos: usize) -> usize {
    if byte_pos == 0 {
        return 0;
    }
    let mut pos = byte_pos - 1;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

/// Find the byte offset of the character after the given position.
fn next_char_boundary(s: &str, byte_pos: usize) -> usize {
    let mut pos = byte_pos + 1;
    while pos < s.len() && !s.is_char_boundary(pos) {
        pos += 1;
    }
    pos.min(s.len())
}

/// Find the start of the word before the given byte position.
fn find_word_start_before(s: &str, byte_pos: usize) -> usize {
    if byte_pos == 0 {
        return 0;
    }
    // Collect all chars before byte_pos.
    let chars: Vec<(usize, char)> = s.char_indices().filter(|(i, _)| *i < byte_pos).collect();
    if chars.is_empty() {
        return 0;
    }
    // Walk backwards to find the end of the previous word (skip non-word chars).
    let mut i = chars.len();
    while i > 0 {
        i -= 1;
        let c = chars[i].1;
        if c.is_alphanumeric() || c == '_' {
            // Found a word char — walk back to find the start of this word.
            while i > 0 {
                let prev_c = chars[i - 1].1;
                if !prev_c.is_alphanumeric() && prev_c != '_' {
                    break;
                }
                i -= 1;
            }
            return chars[i].0;
        }
    }
    0
}

/// Find the end of the word starting from the given byte position.
fn find_word_end(s: &str, byte_pos: usize) -> usize {
    let mut found_word = false;
    for (i, c) in s.char_indices().skip_while(|(i, _)| *i < byte_pos) {
        if c.is_alphanumeric() || c == '_' {
            found_word = true;
        } else if found_word {
            return i;
        }
    }
    s.len()
}

/// Convert a target display column to a byte column in the given string.
fn byte_col_for_display_col(s: &str, target_display_col: usize) -> usize {
    let mut byte_pos = 0;
    let mut display_pos = 0;
    for (i, c) in s.char_indices() {
        if display_pos >= target_display_col {
            break;
        }
        byte_pos = i;
        display_pos += unicode_width::UnicodeWidthChar::width(c).unwrap_or(0) as usize;
    }
    // Check if we should go one character further.
    if display_pos < target_display_col {
        byte_pos = s.len();
    }
    byte_pos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_input_is_empty() {
        let input = TextInput::new(3);
        assert!(input.is_empty());
        assert_eq!(input.content(), "");
        assert_eq!(input.cursor_row, 0);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_insert_chars() {
        let mut input = TextInput::new(3);
        input.insert_char('h');
        input.insert_char('i');
        assert_eq!(input.content(), "hi");
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_backspace() {
        let mut input = TextInput::new(3);
        input.insert_char('h');
        input.insert_char('i');
        assert!(input.backspace());
        assert_eq!(input.content(), "h");
        assert!(input.backspace());
        assert_eq!(input.content(), "");
        assert!(!input.backspace()); // nothing to delete
    }

    #[test]
    fn test_delete() {
        let mut input = TextInput::new(3);
        input.insert_char('h');
        input.insert_char('i');
        input.move_left();
        assert!(input.delete());
        assert_eq!(input.content(), "h");
    }

    #[test]
    fn test_cursor_movement() {
        let mut input = TextInput::new(3);
        input.insert_char('h');
        input.insert_char('i');
        input.move_left();
        assert_eq!(input.cursor_col, 1);
        input.move_left();
        assert_eq!(input.cursor_col, 0);
        input.move_home();
        assert_eq!(input.cursor_col, 0);
        input.move_end();
        assert_eq!(input.cursor_col, 2);
    }

    #[test]
    fn test_word_deletion() {
        let mut input = TextInput::new(3);
        for c in "hello world".chars() {
            input.insert_char(c);
        }
        input.delete_word_before();
        assert_eq!(input.content(), "hello ");
        input.delete_to_line_start();
        assert_eq!(input.content(), "");
    }

    #[test]
    fn test_word_navigation() {
        let mut input = TextInput::new(3);
        for c in "hello world foo".chars() {
            input.insert_char(c);
        }
        input.move_word_left(); // before "foo"
        assert_eq!(input.cursor_col, 12);
        input.move_word_left(); // before "world"
        assert_eq!(input.cursor_col, 6);
        input.move_word_right(); // after "world"
        assert_eq!(input.cursor_col, 11);
    }

    #[test]
    fn test_history_push_and_cycle() {
        let mut input = TextInput::new(3);
        for c in "first".chars() {
            input.insert_char(c);
        }
        input.push_to_history();
        input.clear();
        for c in "second".chars() {
            input.insert_char(c);
        }
        input.push_to_history();

        assert_eq!(input.history.len(), 2);
        assert_eq!(input.history_index, 2);

        input.history_up();
        assert_eq!(input.content(), "second");
        assert_eq!(input.history_index, 1);

        input.history_up();
        assert_eq!(input.content(), "first");
        assert_eq!(input.history_index, 0);

        input.history_down();
        assert_eq!(input.content(), "second");
        assert_eq!(input.history_index, 1);
    }

    #[test]
    fn test_newline_splits_line() {
        let mut input = TextInput::new(5);
        for c in "hello world".chars() {
            input.insert_char(c);
        }
        input.cursor_col = 5; // after "hello"
        input.insert_newline();
        assert_eq!(input.line_count(), 2);
        assert_eq!(input.lines[0], "hello");
        assert_eq!(input.lines[1], " world");
        assert_eq!(input.cursor_row, 1);
        assert_eq!(input.cursor_col, 0);
    }

    #[test]
    fn test_multibyte_cursor() {
        let mut input = TextInput::new(3);
        // "αβγ" — each Greek letter is 2 bytes.
        input.insert_char('α');
        input.insert_char('β');
        input.insert_char('γ');
        assert_eq!(input.content().len(), 6); // 3 chars × 2 bytes
        assert_eq!(input.display_col(), 3); // 3 display columns
        input.move_left();
        assert_eq!(input.display_col(), 2);
        input.move_left();
        assert_eq!(input.display_col(), 1);
    }

    #[test]
    fn test_history_persistence_across_set_content() {
        let mut input = TextInput::new(3);
        for c in "test message".chars() {
            input.insert_char(c);
        }
        input.push_to_history();
        assert_eq!(input.history.len(), 1);

        input.clear();
        for c in "another".chars() {
            input.insert_char(c);
        }
        input.push_to_history();
        assert_eq!(input.history.len(), 2);
    }

    #[test]
    fn test_max_lines_clamping() {
        let mut input = TextInput::new(3);
        input.set_content("line1\nline2\nline3\nline4\nline5");
        assert_eq!(input.line_count(), 3); // clamped to max_lines
    }
}
