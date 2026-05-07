//! @efficiency-role: storage-state
//!
//! Typed session store — persists session transcript parts as JSONL with
//! typed message variants. Supports append, load, search, and aggregation.

use crate::*;

/// A tool call invocation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolCallPart {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) input: String,
}

/// A tool execution result record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolResultPart {
    pub(crate) id: String,
    pub(crate) success: bool,
    pub(crate) output: String,
}

/// A single typed part within a session transcript, stored as JSONL.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub(crate) enum TypedMessagePart {
    #[serde(rename = "user_text")]
    UserText { content: String },
    #[serde(rename = "assistant_text")]
    AssistantText { content: String },
    #[serde(rename = "thinking_block")]
    ThinkingBlock { content: String },
    #[serde(rename = "tool_call")]
    ToolCall(ToolCallPart),
    #[serde(rename = "tool_result")]
    ToolResult(ToolResultPart),
    #[serde(rename = "system_message")]
    SystemMessage { content: String },
    #[serde(rename = "notice")]
    Notice { content: String },
}

/// Type key constants for count_by_type return values.
pub(crate) const TYPE_USER_TEXT: &str = "user_text";
pub(crate) const TYPE_ASSISTANT_TEXT: &str = "assistant_text";
pub(crate) const TYPE_THINKING_BLOCK: &str = "thinking_block";
pub(crate) const TYPE_TOOL_CALL: &str = "tool_call";
pub(crate) const TYPE_TOOL_RESULT: &str = "tool_result";
pub(crate) const TYPE_SYSTEM_MESSAGE: &str = "system_message";
pub(crate) const TYPE_NOTICE: &str = "notice";

impl TypedMessagePart {
    /// Return the type tag string for this part.
    pub(crate) fn type_tag(&self) -> &'static str {
        match self {
            TypedMessagePart::UserText { .. } => TYPE_USER_TEXT,
            TypedMessagePart::AssistantText { .. } => TYPE_ASSISTANT_TEXT,
            TypedMessagePart::ThinkingBlock { .. } => TYPE_THINKING_BLOCK,
            TypedMessagePart::ToolCall(_) => TYPE_TOOL_CALL,
            TypedMessagePart::ToolResult(_) => TYPE_TOOL_RESULT,
            TypedMessagePart::SystemMessage { .. } => TYPE_SYSTEM_MESSAGE,
            TypedMessagePart::Notice { .. } => TYPE_NOTICE,
        }
    }
}

/// JSONL-based typed session store for structured transcript persistence.
///
/// Stores parts as newline-delimited JSON (`session_parts.jsonl`) in the
/// session directory. Each line is a single TypedMessagePart.
pub(crate) struct TypedSessionStore;

impl TypedSessionStore {
    fn parts_path(session_root: &Path) -> PathBuf {
        session_root.join("session_parts.jsonl")
    }

