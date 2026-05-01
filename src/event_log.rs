//! @efficiency-role: data-model
//!
//! Formal Action-Observation Event Log (Task 338)
//!
//! Typed runtime event stream for model actions, tool observations, policy
//! decisions, and session lifecycle events. Complements the evidence ledger
//! with an ordered, replayable event sequence.
//!
//! All events carry a session ID, turn number, monotonic sequence, and
//! timestamp so the stream is always reconstructible regardless of persistence
//! mode.

use crate::*;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use std::sync::{OnceLock, RwLock};

// ============================================================================
// Types
// ============================================================================

/// Monotonic event ID within a session.
pub(crate) type EventSeq = u64;

/// Categorised event kind — each variant carries its own payload.
#[derive(Debug, Clone)]
pub(crate) enum EventKind {
    Action(ActionEvent),
    Observation(ObservationEvent),
    Policy(PolicyEvent),
    Lifecycle(LifecycleEvent),
}

/// Emitted when a tool call / DSL action fires.
#[derive(Debug, Clone)]
pub(crate) struct ActionEvent {
    pub tool_name: String,
    pub input_summary: String,
}

/// Emitted when a tool result arrives.
#[derive(Debug, Clone)]
pub(crate) struct ObservationEvent {
    pub tool_name: String,
    pub success: bool,
    pub duration_ms: u64,
    pub output_size: usize,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub signal_killed: Option<i32>,
}

/// Permission / policy decision.
#[derive(Debug, Clone)]
pub(crate) struct PolicyEvent {
    pub decision: PolicyDecision,
    pub command: String,
    pub reason: String,
    pub risk_level: String,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PolicyDecision {
    Allowed,
    Denied,
    AutoApproved,
}

impl std::fmt::Display for PolicyDecision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PolicyDecision::Allowed => write!(f, "ALLOWED"),
            PolicyDecision::Denied => write!(f, "DENIED"),
            PolicyDecision::AutoApproved => write!(f, "AUTO_APPROVED"),
        }
    }
}

/// Session lifecycle events.
#[derive(Debug, Clone)]
pub(crate) struct LifecycleEvent {
    pub kind: LifecycleKind,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LifecycleKind {
    SessionStart,
    Compaction,
    Finalization,
    Stop,
    StrategyShift,
}

impl std::fmt::Display for LifecycleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LifecycleKind::SessionStart => write!(f, "SESSION_START"),
            LifecycleKind::Compaction => write!(f, "COMPACTION"),
            LifecycleKind::Finalization => write!(f, "FINALIZATION"),
            LifecycleKind::Stop => write!(f, "STOP"),
            LifecycleKind::StrategyShift => write!(f, "STRATEGY_SHIFT"),
        }
    }
}

/// A single event in the log.
#[derive(Debug, Clone)]
pub(crate) struct Event {
    pub seq: EventSeq,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub turn_number: u32,
    pub kind: EventKind,
}

// ============================================================================
// Global Holder (session-scoped)
// ============================================================================

static EVENT_LOG: OnceLock<RwLock<Option<EventLog>>> = OnceLock::new();

fn event_log() -> &'static RwLock<Option<EventLog>> {
    EVENT_LOG.get_or_init(|| RwLock::new(None))
}

/// Initialise the event log for a session.
pub(crate) fn init_event_log(session_id: &str) {
    if let Ok(mut lock) = event_log().write() {
        *lock = Some(EventLog::new(session_id));
        // Emit the session-start lifecycle event.
        if let Some(log) = lock.as_mut() {
            log.emit(EventKind::Lifecycle(LifecycleEvent {
                kind: LifecycleKind::SessionStart,
                detail: format!("session {} started", session_id),
            }));
        }
    }
}

/// Push an event into the log (best-effort; silently ignored if not initialised).
pub(crate) fn emit_event(kind: EventKind) {
    if let Ok(mut lock) = event_log().write() {
        if let Some(log) = lock.as_mut() {
            log.emit(kind);
        }
    }
}

