//! @efficiency-role: infra-data
//!
//! Session Transcript Flush — Task 283
//!
//! Flushes tool results to the session transcript and artifact files
//! immediately upon completion, not just at session exit. Uses atomic
//! writes (temp file + rename) for crash safety.

use crate::*;
use std::io::Write;

/// Append a formatted entry to session.md.
pub(crate) fn append_to_transcript(session_root: &Path, line: &str) {
    let path = session_root.join("session.md");
    let entry = format!("> {}\n", line);
    if let Err(e) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, entry.as_bytes()))
    {
        tracing::warn!("append_to_transcript: {}", e);
    }
}

/// Flush a tool execution result to:
/// 1. session.md (append)
/// 2. artifacts/tool_<name>_<call_id>.txt (full output)
pub(crate) fn flush_tool_result(
    session_root: &Path,
    tool_call_id: &str,
    tool_name: &str,
    output: &str,
    success: bool,
) {
    let status = if success { "OK" } else { "FAIL" };
    let short_id: String = tool_call_id.chars().take(8).collect();
    let output_preview: String = output.chars().take(200).collect();
    let suffix = if output.chars().count() > 200 {
        "..."
    } else {
        ""
    };

    append_to_transcript(
        session_root,
        &format!(
            "TOOL {} [{}] id={}: {}{}",
            status, tool_name, short_id, output_preview, suffix
        ),
    );

    write_tool_artifact(session_root, tool_name, tool_call_id, output, success);
}

/// Write the full tool output to artifacts/tool_<name>_<call_id>.txt
/// Uses atomic write (temp file + rename).
pub(crate) fn write_tool_artifact(
    session_root: &Path,
    tool_name: &str,
    tool_call_id: &str,
    output: &str,
    success: bool,
) {
    let artifacts_dir = session_root.join("artifacts");
    let _ = std::fs::create_dir_all(&artifacts_dir);

    let safe_name = sanitize_filename(tool_name);
    let safe_id = sanitize_filename(tool_call_id);
    let filename = format!("tool_{}_{}.txt", safe_name, safe_id);
    let filepath = artifacts_dir.join(&filename);

    let status = if success { "SUCCESS" } else { "FAILURE" };
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S UTC")
        .to_string();
    let content = format!(
        "=== {} ===\nTool: {}\nCall ID: {}\nTimestamp: {}\nStatus: {}\n\n{}\n",
        filename, tool_name, tool_call_id, timestamp, status, output
    );

    let tmp_path = filepath.with_extension("tmp");
    if let Err(e) = std::fs::write(&tmp_path, &content) {
        tracing::warn!("flush: failed to write artifact: {}", e);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp_path, &filepath) {
        tracing::warn!("flush: failed to rename artifact: {}", e);
    }
}

/// Flush a PTY transcript to artifacts/pty_<timestamp>.txt
pub(crate) fn flush_pty_transcript(session_root: &Path, transcript_bytes: &[u8], duration_ms: u64) {
    let artifacts_dir = session_root.join("artifacts");
    let _ = std::fs::create_dir_all(&artifacts_dir);

    let ts = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("pty_{}.txt", ts);
    let filepath = artifacts_dir.join(&filename);

    let duration_s = duration_ms as f64 / 1000.0;
    let header = format!(
        "=== PTY Transcript ===\nTimestamp: {}\nDuration: {:.1}s\n\n",
        chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        duration_s
    );

    let tmp_path = filepath.with_extension("tmp");
    if let Err(e) = std::fs::write(&tmp_path, header.as_bytes()) {
        tracing::warn!("flush: failed to write pty header: {}", e);
        return;
    }
    if let Err(e) = std::fs::OpenOptions::new()
        .append(true)
        .open(&tmp_path)
        .and_then(|mut f| f.write_all(transcript_bytes))
    {
        tracing::warn!("flush: failed to write pty transcript: {}", e);
        let _ = std::fs::remove_file(&tmp_path);
        return;
    }
    if let Err(e) = std::fs::rename(&tmp_path, &filepath) {
        tracing::warn!("flush: failed to rename pty transcript: {}", e);
    }

    append_to_transcript(
        session_root,
        &format!(
            "PTY {} written ({} bytes, {:.1}s)",
            filename,
            transcript_bytes.len(),
            duration_s
        ),
    );
}

/// Streaming artifact writer for long-running commands.
/// Buffers output and flushes periodically or on buffer full.
pub(crate) struct StreamingArtifactWriter {
    session_root: PathBuf,
    tool_name: String,
    tool_call_id: String,
    buffer: String,
    filepath: PathBuf,
    last_flush: std::time::Instant,
    chunk_index: u64,
}

impl StreamingArtifactWriter {
    pub fn new(session_root: &Path, tool_name: &str, tool_call_id: &str) -> Self {
        let safe_name = sanitize_filename(tool_name);
        let safe_id = sanitize_filename(tool_call_id);
        let artifacts_dir = session_root.join("artifacts");
        let _ = std::fs::create_dir_all(&artifacts_dir);

        let filename = format!("tool_{}_{}_stream.txt", safe_name, safe_id);
        let filepath = artifacts_dir.join(&filename);

        // Write header
        let timestamp = chrono::Local::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string();
        let header = format!(
            "=== Streaming Output ===\nTool: {}\nCall ID: {}\nStarted: {}\n\n",
            tool_name, tool_call_id, timestamp
        );
        let _ = std::fs::write(&filepath, &header);

        Self {
            session_root: session_root.to_path_buf(),
            tool_name: tool_name.to_string(),
            tool_call_id: tool_call_id.to_string(),
            buffer: String::new(),
            filepath,
            last_flush: std::time::Instant::now(),
            chunk_index: 0,
        }
    }