    /// Append a single typed message part to the session JSONL file.
    pub(crate) fn append(session_root: &Path, part: TypedMessagePart) -> Result<()> {
        let path = Self::parts_path(session_root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create {}", parent.display()))?;
        }
        let json = serde_json::to_string(&part).with_context(|| "serialize message part")?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("open {}", path.display()))?;
        writeln!(file, "{}", json).with_context(|| format!("write to {}", path.display()))?;
        Ok(())
    }

    /// Load all typed message parts from a session JSONL file.
    pub(crate) fn load(session_root: &Path) -> Vec<TypedMessagePart> {
        let path = Self::parts_path(session_root);
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() {
                    return None;
                }
                serde_json::from_str(line).ok()
            })
            .collect()
    }

    /// Search loaded parts for a query string, returning matching indices.
    pub(crate) fn search(parts: &[TypedMessagePart], query: &str) -> Vec<usize> {
        let q = query.to_lowercase();
        parts
            .iter()
            .enumerate()
            .filter(|(_, part)| {
                let text: &str = match part {
                    TypedMessagePart::UserText { content } => content,
                    TypedMessagePart::AssistantText { content } => content,
                    TypedMessagePart::ThinkingBlock { content } => content,
                    TypedMessagePart::ToolCall(t) => &t.input,
                    TypedMessagePart::ToolResult(t) => &t.output,
                    TypedMessagePart::SystemMessage { content } => content,
                    TypedMessagePart::Notice { content } => content,
                };
                text.to_lowercase().contains(&q)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Count parts by their type tag.
    pub(crate) fn count_by_type(parts: &[TypedMessagePart]) -> HashMap<&'static str, usize> {
        let mut counts = HashMap::new();
        for part in parts {
            *counts.entry(part.type_tag()).or_insert(0) += 1;
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn ut(msg: &str) -> TypedMessagePart {
        TypedMessagePart::UserText {
            content: msg.to_string(),
        }
    }
    fn at(msg: &str) -> TypedMessagePart {
        TypedMessagePart::AssistantText {
            content: msg.to_string(),
        }
    }
    fn tb(msg: &str) -> TypedMessagePart {
        TypedMessagePart::ThinkingBlock {
            content: msg.to_string(),
        }
    }
    fn tc(id: &str, name: &str, input: &str) -> TypedMessagePart {
        TypedMessagePart::ToolCall(ToolCallPart {
            id: id.to_string(),
            name: name.to_string(),
            input: input.to_string(),
        })
    }
    fn tr(id: &str, success: bool, output: &str) -> TypedMessagePart {
        TypedMessagePart::ToolResult(ToolResultPart {
            id: id.to_string(),
            success,
            output: output.to_string(),
        })
    }
    fn sm(msg: &str) -> TypedMessagePart {
        TypedMessagePart::SystemMessage {
            content: msg.to_string(),
        }
    }
    fn nt(msg: &str) -> TypedMessagePart {
        TypedMessagePart::Notice {
            content: msg.to_string(),
        }
    }

    #[test]
    fn test_type_tag_values() {
        assert_eq!(ut("hi").type_tag(), TYPE_USER_TEXT);
        assert_eq!(at("ok").type_tag(), TYPE_ASSISTANT_TEXT);
        assert_eq!(tb("hmm").type_tag(), TYPE_THINKING_BLOCK);
        assert_eq!(tc("t1", "read", "{}").type_tag(), TYPE_TOOL_CALL);
        assert_eq!(tr("t1", true, "ok").type_tag(), TYPE_TOOL_RESULT);
        assert_eq!(sm("init").type_tag(), TYPE_SYSTEM_MESSAGE);
        assert_eq!(nt("notice").type_tag(), TYPE_NOTICE);
    }

    #[test]
    fn test_append_and_load() {
        let tmp = TempDir::new().unwrap();
        TypedSessionStore::append(tmp.path(), ut("hello")).unwrap();
        TypedSessionStore::append(tmp.path(), at("world")).unwrap();

        let parts = TypedSessionStore::load(tmp.path());
        assert_eq!(parts.len(), 2);
        assert!(matches!(parts[0], TypedMessagePart::UserText { .. }));
        assert!(matches!(parts[1], TypedMessagePart::AssistantText { .. }));
    }

    #[test]
    fn test_append_tool_call() {
        let tmp = TempDir::new().unwrap();
        TypedSessionStore::append(
            tmp.path(),
            TypedMessagePart::ToolCall(ToolCallPart {
                id: "call_1".to_string(),
                name: "bash".to_string(),
                input: "ls -la".to_string(),
            }),
        )
        .unwrap();
        TypedSessionStore::append(
            tmp.path(),
            TypedMessagePart::ToolResult(ToolResultPart {
                id: "call_1".to_string(),
                success: true,
                output: "total 42".to_string(),
            }),
        )
        .unwrap();

        let parts = TypedSessionStore::load(tmp.path());
        assert_eq!(parts.len(), 2);
        if let TypedMessagePart::ToolCall(ref tc) = parts[0] {
            assert_eq!(tc.name, "bash");
            assert_eq!(tc.id, "call_1");
        } else {
            panic!("expected tool_call");
        }
    }

    #[test]
    fn test_load_empty_session() {
        let tmp = TempDir::new().unwrap();
        let parts = TypedSessionStore::load(tmp.path());
        assert!(parts.is_empty());
    }

    #[test]
    fn test_search() {
        let parts = vec![
            ut("hello world"),
            at("goodbye moon"),
            tc("t1", "read", "world peace"),
        ];
        let results = TypedSessionStore::search(&parts, "world");
        assert_eq!(results, vec![0, 2]);
    }

    #[test]
    fn test_search_no_match() {
        let parts = vec![ut("hello")];
        let results = TypedSessionStore::search(&parts, "zzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_case_insensitive() {
        let parts = vec![at("Hello World")];
        let results = TypedSessionStore::search(&parts, "hello");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_count_by_type() {
        let parts = vec![
            ut("a"),
            ut("b"),
            at("c"),
            tc("t1", "x", "y"),
            tr("t1", true, "z"),
        ];
        let counts = TypedSessionStore::count_by_type(&parts);
        assert_eq!(counts.get(TYPE_USER_TEXT), Some(&2));
        assert_eq!(counts.get(TYPE_ASSISTANT_TEXT), Some(&1));
        assert_eq!(counts.get(TYPE_TOOL_CALL), Some(&1));
        assert_eq!(counts.get(TYPE_TOOL_RESULT), Some(&1));
        assert_eq!(counts.get(TYPE_THINKING_BLOCK), None);
    }

    #[test]
    fn test_multiple_append_creates_jsonl() {
        let tmp = TempDir::new().unwrap();
        TypedSessionStore::append(tmp.path(), sm("init")).unwrap();
        TypedSessionStore::append(tmp.path(), nt("starting")).unwrap();

        let content = std::fs::read_to_string(tmp.path().join("session_parts.jsonl")).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("system_message"));
        assert!(lines[1].contains("notice"));
    }

    #[test]
    fn test_jsonl_format_has_type_tag() {
        let tmp = TempDir::new().unwrap();
        TypedSessionStore::append(tmp.path(), ut("test")).unwrap();

        let raw = std::fs::read_to_string(tmp.path().join("session_parts.jsonl")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(raw.trim()).unwrap();
        assert_eq!(parsed["type"], "user_text");
        assert_eq!(parsed["content"], "test");
    }

    #[test]
    fn test_empty_search_returns_empty() {
        let results = TypedSessionStore::search(&[], "anything");
        assert!(results.is_empty());
    }

    #[test]
    fn test_append_creates_dir() {
        let tmp = TempDir::new().unwrap();
        let nested = tmp.path().join("subdir").join("nested");
        TypedSessionStore::append(&nested, nt("created")).unwrap();
        let parts = TypedSessionStore::load(&nested);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_append_and_search_tool_result() {
        let tmp = TempDir::new().unwrap();
        TypedSessionStore::append(
            tmp.path(),
            TypedMessagePart::ToolResult(ToolResultPart {
                id: "r1".to_string(),
                success: true,
                output: "build succeeded".to_string(),
            }),
        )
        .unwrap();

        let parts = TypedSessionStore::load(tmp.path());
        let results = TypedSessionStore::search(&parts, "succeeded");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_append_and_search_thinking_block() {
        let tmp = TempDir::new().unwrap();
        TypedSessionStore::append(tmp.path(), tb("let me analyze this")).unwrap();

        let parts = TypedSessionStore::load(tmp.path());
        let results = TypedSessionStore::search(&parts, "analyze");
        assert_eq!(results, vec![0]);
    }

    #[test]
    fn test_serde_roundtrip_user_text() {
        let original = ut("hello");
        let json = serde_json::to_string(&original).unwrap();
        let parsed: TypedMessagePart = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.type_tag(), TYPE_USER_TEXT);
        if let TypedMessagePart::UserText { content } = parsed {
            assert_eq!(content, "hello");
        } else {
            panic!("expected UserText");
        }
    }

    #[test]
    fn test_serde_roundtrip_tool_call() {
        let original = TypedMessagePart::ToolCall(ToolCallPart {
            id: "c1".into(),
            name: "bash".into(),
            input: "ls".into(),
        });
        let json = serde_json::to_string(&original).unwrap();
        let parsed: TypedMessagePart = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.type_tag(), TYPE_TOOL_CALL);
        if let TypedMessagePart::ToolCall(ref tc) = parsed {
            assert_eq!(tc.name, "bash");
        } else {
            panic!("expected ToolCall");
        }
    }
}