/// Set the current turn number for subsequent events.
pub(crate) fn set_turn(turn: u32) {
    if let Ok(mut lock) = event_log().write() {
        if let Some(log) = lock.as_mut() {
            log.turn_number = turn;
        }
    }
}

/// Read-only access to current events (returns empty vec if not initialised).
pub(crate) fn read_events() -> Vec<Event> {
    if let Ok(lock) = event_log().read() {
        if let Some(log) = lock.as_ref() {
            return log.events.clone();
        }
    }
    vec![]
}

/// Filter events by kind category.
pub(crate) fn filter_events(kind: &str) -> Vec<Event> {
    read_events()
        .into_iter()
        .filter(|e| {
            let label = match &e.kind {
                EventKind::Action(_) => "action",
                EventKind::Observation(_) => "observation",
                EventKind::Policy(_) => "policy",
                EventKind::Lifecycle(_) => "lifecycle",
            };
            label == kind
        })
        .collect()
}

/// Clear the event log (called at session end or re-init).
pub(crate) fn clear_event_log() {
    if let Ok(mut lock) = event_log().write() {
        *lock = None;
    }
}

// ============================================================================
// EventLog
// ============================================================================

#[derive(Debug)]
pub(crate) struct EventLog {
    session_id: String,
    events: Vec<Event>,
    next_seq: EventSeq,
    turn_number: u32,
}

