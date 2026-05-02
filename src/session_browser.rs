use crate::session_index::{build_index, count_session_artifacts, SessionIndex, SessionIndexEntry};
use crate::session_write::load_session_doc;
use crate::*;
use std::io::Read;
use std::path::PathBuf;

/// Entry shown in the session picker dialog.
#[derive(Debug, Clone)]
pub(crate) struct SessionPickerEntry {
    pub(crate) id: String,
    pub(crate) path: PathBuf,
    pub(crate) status: String,
    pub(crate) created_at_unix: u64,
    pub(crate) last_modified_unix: u64,
    pub(crate) artifact_count: u32,
    pub(crate) model: Option<String>,
    pub(crate) workspace_root: Option<String>,
    pub(crate) preview: String,
    pub(crate) is_current: bool,
    pub(crate) resumable: bool,
    pub(crate) warning: Option<String>,
}

/// Load session picker entries from the index + session.json metadata.
pub(crate) fn load_session_picker_entries(
    sessions_root: &PathBuf,
    current_id: Option<&str>,
) -> Vec<SessionPickerEntry> {
    let index = match rebuild_if_needed(sessions_root) {
        Some(idx) => idx,
        None => return Vec::new(),
    };

    index
        .sessions
        .iter()
        .map(|entry| build_picker_entry(sessions_root, entry, current_id))
        .collect()
}

/// Force rebuild the session index and return it.
pub(crate) fn rebuild_session_index(sessions_root: &PathBuf) -> Vec<SessionPickerEntry> {
    let index = match build_index(sessions_root) {
        Ok(idx) => idx,
        Err(_) => return Vec::new(),
    };
    let _ = index.save(sessions_root);
    // Convert to entries with no current marker
    index
        .sessions
        .iter()
        .map(|entry| build_picker_entry(sessions_root, entry, None))
        .collect()
}

fn rebuild_if_needed(sessions_root: &PathBuf) -> Option<SessionIndex> {
    let index = SessionIndex::load(sessions_root).ok()?;
    if index.sessions.is_empty() && sessions_root.exists() {
        // Try building from scratch
        if let Ok(built) = build_index(sessions_root) {
            let _ = built.save(sessions_root);
            return Some(built);
        }
    }
    Some(index)
}

fn build_picker_entry(
    sessions_root: &PathBuf,
    entry: &SessionIndexEntry,
    current_id: Option<&str>,
) -> SessionPickerEntry {
    let session_dir = sessions_root.join(&entry.id);

    // Read session.json for model and other metadata
    let doc = load_session_doc(&session_dir);
    let model = doc
        .get("runtime")
        .and_then(|r| r.get("model"))
        .and_then(|m| m.as_str())
        .map(|s| s.to_string());
    let workspace_root = doc
        .get("runtime")
        .and_then(|r| r.get("workspace_root"))
        .and_then(|w| w.as_str())
        .map(|s| s.to_string());

    // Compute preview from session.md (first line of content)
    let preview = read_session_preview(&session_dir);
    let is_current = current_id.map_or(false, |curr| entry.id == curr);

    // Check resumability
    let session_md = session_dir.join("session.md");
    let has_valid_state = entry.status != "error" && entry.status != "unknown";
    let resumable = has_valid_state && (session_md.exists() || has_legacy_transcript(&session_dir));

    let warning = if !resumable && has_legacy_transcript(&session_dir) {
        Some("session.md missing".to_string())
    } else if !resumable {
        Some("no transcript".to_string())
    } else {
        None
    };

    SessionPickerEntry {
        id: entry.id.clone(),
        path: session_dir,
        status: entry.status.clone(),
        created_at_unix: entry.created_at_unix,
        last_modified_unix: entry.last_modified_unix,
        artifact_count: entry.artifact_count,
        model,
        workspace_root,
        preview,
        is_current,
        resumable,
        warning,
    }
}

fn read_session_preview(session_dir: &PathBuf) -> String {
    let md_path = session_dir.join("session.md");
    if md_path.exists() {
        if let Ok(mut f) = std::fs::File::open(&md_path) {
            let mut buf = [0u8; 200];
            match f.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let s = String::from_utf8_lossy(&buf[..n]).to_string();
                    // Take first line, strip markdown formatting
                    let line = s.lines().next().unwrap_or("").to_string();
                    let clean = strip_preview_md(&line);
                    if clean.len() > 80 {
                        format!("{}…", &clean[..80])
                    } else {
                        clean
                    }
                }
                _ => String::new(),
            }
        } else {
            String::new()
        }
    } else {
        // Legacy fallback
        let legacy = session_dir.join("display").join("terminal_transcript.txt");
        if legacy.exists() {
            if let Ok(mut f) = std::fs::File::open(&legacy) {
                let mut buf = [0u8; 200];
                match f.read(&mut buf) {
                    Ok(n) if n > 0 => {
                        let s = String::from_utf8_lossy(&buf[..n]).to_string();
                        let line = s.lines().next().unwrap_or("").to_string();
                        if line.len() > 80 {
                            format!("{}…", &line[..80])
                        } else {
                            line
                        }
                    }
                    _ => String::new(),
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    }
}

fn has_legacy_transcript(session_dir: &PathBuf) -> bool {
    session_dir
        .join("display")
        .join("terminal_transcript.txt")
        .exists()
}

fn strip_preview_md(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_bracket = false;
    for c in s.chars() {
        match c {
            '[' => {
                in_bracket = true;
            }
            ']' => {
                in_bracket = false;
            }
            '*' | '`' | '_' | '#' | '>' => {
                // skip markdown formatting
            }
            _ => {
                if !in_bracket {
                    out.push(c);
                }
            }
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_preview_md_removes_formatting() {
        assert_eq!(
            strip_preview_md("**[10:00] USER:** hello world"),
            "USER: hello world"
        );
        assert_eq!(strip_preview_md("`code` *italic*"), "code italic");
        assert_eq!(strip_preview_md("plain text"), "plain text");
    }

    #[test]
    fn test_load_empty_sessions_root() {
        let tmp = tempfile::tempdir().unwrap();
        let entries = load_session_picker_entries(&tmp.path().to_path_buf(), None);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_picker_entry_from_index_entry() {
        let tmp = tempfile::tempdir().unwrap();
        let sr = tmp.path().to_path_buf();
        let sid = "s_10000_123456789";
        let session_dir = sr.join(sid);
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("session.md"), "# Hello\nWorld").unwrap();

        // Build index
        let index = build_index(&sr).unwrap();
        assert_eq!(index.sessions.len(), 1);

        let entry = build_picker_entry(&sr, &index.sessions[0], None);
        assert_eq!(entry.id, sid);
        assert!(entry.resumable);
    }
}