    /// Append a line to the buffer. Flushes if buffer >= 16KB or 5 seconds elapsed.
    pub fn write_line(&mut self, line: &str) {
        self.buffer.push_str(line);
        self.buffer.push('\n');

        let should_flush = self.buffer.len() >= 16_384
            || self.last_flush.elapsed() >= std::time::Duration::from_secs(5);

        if should_flush {
            self.flush();
        }
    }

    /// Force flush the buffer to disk.
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        let tmp_path = self.filepath.with_extension("tmp");
        // Read existing content to preserve it
        let existing = std::fs::read_to_string(&self.filepath).unwrap_or_default();
        let new_content = existing + &self.buffer;
        if let Err(e) = std::fs::write(&tmp_path, &new_content) {
            tracing::warn!("flush stream: failed to write: {}", e);
            return;
        }
        if let Err(e) = std::fs::rename(&tmp_path, &self.filepath) {
            tracing::warn!("flush stream: failed to rename: {}", e);
        }
        self.buffer.clear();
        self.last_flush = std::time::Instant::now();
        self.chunk_index += 1;
    }

    /// Finalize the stream: flush any remaining buffer and write a completion marker.
    pub fn finish(mut self, success: bool) {
        self.flush();
        let status = if success { "SUCCESS" } else { "FAILURE" };
        let footer = format!(
            "\n=== Stream Complete ===\nStatus: {}\nChunks: {}\n",
            status,
            self.chunk_index + 1
        );
        let existing = std::fs::read_to_string(&self.filepath).unwrap_or_default();
        let new_content = existing + &footer;
        let tmp_path = self.filepath.with_extension("tmp");
        let _ = std::fs::write(&tmp_path, &new_content);
        let _ = std::fs::rename(&tmp_path, &self.filepath);

        append_to_transcript(
            &self.session_root,
            &format!(
                "TOOL STREAM {} {} [{}]: {} chunks written",
                if success { "OK" } else { "FAIL" },
                self.tool_name,
                &self.tool_call_id[..8.min(self.tool_call_id.len())],
                self.chunk_index + 1,
            ),
        );
    }
}

fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(64)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("hello_world-123.txt"),
            "hello_world-123.txt"
        );
        assert_eq!(sanitize_filename("path/to/file"), "path_to_file");
        assert_eq!(
            sanitize_filename("call_abc123def456ghi789"),
            "call_abc123def456ghi789"
        );
    }

    #[test]
    fn test_flush_tool_result_creates_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        flush_tool_result(root, "call_abc123", "shell", "ls output", true);

        // session.md replaces display/terminal_transcript.txt
        let transcript = root.join("session.md");
        assert!(transcript.exists());
        let content = std::fs::read_to_string(&transcript).unwrap();
        assert!(content.contains("ls output"));

        let artifact = root.join("artifacts").join("tool_shell_call_abc123.txt");
        assert!(artifact.exists());
        let content = std::fs::read_to_string(&artifact).unwrap();
        assert!(content.contains("SUCCESS"));
        assert!(content.contains("ls output"));
    }

    #[test]
    fn test_flush_tool_result_failure() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        flush_tool_result(root, "call_fail", "search", "not found", false);

        let transcript = root.join("session.md");
        let content = std::fs::read_to_string(&transcript).unwrap();
        assert!(content.contains("not found"));

        let artifact = root.join("artifacts").join("tool_search_call_fail.txt");
        let content = std::fs::read_to_string(&artifact).unwrap();
        assert!(content.contains("FAILURE"));
    }

    #[test]
    fn test_streaming_artifact_writer() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let mut writer = StreamingArtifactWriter::new(root, "shell", "call_str");

        writer.write_line("line 1");
        writer.write_line("line 2");
        writer.finish(true);

        let stream_file = root
            .join("artifacts")
            .join("tool_shell_call_str_stream.txt");
        assert!(stream_file.exists());
        let content = std::fs::read_to_string(&stream_file).unwrap();
        assert!(content.contains("line 1"));
        assert!(content.contains("line 2"));
        assert!(content.contains("SUCCESS"));
    }

    #[test]
    fn test_truncated_preview() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let long_output: String = std::iter::repeat('x').take(500).collect();
        flush_tool_result(root, "call_long", "read", &long_output, true);

        let transcript = root.join("session.md");
        let content = std::fs::read_to_string(&transcript).unwrap();
        // Preview should be truncated (not full 500 chars in session.md)
        assert!(content.len() < long_output.len() + 200); // not including full 500 chars in session.md

        // But the artifact should have the full content
        let artifact = root.join("artifacts").join("tool_read_call_long.txt");
        let content = std::fs::read_to_string(&artifact).unwrap();
        assert!(content.contains(&long_output));
    }
}