impl EventLog {
    pub(crate) fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            events: Vec::new(),
            next_seq: 1,
            turn_number: 0,
        }
    }

    /// Set the current turn number for subsequent events.
    pub(crate) fn set_turn(&mut self, turn: u32) {
        self.turn_number = turn;
    }

    /// Append an event with the next auto-incrementing sequence.
    pub(crate) fn emit(&mut self, kind: EventKind) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.events.push(Event {
            seq,
            timestamp: Utc::now(),
            session_id: self.session_id.clone(),
            turn_number: self.turn_number,
            kind,
        });
    }

    /// Return all events in sequence order.
    pub(crate) fn all_events(&self) -> &[Event] {
        &self.events
    }

    /// Number of events recorded.
    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    /// Render events as a compact text summary for debugging / tests.
    pub(crate) fn render_text(&self) -> String {
        self.events
            .iter()
            .map(|e| {
                let label = match &e.kind {
                    EventKind::Action(a) => format!("ACTION {}", a.tool_name),
                    EventKind::Observation(o) => {
                        format!("OBSERV {} ok={}", o.tool_name, o.success)
                    }
                    EventKind::Policy(p) => format!("POLICY {} {}", p.decision, p.risk_level),
                    EventKind::Lifecycle(l) => format!("LIFECYCLE {}", l.kind),
                };
                format!("  #{} turn={} {}", e.seq, e.turn_number, label)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Find the latest event of a given category.
    pub(crate) fn last_event_of_kind(&self, kind: &str) -> Option<&Event> {
        self.events.iter().rev().find(|e| {
            let label = match &e.kind {
                EventKind::Action(_) => "action",
                EventKind::Observation(_) => "observation",
                EventKind::Policy(_) => "policy",
                EventKind::Lifecycle(_) => "lifecycle",
            };
            label == kind
        })
    }
}

// ============================================================================
// Persistence — SQLite
// ============================================================================

/// Persist all events for the current session to the SQLite store.
pub(crate) fn persist_events(db_path: &std::path::Path) -> Result<()> {
    let events = read_events();
    if events.is_empty() {
        return Ok(());
    }

    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open event log database: {}", db_path.display()))?;

    // Ensure the event_log table exists (safe if already created by session_store)
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS event_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            timestamp TEXT NOT NULL,
            turn_number INTEGER NOT NULL DEFAULT 0,
            kind TEXT NOT NULL,
            tool_name TEXT,
            input_summary TEXT,
            success INTEGER,
            duration_ms INTEGER,
            output_size INTEGER,
            exit_code INTEGER,
            timed_out INTEGER,
            signal_killed INTEGER,
            decision TEXT,
            command TEXT,
            reason TEXT,
            risk_level TEXT,
            context TEXT,
            lifecycle_kind TEXT,
            detail TEXT,
            schema_version INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_event_log_session ON event_log(session_id);
        CREATE INDEX IF NOT EXISTS idx_event_log_seq ON event_log(session_id, seq);",
    )
    .with_context(|| "Failed to ensure event_log table exists")?;

    for event in &events {
        let kind_str = match &event.kind {
            EventKind::Action(_) => "action",
            EventKind::Observation(_) => "observation",
            EventKind::Policy(_) => "policy",
            EventKind::Lifecycle(_) => "lifecycle",
        };

        match &event.kind {
            EventKind::Action(a) => {
                conn.execute(
                    "INSERT INTO event_log \
                     (session_id, seq, timestamp, turn_number, kind, \
                      tool_name, input_summary, schema_version) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        event.session_id,
                        event.seq as i64,
                        event.timestamp.to_rfc3339(),
                        event.turn_number as i32,
                        kind_str,
                        &a.tool_name,
                        &a.input_summary,
                        1_i32,
                    ],
                )
                .with_context(|| "Failed to insert action event")?;
            }
            EventKind::Observation(o) => {
                conn.execute(
                    "INSERT INTO event_log \
                     (session_id, seq, timestamp, turn_number, kind, \
                      tool_name, success, duration_ms, output_size, exit_code, \
                      timed_out, signal_killed, schema_version) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    params![
                        event.session_id,
                        event.seq as i64,
                        event.timestamp.to_rfc3339(),
                        event.turn_number as i32,
                        kind_str,
                        &o.tool_name,
                        o.success as i32,
                        o.duration_ms as i64,
                        o.output_size as i64,
                        o.exit_code,
                        o.timed_out as i32,
                        o.signal_killed,
                        1_i32,
                    ],
                )
                .with_context(|| "Failed to insert observation event")?;
            }
            EventKind::Policy(p) => {
                conn.execute(
                    "INSERT INTO event_log \
                     (session_id, seq, timestamp, turn_number, kind, \
                      decision, command, reason, risk_level, context, schema_version) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                    params![
                        event.session_id,
                        event.seq as i64,
                        event.timestamp.to_rfc3339(),
                        event.turn_number as i32,
                        kind_str,
                        p.decision.to_string(),
                        &p.command,
                        &p.reason,
                        &p.risk_level,
                        &p.context,
                        1_i32,
                    ],
                )
                .with_context(|| "Failed to insert policy event")?;
            }
            EventKind::Lifecycle(l) => {
                conn.execute(
                    "INSERT INTO event_log \
                     (session_id, seq, timestamp, turn_number, kind, \
                      lifecycle_kind, detail, schema_version) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        event.session_id,
                        event.seq as i64,
                        event.timestamp.to_rfc3339(),
                        event.turn_number as i32,
                        kind_str,
                        l.kind.to_string(),
                        &l.detail,
                        1_i32,
                    ],
                )
                .with_context(|| "Failed to insert lifecycle event")?;
            }
        }
    }

    Ok(())
}

