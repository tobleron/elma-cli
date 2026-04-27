//! @efficiency-role: data-model
//!
//! Session Display Capture - Captures user-visible terminal output for debugging.
//! Saves what the user actually sees on the terminal.

use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};

static DISPLAY_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone)]
pub(crate) enum DisplayEntry {
    Tool {
        name: String,
        command: String,
        output: String,
        success: bool,
    },
    Thinking {
        content: String,
    },
    FinalAnswer {
        content: String,
    },
    UserPrompt {
        content: String,
    },
}

pub(crate) fn save_display_entry(session: &SessionPaths, entry: DisplayEntry) -> Result<PathBuf> {
    let display_dir = &session.display_dir;
    let counter = DISPLAY_COUNTER.fetch_add(1, Ordering::SeqCst);
    let (filename, content) = match entry {
        DisplayEntry::Tool {
            name,
            command,
            output,
            success,
        } => (
            format!(
                "{:04}_tool_{}_{}.txt",
                counter,
                name,
                if success { "success" } else { "fail" }
            ),
            format!(
                "=== Tool: {} ===\nCommand: {}\n\nOutput:\n{}\n",
                name, command, output
            ),
        ),
        DisplayEntry::Thinking { content } => (
            format!("{:04}_thinking.txt", counter),
            format!("=== Thinking ===\n{}\n", content),
        ),
        DisplayEntry::FinalAnswer { content } => (
            format!("{:04}_final_answer.txt", counter),
            format!("=== Final Answer ===\n{}\n", content),
        ),
        DisplayEntry::UserPrompt { content } => (
            format!("{:04}_user_prompt.txt", counter),
            format!("=== User Prompt ===\n{}\n", content),
        ),
    };

    let path = display_dir.join(&filename);
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

pub(crate) fn save_tool_display(
    session: &SessionPaths,
    name: &str,
    command: &str,
    output: &str,
    success: bool,
) -> Result<PathBuf> {
    save_display_entry(
        session,
        DisplayEntry::Tool {
            name: name.to_string(),
            command: command.to_string(),
            output: output.to_string(),
            success,
        },
    )
}

pub(crate) fn save_thinking_display(session: &SessionPaths, content: &str) -> Result<PathBuf> {
    save_display_entry(
        session,
        DisplayEntry::Thinking {
            content: content.to_string(),
        },
    )
}

pub(crate) fn save_final_answer_display(session: &SessionPaths, content: &str) -> Result<PathBuf> {
    save_display_entry(
        session,
        DisplayEntry::FinalAnswer {
            content: content.to_string(),
        },
    )
}

pub(crate) fn save_user_prompt_display(session: &SessionPaths, content: &str) -> Result<PathBuf> {
    save_display_entry(
        session,
        DisplayEntry::UserPrompt {
            content: content.to_string(),
        },
    )
}
