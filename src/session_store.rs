//! @efficiency-role: infra-adapter
//!
//! SQLite Session Storage — structured storage for session metadata,
//! transcripts, and tool execution records.
//!
//! Complements file-based session storage with queryable structured data.

use crate::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

/// Session status in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionStatus {
    Active,
    Completed,
    Failed,
    Archived,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Active => write!(f, "active"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::Archived => write!(f, "archived"),
        }
    }
}

impl SessionStatus {
    fn from_str(s: &str) -> Self {
        match s {
            "active" => SessionStatus::Active,
            "completed" => SessionStatus::Completed,
            "failed" => SessionStatus::Failed,
            "archived" => SessionStatus::Archived,
            _ => SessionStatus::Active,
        }
    }
}

/// Session metadata record.
#[derive(Debug, Clone)]
pub(crate) struct SessionRecord {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub model: Option<String>,
    pub workspace: Option<String>,
    pub message_count: usize,
    pub tool_call_count: usize,
    pub root_path: Option<String>,
}

/// Tool execution record.
#[derive(Debug, Clone)]
pub(crate) struct ToolExecutionRecord {
    pub id: i64,
    pub session_id: String,
    pub tool_name: String,
    pub input_summary: String,
    pub output_summary: String,
    pub duration_ms: Option<i64>,
    pub success: bool,
    pub executed_at: DateTime<Utc>,
}

/// Message record for transcript storage.
#[derive(Debug, Clone)]
pub(crate) struct MessageRecord {
    pub id: i64,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub token_count: Option<i64>,
}

/// SQLite session store providing structured session storage.
pub(crate) struct SessionStore {
    conn: Connection,
    db_path: PathBuf,
}

impl SessionStore {
    /// Create or open a session store at the given path.
    pub(crate) fn open(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create db directory: {}", parent.display()))?;
        }

        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open SQLite database: {}", db_path.display()))?;

        let store = Self {
            conn,
            db_path: db_path.to_path_buf(),
        };

