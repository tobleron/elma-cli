//! @efficiency-role: data-model
//!
//! Tool Execution Event Ledger — records every tool call with raw payload references.
//!
//! Provides:
//! - Structured recording of tool execution events with full metadata
//! - Querying by event ID, turn ID, or tool name
//! - Persistence to JSONL files for offline analysis
//! - Raw payload storage and retrieval for input/output inspection

use crate::*;
use serde::{Deserialize, Serialize};

// ============================================================================
// ToolExecutionEvent
// ============================================================================

/// A recorded tool execution event with full metadata and optional payload link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolExecutionEvent {
    pub event_id: String,
    pub timestamp: u64,
    pub turn_id: String,
    pub tool_name: String,
    pub input_args: HashMap<String, String>,
    pub output_summary: String,
    pub success: bool,
    pub duration_ms: u64,
    pub raw_payload_path: Option<PathBuf>,
}

// ============================================================================
// EventLedger
// ============================================================================

/// In-memory ring-buffer ledger of tool execution events.
#[derive(Debug, Clone)]
pub(crate) struct EventLedger {
    events: Vec<ToolExecutionEvent>,
    max_events: usize,
    session_root: Option<PathBuf>,
}

impl EventLedger {
    /// Create a new empty ledger with default capacity (1000).
    pub(crate) fn new() -> Self {
        Self {
            events: Vec::new(),
            max_events: 1000,
            session_root: None,
        }
    }

    /// Create a new empty ledger with a specific capacity.
    pub(crate) fn with_capacity(max_events: usize) -> Self {
        Self {
            events: Vec::new(),
            max_events,
            session_root: None,
        }
    }

    /// Create a new empty ledger linked to a session root for persistence.
    pub(crate) fn with_session_root(session_root: PathBuf) -> Self {
        Self {
            events: Vec::new(),
            max_events: 1000,
            session_root: Some(session_root),
        }
    }

    /// Set or update the session root path.
    pub(crate) fn set_session_root(&mut self, session_root: PathBuf) {
        self.session_root = Some(session_root);
    }

    /// Record an event. Returns the event's ID.
    /// Drops the oldest event if the ledger is at capacity.
    pub(crate) fn record(&mut self, event: ToolExecutionEvent) -> String {
        let event_id = event.event_id.clone();
        if self.events.len() >= self.max_events {
            self.events.remove(0);
        }
        self.events.push(event);
        event_id
    }

    /// Retrieve a reference to an event by its ID.
    pub(crate) fn get_event(&self, id: &str) -> Option<&ToolExecutionEvent> {
        self.events.iter().find(|e| e.event_id == id)
    }

    /// Return all events matching a given turn ID.
    pub(crate) fn events_for_turn(&self, turn_id: &str) -> Vec<&ToolExecutionEvent> {
        self.events
            .iter()
            .filter(|e| e.turn_id == turn_id)
            .collect()
    }

    /// Return all events matching a given tool name.
    pub(crate) fn events_for_tool(&self, tool_name: &str) -> Vec<&ToolExecutionEvent> {
        self.events
            .iter()
            .filter(|e| e.tool_name == tool_name)
            .collect()
    }

    /// Persist all events to `session_root/events/events.jsonl`.
    /// Each line is a JSON-serialized `ToolExecutionEvent`.
    pub(crate) fn persist(&self) -> Result<()> {
        let session_root = self
            .session_root
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("EventLedger: session_root not set, cannot persist"))?;

        let events_dir = session_root.join("events");
        std::fs::create_dir_all(&events_dir).with_context(|| {
            format!(
                "Failed to create events directory: {}",
                events_dir.display()
            )
        })?;

        let path = events_dir.join("events.jsonl");
        let mut file = std::fs::File::create(&path)
            .with_context(|| format!("Failed to create events file: {}", path.display()))?;

        for event in &self.events {
            let line = serde_json::to_string(event)
                .with_context(|| "Failed to serialize tool execution event")?;
            writeln!(file, "{}", line)
                .with_context(|| format!("Failed to write event to {}", path.display()))?;
        }

        Ok(())
    }

    /// Return the current number of recorded events.
    pub(crate) fn count(&self) -> usize {
        self.events.len()
    }
}

// ============================================================================
// RawPayloadStore
// ============================================================================

/// On-disk storage for raw tool input/output payloads.
#[derive(Debug, Clone)]
pub(crate) struct RawPayloadStore {
    pub store_dir: PathBuf,
}

impl RawPayloadStore {
    /// Create a new store rooted at the given directory.
    pub(crate) fn new(store_dir: PathBuf) -> Self {
        Self { store_dir }
    }