/// Replay events from the SQLite store for a given session.
pub(crate) fn replay_events(db_path: &std::path::Path, session_id: &str) -> Result<Vec<Event>> {
    let conn = Connection::open(db_path)
        .with_context(|| format!("Failed to open event log database: {}", db_path.display()))?;

    // Ensure the event_log table exists
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS event_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            seq INTEGER NOT NULL,
            timestamp TEXT NOT NULL,
            turn_number INTEGER NOT NULL DEFAULT 0,
            kind TEXT NOT NULL,
            tool_name TEXT,
            input_summary TEXT,
            success INTEGER,
            duration_ms INTEGER,
            output_size INTEGER,
            exit_code INTEGER,
            timed_out INTEGER,
            signal_killed INTEGER,
            decision TEXT,
            command TEXT,
            reason TEXT,
            risk_level TEXT,
            context TEXT,
            lifecycle_kind TEXT,
            detail TEXT,
            schema_version INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_event_log_session ON event_log(session_id);
        CREATE INDEX IF NOT EXISTS idx_event_log_seq ON event_log(session_id, seq);",
    )
    .with_context(|| "Failed to ensure event_log table exists")?;

    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, seq, timestamp, turn_number, kind, \
                    tool_name, input_summary, success, duration_ms, output_size, exit_code, \
                    timed_out, signal_killed, decision, command, reason, risk_level, context, \
                    lifecycle_kind, detail \
             FROM event_log \
             WHERE session_id = ?1 \
             ORDER BY seq",
        )
        .with_context(|| "Failed to prepare replay query")?;

    let events = stmt
        .query_map(params![session_id], |row| {
            let kind: String = row.get(5)?;
            let session_id: String = row.get(1)?;
            let seq: i64 = row.get(2)?;
            let timestamp_str: String = row.get(3)?;
            let turn_number: i32 = row.get(4)?;

            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_default();

            let event_kind = match kind.as_str() {
                "action" => EventKind::Action(ActionEvent {
                    tool_name: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    input_summary: row.get::<_, Option<String>>(7)?.unwrap_or_default(),
                }),
                "observation" => EventKind::Observation(ObservationEvent {
                    tool_name: row.get::<_, Option<String>>(6)?.unwrap_or_default(),
                    success: row.get::<_, Option<i32>>(8)?.unwrap_or(0) != 0,
                    duration_ms: row.get::<_, Option<i64>>(9)?.unwrap_or(0) as u64,
                    output_size: row.get::<_, Option<i64>>(10)?.unwrap_or(0) as usize,
                    exit_code: row.get(11)?,
                    timed_out: row.get::<_, Option<i32>>(12)?.unwrap_or(0) != 0,
                    signal_killed: row.get(13)?,
                }),
                "policy" => EventKind::Policy(PolicyEvent {
                    decision: parse_policy_decision(
                        &row.get::<_, Option<String>>(14)?.unwrap_or_default(),
                    ),
                    command: row.get::<_, Option<String>>(15)?.unwrap_or_default(),
                    reason: row.get::<_, Option<String>>(16)?.unwrap_or_default(),
                    risk_level: row.get::<_, Option<String>>(17)?.unwrap_or_default(),
                    context: row.get::<_, Option<String>>(18)?.unwrap_or_default(),
                }),
                "lifecycle" => EventKind::Lifecycle(LifecycleEvent {
                    kind: parse_lifecycle_kind(
                        &row.get::<_, Option<String>>(19)?.unwrap_or_default(),
                    ),
                    detail: row.get::<_, Option<String>>(20)?.unwrap_or_default(),
                }),
                _ => {
                    return Err(rusqlite::Error::InvalidParameterName(format!(
                        "unknown event kind: {}",
                        kind
                    )))
                }
            };

            Ok(Event {
                seq: seq as EventSeq,
                timestamp,
                session_id,
                turn_number: turn_number as u32,
                kind: event_kind,
            })
        })
        .with_context(|| format!("Failed to query events for session: {}", session_id))?;

    let events: Vec<Event> = events
        .collect::<rusqlite::Result<Vec<_>>>()
        .with_context(|| {
            format!(
                "Failed to collect replayed events for session: {}",
                session_id
            )
        })?;

    Ok(events)
}

/// Parse a policy decision string from the database.
fn parse_policy_decision(s: &str) -> PolicyDecision {
    match s {
        "ALLOWED" => PolicyDecision::Allowed,
        "DENIED" => PolicyDecision::Denied,
        "AUTO_APPROVED" => PolicyDecision::AutoApproved,
        _ => PolicyDecision::Denied,
    }
}