        store.run_migrations()?;
        Ok(store)
    }

    /// Run database migrations to ensure schema is up to date.
    fn run_migrations(&self) -> Result<()> {
        self.conn
            .execute_batch(
                "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'active',
                model TEXT,
                workspace TEXT,
                message_count INTEGER NOT NULL DEFAULT 0,
                tool_call_count INTEGER NOT NULL DEFAULT 0,
                root_path TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                token_count INTEGER,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS tool_executions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                tool_name TEXT NOT NULL,
                input_summary TEXT NOT NULL,
                output_summary TEXT NOT NULL,
                duration_ms INTEGER,
                success INTEGER NOT NULL DEFAULT 1,
                executed_at TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE TABLE IF NOT EXISTS session_tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                tag TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
            CREATE INDEX IF NOT EXISTS idx_tool_executions_session ON tool_executions(session_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
            CREATE INDEX IF NOT EXISTS idx_session_tags_session ON session_tags(session_id);
            CREATE INDEX IF NOT EXISTS idx_session_tags_tag ON session_tags(tag);
            ",
            )
            .with_context(|| "Failed to run database migrations")?;

        Ok(())
    }

    /// Create a new session record.
    pub(crate) fn create_session(
        &self,
        id: &str,
        model: Option<&str>,
        workspace: Option<&str>,
        root_path: Option<&str>,
    ) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO sessions (id, created_at, updated_at, status, model, workspace, root_path)
             VALUES (?1, ?2, ?2, 'active', ?3, ?4, ?5)",
            params![id, now, model, workspace, root_path],
        )
        .with_context(|| format!("Failed to create session: {}", id))?;

        Ok(())
    }

    /// Update session status.
    pub(crate) fn update_session_status(&self, id: &str, status: SessionStatus) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "UPDATE sessions SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.to_string(), now, id],
            )
            .with_context(|| format!("Failed to update session status: {}", id))?;

        Ok(())
    }

    /// Increment message count for a session.
    pub(crate) fn increment_message_count(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET message_count = message_count + 1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .with_context(|| format!("Failed to increment message count: {}", id))?;

        Ok(())
    }

    /// Increment tool call count for a session.
    pub(crate) fn increment_tool_call_count(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE sessions SET tool_call_count = tool_call_count + 1, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )
        .with_context(|| format!("Failed to increment tool call count: {}", id))?;

        Ok(())
    }

    /// Add a message to a session.
    pub(crate) fn add_message(
        &self,
        session_id: &str,
        role: &str,
        content: &str,
        token_count: Option<i64>,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO messages (session_id, role, content, timestamp, token_count)
             VALUES (?1, ?2, ?3, ?4, ?5)",
                params![session_id, role, content, now, token_count],
            )
            .with_context(|| format!("Failed to add message to session: {}", session_id))?;

        self.increment_message_count(session_id)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Record a tool execution.
    pub(crate) fn record_tool_execution(
        &self,
        session_id: &str,
        tool_name: &str,
        input_summary: &str,
        output_summary: &str,
        duration_ms: Option<i64>,
        success: bool,
    ) -> Result<i64> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO tool_executions (session_id, tool_name, input_summary, output_summary, duration_ms, success, executed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![session_id, tool_name, input_summary, output_summary, duration_ms, success as i32, now],
        )
        .with_context(|| format!("Failed to record tool execution: {}", session_id))?;

        self.increment_tool_call_count(session_id)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Add a tag to a session.
    pub(crate) fn add_tag(&self, session_id: &str, tag: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO session_tags (session_id, tag) VALUES (?1, ?2)",
                params![session_id, tag],
            )
            .with_context(|| format!("Failed to add tag to session: {}", session_id))?;

        Ok(())
    }

    /// Get a session by ID.
    pub(crate) fn get_session(&self, id: &str) -> Result<Option<SessionRecord>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, created_at, updated_at, status, model, workspace, message_count, tool_call_count, root_path FROM sessions WHERE id = ?1")
            .with_context(|| "Failed to prepare session query")?;

        let session = stmt
            .query_row(params![id], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    status: SessionStatus::from_str(&row.get::<_, String>(3)?),
                    model: row.get(4).ok(),
                    workspace: row.get(5).ok(),
                    message_count: row.get(6)?,
                    tool_call_count: row.get(7)?,
                    root_path: row.get(8).ok(),
                })
            })
            .optional()
            .with_context(|| format!("Failed to query session: {}", id))?;

        Ok(session)
    }

    /// List sessions with optional status filter and limit.
    pub(crate) fn list_sessions(
        &self,
        status: Option<SessionStatus>,
        limit: usize,
    ) -> Result<Vec<SessionRecord>> {
        let query = match status {
            Some(s) => format!(
                "SELECT id, created_at, updated_at, status, model, workspace, message_count, tool_call_count, root_path
                 FROM sessions WHERE status = '{}' ORDER BY updated_at DESC LIMIT {}",
                s.to_string(),
                limit
            ),
            None => format!(
                "SELECT id, created_at, updated_at, status, model, workspace, message_count, tool_call_count, root_path
                 FROM sessions ORDER BY updated_at DESC LIMIT {}",
                limit
            ),
        };

        let mut stmt = self
            .conn
            .prepare(&query)
            .with_context(|| "Failed to prepare session list query")?;

        let sessions = stmt
            .query_map(params![], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    status: SessionStatus::from_str(&row.get::<_, String>(3)?),
                    model: row.get(4).ok(),
                    workspace: row.get(5).ok(),
                    message_count: row.get(6)?,
                    tool_call_count: row.get(7)?,
                    root_path: row.get(8).ok(),
                })
            })
            .with_context(|| "Failed to query sessions")?
            .collect::<rusqlite::Result<Vec<_>>>()
            .with_context(|| "Failed to collect session records")?;

        Ok(sessions)
    }

    /// Get messages for a session.
    pub(crate) fn get_messages(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<MessageRecord>> {
        let query = format!(
            "SELECT id, session_id, role, content, timestamp, token_count
             FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC LIMIT {}",
            limit
        );

        let mut stmt = self
            .conn
            .prepare(&query)
            .with_context(|| "Failed to prepare message query")?;

        let messages = stmt
            .query_map(params![session_id], |row| {
                Ok(MessageRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    role: row.get(2)?,
                    content: row.get(3)?,
                    timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    token_count: row.get(5).ok(),
                })
            })
            .with_context(|| format!("Failed to query messages for session: {}", session_id))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .with_context(|| "Failed to collect message records")?;

        Ok(messages)
    }

    /// Get tool executions for a session.
    pub(crate) fn get_tool_executions(
        &self,
        session_id: &str,
        limit: usize,
    ) -> Result<Vec<ToolExecutionRecord>> {
        let query = format!(
            "SELECT id, session_id, tool_name, input_summary, output_summary, duration_ms, success, executed_at
             FROM tool_executions WHERE session_id = ?1 ORDER BY executed_at ASC LIMIT {}",
            limit
        );

        let mut stmt = self
            .conn
            .prepare(&query)
            .with_context(|| "Failed to prepare tool execution query")?;

        let executions = stmt
            .query_map(params![session_id], |row| {
                Ok(ToolExecutionRecord {
                    id: row.get(0)?,
                    session_id: row.get(1)?,
                    tool_name: row.get(2)?,
                    input_summary: row.get(3)?,
                    output_summary: row.get(4)?,
                    duration_ms: row.get(5).ok(),
                    success: row.get::<_, i32>(6)? != 0,
                    executed_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                })
            })
            .with_context(|| {
                format!(
                    "Failed to query tool executions for session: {}",
                    session_id
                )
            })?
            .collect::<rusqlite::Result<Vec<_>>>()
            .with_context(|| "Failed to collect tool execution records")?;

        Ok(executions)
    }

    /// Search sessions by tag.
    pub(crate) fn find_sessions_by_tag(&self, tag: &str) -> Result<Vec<SessionRecord>> {
        let query = "SELECT s.id, s.created_at, s.updated_at, s.status, s.model, s.workspace, s.message_count, s.tool_call_count, s.root_path
                     FROM sessions s
                     JOIN session_tags st ON s.id = st.session_id
                     WHERE st.tag = ?1
                     ORDER BY s.updated_at DESC";

        let mut stmt = self
            .conn
            .prepare(query)
            .with_context(|| "Failed to prepare tag search query")?;

        let sessions = stmt
            .query_map(params![tag], |row| {
                Ok(SessionRecord {
                    id: row.get(0)?,
                    created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(2)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_default(),
                    status: SessionStatus::from_str(&row.get::<_, String>(3)?),
                    model: row.get(4).ok(),
                    workspace: row.get(5).ok(),
                    message_count: row.get(6)?,
                    tool_call_count: row.get(7)?,
                    root_path: row.get(8).ok(),
                })
            })
            .with_context(|| format!("Failed to search sessions by tag: {}", tag))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .with_context(|| "Failed to collect session records")?;

        Ok(sessions)
    }

    /// Get session statistics.
    pub(crate) fn get_stats(&self) -> Result<SessionStats> {
        let total: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", params![], |r| r.get(0))
            .with_context(|| "Failed to count sessions")?;

        let active: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE status = 'active'",
                params![],
                |r| r.get(0),
            )
            .with_context(|| "Failed to count active sessions")?;

        let completed: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sessions WHERE status = 'completed'",
                params![],
                |r| r.get(0),
            )
            .with_context(|| "Failed to count completed sessions")?;

        let total_messages: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(message_count), 0) FROM sessions",
                params![],
                |r| r.get(0),
            )
            .with_context(|| "Failed to sum message counts")?;

        let total_tools: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(tool_call_count), 0) FROM sessions",
                params![],
                |r| r.get(0),
            )
            .with_context(|| "Failed to sum tool call counts")?;

        Ok(SessionStats {
            total: total as usize,
            active: active as usize,
            completed: completed as usize,
            total_messages: total_messages as usize,
            total_tool_calls: total_tools as usize,
        })
    }

    /// Delete a session and all associated data.
    pub(crate) fn delete_session(&self, id: &str) -> Result<usize> {
        // SQLite foreign keys will cascade if enabled, but we'll do it explicitly
        let mut deleted = 0;

        deleted += self
            .conn
            .execute(
                "DELETE FROM session_tags WHERE session_id = ?1",
                params![id],
            )
            .with_context(|| format!("Failed to delete tags for session: {}", id))?;

        deleted += self
            .conn
            .execute("DELETE FROM messages WHERE session_id = ?1", params![id])
            .with_context(|| format!("Failed to delete messages for session: {}", id))?;

        deleted += self
            .conn
            .execute(
                "DELETE FROM tool_executions WHERE session_id = ?1",
                params![id],
            )
            .with_context(|| format!("Failed to delete tool executions for session: {}", id))?;

        deleted += self
            .conn
            .execute("DELETE FROM sessions WHERE id = ?1", params![id])
            .with_context(|| format!("Failed to delete session: {}", id))?;

        Ok(deleted)
    }

    /// Get the database path.
    pub(crate) fn db_path(&self) -> &Path {
        &self.db_path
    }
}