    /// Write a raw payload (input + output) to disk.
    ///
    /// The file is named `{tool_name}_{unix_timestamp}.json` and stored
    /// inside `store_dir`. Returns the path to the written file.
    pub(crate) fn store_payload(
        &self,
        tool_name: &str,
        input: &str,
        output: &str,
    ) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.store_dir).with_context(|| {
            format!(
                "Failed to create payload store directory: {}",
                self.store_dir.display()
            )
        })?;

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let filename = format!("{}_{}.json", tool_name, timestamp);
        let path = self.store_dir.join(&filename);

        let payload = serde_json::json!({
            "tool_name": tool_name,
            "timestamp": timestamp,
            "input": input,
            "output": output,
        });

        let content = serde_json::to_string_pretty(&payload)
            .with_context(|| "Failed to serialize payload JSON")?;

        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write payload to {}", path.display()))?;

        Ok(path)
    }

    /// Load a raw payload from disk.
    ///
    /// Accepts any path (not necessarily inside `store_dir`).
    /// Returns `None` if the file does not exist or cannot be read.
    pub(crate) fn load_payload(path: &Path) -> Option<String> {
        std::fs::read_to_string(path).ok()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_event(
        event_id: &str,
        turn_id: &str,
        tool_name: &str,
        success: bool,
    ) -> ToolExecutionEvent {
        ToolExecutionEvent {
            event_id: event_id.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            turn_id: turn_id.to_string(),
            tool_name: tool_name.to_string(),
            input_args: HashMap::new(),
            output_summary: "test output".to_string(),
            success,
            duration_ms: 42,
            raw_payload_path: None,
        }
    }

    #[test]
    fn test_record_and_get_event() {
        let mut ledger = EventLedger::with_capacity(10);
        let event = make_event("evt_001", "turn_1", "bash", true);
        let id = ledger.record(event);
        assert_eq!(id, "evt_001");
        let retrieved = ledger.get_event("evt_001");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().tool_name, "bash");
    }

    #[test]
    fn test_get_event_nonexistent() {
        let ledger = EventLedger::new();
        assert!(ledger.get_event("evt_999").is_none());
    }

    #[test]
    fn test_events_for_turn() {
        let mut ledger = EventLedger::with_capacity(10);
        ledger.record(make_event("evt_001", "turn_1", "bash", true));
        ledger.record(make_event("evt_002", "turn_1", "read", true));
        ledger.record(make_event("evt_003", "turn_2", "glob", true));

        let turn1_events = ledger.events_for_turn("turn_1");
        assert_eq!(turn1_events.len(), 2);
        assert!(turn1_events.iter().all(|e| e.turn_id == "turn_1"));
    }

    #[test]
    fn test_events_for_tool() {
        let mut ledger = EventLedger::with_capacity(10);
        ledger.record(make_event("evt_001", "turn_1", "bash", true));
        ledger.record(make_event("evt_002", "turn_2", "bash", false));
        ledger.record(make_event("evt_003", "turn_1", "read", true));

        let bash_events = ledger.events_for_tool("bash");
        assert_eq!(bash_events.len(), 2);
        assert!(bash_events.iter().all(|e| e.tool_name == "bash"));
    }

    #[test]
    fn test_record_drops_oldest_at_capacity() {
        let mut ledger = EventLedger::with_capacity(2);
        ledger.record(make_event("evt_001", "turn_1", "bash", true));
        ledger.record(make_event("evt_002", "turn_1", "read", true));
        ledger.record(make_event("evt_003", "turn_1", "glob", true));

        assert_eq!(ledger.count(), 2);
        assert!(ledger.get_event("evt_001").is_none());
        assert!(ledger.get_event("evt_002").is_some());
        assert!(ledger.get_event("evt_003").is_some());
    }

    #[test]
    fn test_count() {
        let mut ledger = EventLedger::with_capacity(100);
        assert_eq!(ledger.count(), 0);
        ledger.record(make_event("evt_001", "turn_1", "bash", true));
        assert_eq!(ledger.count(), 1);
        ledger.record(make_event("evt_002", "turn_1", "read", true));
        assert_eq!(ledger.count(), 2);
    }

    #[test]
    fn test_persist_and_recover() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let session_root = dir.path().to_path_buf();

        let mut ledger = EventLedger::with_capacity(100);
        ledger.set_session_root(session_root.clone());
        ledger.record(make_event("evt_001", "turn_1", "bash", true));
        ledger.record(make_event("evt_002", "turn_1", "read", false));

        assert!(ledger.persist().is_ok());

        // Verify the JSONL file was written correctly
        let events_path = session_root.join("events").join("events.jsonl");
        assert!(events_path.exists());

        let content = std::fs::read_to_string(&events_path).expect("failed to read events.jsonl");
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 2);

        // Deserialize and verify
        let recovered: ToolExecutionEvent =
            serde_json::from_str(lines[0]).expect("failed to parse first event");
        assert_eq!(recovered.event_id, "evt_001");
        assert_eq!(recovered.tool_name, "bash");
        assert!(recovered.success);

        let recovered2: ToolExecutionEvent =
            serde_json::from_str(lines[1]).expect("failed to parse second event");
        assert_eq!(recovered2.event_id, "evt_002");
        assert_eq!(recovered2.tool_name, "read");
        assert!(!recovered2.success);
    }

    #[test]
    fn test_persist_fails_without_session_root() {
        let ledger = EventLedger::new();
        let result = ledger.persist();
        assert!(result.is_err());
    }

    #[test]
    fn test_raw_payload_store_roundtrip() {
        let dir = tempfile::tempdir().expect("failed to create temp dir");
        let store = RawPayloadStore::new(dir.path().to_path_buf());

        let path = store
            .store_payload("bash", "echo hello", "hello\n")
            .expect("store_payload failed");

        assert!(path.exists());

        let loaded = RawPayloadStore::load_payload(&path);
        assert!(loaded.is_some());
        let content = loaded.unwrap();
        assert!(content.contains("echo hello"));
        assert!(content.contains("hello"));
        assert!(content.contains("bash"));
    }

    #[test]
    fn test_raw_payload_load_nonexistent() {
        let path = PathBuf::from("/tmp/nonexistent_payload_should_not_exist.json");
        let loaded = RawPayloadStore::load_payload(&path);
        assert!(loaded.is_none());
    }
}