/// Parse a lifecycle kind string from the database.
fn parse_lifecycle_kind(s: &str) -> LifecycleKind {
    match s {
        "SESSION_START" => LifecycleKind::SessionStart,
        "COMPACTION" => LifecycleKind::Compaction,
        "FINALIZATION" => LifecycleKind::Finalization,
        "STOP" => LifecycleKind::Stop,
        "STRATEGY_SHIFT" => LifecycleKind::StrategyShift,
        _ => LifecycleKind::SessionStart,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_db_path(temp: &TempDir) -> std::path::PathBuf {
        temp.path().join("test_events.db")
    }

    fn init_test_session(session_id: &str) {
        clear_event_log();
        init_event_log(session_id);
    }

    fn with_log(f: impl FnOnce(&mut EventLog)) {
        let mut log = EventLog::new("test-session");
        f(&mut log);
    }

    #[test]
    fn test_empty_log() {
        with_log(|log| {
            assert_eq!(log.len(), 0);
            assert!(log.all_events().is_empty());
        });
    }

    #[test]
    fn test_emit_action_event() {
        with_log(|log| {
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "read".into(),
                input_summary: "src/main.rs".into(),
            }));
            assert_eq!(log.len(), 1);
            let e = &log.all_events()[0];
            assert_eq!(e.seq, 1);
            assert_eq!(e.turn_number, 0);
            match &e.kind {
                EventKind::Action(a) => {
                    assert_eq!(a.tool_name, "read");
                }
                _ => panic!("expected Action event"),
            }
        });
    }

    #[test]
    fn test_emit_observation_event() {
        with_log(|log| {
            log.emit(EventKind::Observation(ObservationEvent {
                tool_name: "shell".into(),
                success: true,
                duration_ms: 42,
                output_size: 128,
                exit_code: Some(0),
                timed_out: false,
                signal_killed: None,
            }));
            let e = &log.all_events()[0];
            match &e.kind {
                EventKind::Observation(o) => {
                    assert!(o.success);
                    assert_eq!(o.duration_ms, 42);
                    assert_eq!(o.exit_code, Some(0));
                }
                _ => panic!("expected Observation event"),
            }
        });
    }

    #[test]
    fn test_emit_policy_event() {
        with_log(|log| {
            log.emit(EventKind::Policy(PolicyEvent {
                decision: PolicyDecision::Denied,
                command: "rm -rf /".into(),
                reason: "destructive command".into(),
                risk_level: "dangerous".into(),
                context: "shell preflight".into(),
            }));
            let e = &log.all_events()[0];
            match &e.kind {
                EventKind::Policy(p) => {
                    assert_eq!(p.decision, PolicyDecision::Denied);
                    assert_eq!(p.risk_level, "dangerous");
                }
                _ => panic!("expected Policy event"),
            }
        });
    }

    #[test]
    fn test_emit_lifecycle_event() {
        with_log(|log| {
            log.emit(EventKind::Lifecycle(LifecycleEvent {
                kind: LifecycleKind::Compaction,
                detail: "removed 8 messages".into(),
            }));
            let e = &log.all_events()[0];
            match &e.kind {
                EventKind::Lifecycle(l) => {
                    assert_eq!(l.kind, LifecycleKind::Compaction);
                    assert_eq!(l.detail, "removed 8 messages");
                }
                _ => panic!("expected Lifecycle event"),
            }
        });
    }

    #[test]
    fn test_sequence_increments() {
        with_log(|log| {
            log.emit(EventKind::Lifecycle(LifecycleEvent {
                kind: LifecycleKind::SessionStart,
                detail: "start".into(),
            }));
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "read".into(),
                input_summary: "f".into(),
            }));
            log.emit(EventKind::Observation(ObservationEvent {
                tool_name: "read".into(),
                success: true,
                duration_ms: 5,
                output_size: 50,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }));
            assert_eq!(log.len(), 3);
            assert_eq!(log.all_events()[0].seq, 1);
            assert_eq!(log.all_events()[1].seq, 2);
            assert_eq!(log.all_events()[2].seq, 3);
        });
    }

    #[test]
    fn test_set_turn() {
        with_log(|log| {
            log.set_turn(3);
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "search".into(),
                input_summary: "foo".into(),
            }));
            assert_eq!(log.all_events()[0].turn_number, 3);
        });
    }

    #[test]
    fn test_filter_events() {
        with_log(|log| {
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "a".into(),
                input_summary: "x".into(),
            }));
            log.emit(EventKind::Observation(ObservationEvent {
                tool_name: "a".into(),
                success: true,
                duration_ms: 0,
                output_size: 0,
                exit_code: None,
                timed_out: false,
                signal_killed: None,
            }));
            log.emit(EventKind::Lifecycle(LifecycleEvent {
                kind: LifecycleKind::Stop,
                detail: "done".into(),
            }));
            // Simulate what filter_events would do (it uses read_events global)
            let action_count = log
                .all_events()
                .iter()
                .filter(|e| matches!(e.kind, EventKind::Action(_)))
                .count();
            assert_eq!(action_count, 1);
        });
    }

    #[test]
    fn test_render_text() {
        with_log(|log| {
            log.emit(EventKind::Lifecycle(LifecycleEvent {
                kind: LifecycleKind::SessionStart,
                detail: "start".into(),
            }));
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "read".into(),
                input_summary: "f".into(),
            }));
            let text = log.render_text();
            assert!(text.contains("#1"));
            assert!(text.contains("LIFECYCLE SESSION_START"));
            assert!(text.contains("#2"));
            assert!(text.contains("ACTION read"));
        });
    }

    #[test]
    fn test_last_event_of_kind() {
        with_log(|log| {
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "a1".into(),
                input_summary: "x".into(),
            }));
            log.emit(EventKind::Action(ActionEvent {
                tool_name: "a2".into(),
                input_summary: "y".into(),
            }));
            let last = log.last_event_of_kind("action").unwrap();
            match &last.kind {
                EventKind::Action(a) => assert_eq!(a.tool_name, "a2"),
                _ => panic!(),
            }
        });
    }

    #[test]
    fn test_global_init_and_emit() {
        clear_event_log();
        init_event_log("global-test");
        emit_event(EventKind::Action(ActionEvent {
            tool_name: "search".into(),
            input_summary: "query".into(),
        }));
        let events = read_events();
        assert_eq!(events.len(), 2); // SessionStart + Action
        assert!(matches!(events[0].kind, EventKind::Lifecycle(_)));
        assert!(matches!(events[1].kind, EventKind::Action(_)));
        clear_event_log();
    }

    #[test]
    fn test_persist_and_replay_empty() {
        let temp = TempDir::new().unwrap();
        clear_event_log();
        let result = persist_events(&test_db_path(&temp));
        assert!(result.is_ok());
        let replayed = replay_events(&test_db_path(&temp), "nonexistent").unwrap();
        assert!(replayed.is_empty());
    }

    #[test]
    fn test_persist_and_replay_all_kinds() {
        let temp = TempDir::new().unwrap();
        let session_id = "persist-test-session";
        init_test_session(session_id);

        // Emit one of each event kind
        emit_event(EventKind::Action(ActionEvent {
            tool_name: "test_tool".into(),
            input_summary: "test input".into(),
        }));
        emit_event(EventKind::Observation(ObservationEvent {
            tool_name: "test_tool".into(),
            success: true,
            duration_ms: 100,
            output_size: 512,
            exit_code: Some(0),
            timed_out: false,
            signal_killed: None,
        }));
        emit_event(EventKind::Policy(PolicyEvent {
            decision: PolicyDecision::Allowed,
            command: "ls".into(),
            reason: "safe command".into(),
            risk_level: "low".into(),
            context: "shell preflight".into(),
        }));
        emit_event(EventKind::Lifecycle(LifecycleEvent {
            kind: LifecycleKind::Compaction,
            detail: "freed 1000 tokens".into(),
        }));

        let result = persist_events(&test_db_path(&temp));
        assert!(result.is_ok(), "persist_events failed: {:?}", result);

        let replayed = replay_events(&test_db_path(&temp), session_id).unwrap();
        // SessionStart (1) + Action + Observation + Policy + Lifecycle = 5
        assert_eq!(
            replayed.len(),
            5,
            "expected 5 events, got {}",
            replayed.len()
        );

        // Verify event kinds in order
        assert!(matches!(replayed[0].kind, EventKind::Lifecycle(_)));
        assert!(matches!(replayed[1].kind, EventKind::Action(_)));
        assert!(matches!(replayed[2].kind, EventKind::Observation(_)));
        assert!(matches!(replayed[3].kind, EventKind::Policy(_)));
        assert!(matches!(replayed[4].kind, EventKind::Lifecycle(_)));

        // Verify field values survived round-trip
        match &replayed[1].kind {
            EventKind::Action(a) => {
                assert_eq!(a.tool_name, "test_tool");
                assert_eq!(a.input_summary, "test input");
            }
            _ => panic!("expected Action at index 1"),
        }
        match &replayed[2].kind {
            EventKind::Observation(o) => {
                assert!(o.success);
                assert_eq!(o.duration_ms, 100);
                assert_eq!(o.exit_code, Some(0));
            }
            _ => panic!("expected Observation at index 2"),
        }
        match &replayed[3].kind {
            EventKind::Policy(p) => {
                assert_eq!(p.decision, PolicyDecision::Allowed);
                assert_eq!(p.command, "ls");
            }
            _ => panic!("expected Policy at index 3"),
        }
        match &replayed[4].kind {
            EventKind::Lifecycle(l) => {
                assert_eq!(l.kind, LifecycleKind::Compaction);
                assert_eq!(l.detail, "freed 1000 tokens");
            }
            _ => panic!("expected Lifecycle at index 4"),
        }

        clear_event_log();
    }

    #[test]
    fn test_persist_replay_isolated_sessions() {
        let temp = TempDir::new().unwrap();
        let session_a = "session-a";
        let session_b = "session-b";

        // Persist events for session A
        init_test_session(session_a);
        emit_event(EventKind::Action(ActionEvent {
            tool_name: "read".into(),
            input_summary: "a.rs".into(),
        }));
        persist_events(&test_db_path(&temp)).unwrap();
        clear_event_log();

        // Persist events for session B
        init_test_session(session_b);
        emit_event(EventKind::Action(ActionEvent {
            tool_name: "search".into(),
            input_summary: "b.rs".into(),
        }));
        emit_event(EventKind::Observation(ObservationEvent {
            tool_name: "search".into(),
            success: true,
            duration_ms: 50,
            output_size: 200,
            exit_code: Some(0),
            timed_out: false,
            signal_killed: None,
        }));
        persist_events(&test_db_path(&temp)).unwrap();
        clear_event_log();

        // Replay sessions independently
        let replay_a = replay_events(&test_db_path(&temp), session_a).unwrap();
        let replay_b = replay_events(&test_db_path(&temp), session_b).unwrap();

        // Session A: SessionStart + Action = 2
        assert_eq!(replay_a.len(), 2);
        match &replay_a[1].kind {
            EventKind::Action(a) => assert_eq!(a.tool_name, "read"),
            _ => panic!("expected Action"),
        }

        // Session B: SessionStart + Action + Observation = 3
        assert_eq!(replay_b.len(), 3);
        match &replay_b[1].kind {
            EventKind::Action(a) => assert_eq!(a.tool_name, "search"),
            _ => panic!("expected Action"),
        }
    }
}