/// Session statistics summary.
#[derive(Debug, Clone)]
pub(crate) struct SessionStats {
    pub total: usize,
    pub active: usize,
    pub completed: usize,
    pub total_messages: usize,
    pub total_tool_calls: usize,
}

impl std::fmt::Display for SessionStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sessions: {} total ({} active, {} completed), {} messages, {} tool calls",
            self.total, self.active, self.completed, self.total_messages, self.total_tool_calls
        )
    }
}

/// Get the default database path in the elma data directory.
pub(crate) fn default_db_path() -> Result<PathBuf> {
    let paths = crate::dirs::ElmaPaths::new().with_context(|| "Failed to get elma paths")?;
    Ok(paths.sessions_dir().join("sessions.db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_store() -> (SessionStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_sessions.db");
        let store = SessionStore::open(&db_path).unwrap();
        (store, temp_dir)
    }

    #[test]
    fn test_create_and_get_session() {
        let (store, _temp) = create_test_store();

        store
            .create_session(
                "test_001",
                Some("llama-3b"),
                Some("/workspace"),
                Some("/workspace/session_001"),
            )
            .unwrap();

        let session = store.get_session("test_001").unwrap().unwrap();
        assert_eq!(session.id, "test_001");
        assert_eq!(session.model, Some("llama-3b".to_string()));
        assert_eq!(session.status, SessionStatus::Active);
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_update_session_status() {
        let (store, _temp) = create_test_store();

        store.create_session("test_002", None, None, None).unwrap();
        store
            .update_session_status("test_002", SessionStatus::Completed)
            .unwrap();

        let session = store.get_session("test_002").unwrap().unwrap();
        assert_eq!(session.status, SessionStatus::Completed);
    }

    #[test]
    fn test_add_and_get_messages() {
        let (store, _temp) = create_test_store();

        store.create_session("test_003", None, None, None).unwrap();
        store
            .add_message("test_003", "user", "Hello", None)
            .unwrap();
        store
            .add_message("test_003", "assistant", "Hi there", Some(100))
            .unwrap();

        let messages = store.get_messages("test_003", 10).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].token_count, Some(100));

        // Verify message count incremented
        let session = store.get_session("test_003").unwrap().unwrap();
        assert_eq!(session.message_count, 2);
    }

    #[test]
    fn test_record_tool_execution() {
        let (store, _temp) = create_test_store();

        store.create_session("test_004", None, None, None).unwrap();
        store
            .record_tool_execution(
                "test_004",
                "shell",
                "ls -la",
                "total 42 files",
                Some(150),
                true,
            )
            .unwrap();

        let executions = store.get_tool_executions("test_004", 10).unwrap();
        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].tool_name, "shell");
        assert_eq!(executions[0].duration_ms, Some(150));
        assert!(executions[0].success);

        // Verify tool call count incremented
        let session = store.get_session("test_004").unwrap().unwrap();
        assert_eq!(session.tool_call_count, 1);
    }

    #[test]
    fn test_session_tags() {
        let (store, _temp) = create_test_store();

        store.create_session("test_005", None, None, None).unwrap();
        store.add_tag("test_005", "bugfix").unwrap();
        store.add_tag("test_005", "rust").unwrap();

        let sessions = store.find_sessions_by_tag("bugfix").unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, "test_005");
    }

    #[test]
    fn test_list_sessions() {
        let (store, _temp) = create_test_store();

        store.create_session("test_006", None, None, None).unwrap();
        store.create_session("test_007", None, None, None).unwrap();
        store
            .update_session_status("test_007", SessionStatus::Completed)
            .unwrap();

        let all = store.list_sessions(None, 10).unwrap();
        assert_eq!(all.len(), 2);

        let completed = store
            .list_sessions(Some(SessionStatus::Completed), 10)
            .unwrap();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, "test_007");
    }

    #[test]
    fn test_session_stats() {
        let (store, _temp) = create_test_store();

        store.create_session("test_008", None, None, None).unwrap();
        store.create_session("test_009", None, None, None).unwrap();
        store
            .update_session_status("test_009", SessionStatus::Completed)
            .unwrap();
        store.add_message("test_008", "user", "test", None).unwrap();

        let stats = store.get_stats().unwrap();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.active, 1);
        assert_eq!(stats.completed, 1);
        assert_eq!(stats.total_messages, 1);
    }

    #[test]
    fn test_delete_session() {
        let (store, _temp) = create_test_store();

        store.create_session("test_010", None, None, None).unwrap();
        store.add_message("test_010", "user", "test", None).unwrap();
        store.add_tag("test_010", "test").unwrap();

        let deleted = store.delete_session("test_010").unwrap();
        assert!(deleted > 0);

        let session = store.get_session("test_010").unwrap();
        assert!(session.is_none());
    }

    #[test]
    fn test_default_db_path() {
        let path = default_db_path();
        assert!(path.is_ok());
        assert!(path.unwrap().ends_with("sessions.db"));
    }
}
