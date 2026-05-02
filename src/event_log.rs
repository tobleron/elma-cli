//! @efficiency-role: data-model
//!
//! Action-Observation Event Log — canonical runtime timeline for each session turn.
//!
//! Events are compact, append-only, and replayable. Large payloads are stored as artifact references.

use crate::*;
use serde::{Deserialize, Serialize};
use std::sync::{OnceLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

static SESSION_EVENT_LOG: OnceLock<RwLock<Option<EventLog>>> = OnceLock::new();
static CURRENT_TURN_ID: OnceLock<RwLock<Option<String>>> = OnceLock::new();

fn session_event_log() -> &'static RwLock<Option<EventLog>> {
    SESSION_EVENT_LOG.get_or_init(|| RwLock::new(None))
}

fn current_turn_id() -> &'static RwLock<Option<String>> {
    CURRENT_TURN_ID.get_or_init(|| RwLock::new(None))
}

pub(crate) fn set_current_turn(turn_id: &str) {
    if let Ok(mut lock) = current_turn_id().write() {
        *lock = Some(turn_id.to_string());
    }
}

pub(crate) fn clear_current_turn() {
    if let Ok(mut lock) = current_turn_id().write() {
        *lock = None;
    }
}

pub(crate) fn get_current_turn_id() -> Option<String> {
    current_turn_id().read().ok().and_then(|lock| lock.clone())
}

pub(crate) fn record_lifecycle(event_type: LifecycleEventType, turn_id: Option<&str>) {
    let _ = with_session_event_log(|log| {
        log.record_lifecycle(event_type, turn_id);
    });
}

pub(crate) fn record_model_event(event_type: ModelEventType, turn_id: &str, tool_call_id: Option<&str>, model_request_id: Option<&str>) {
    let _ = with_session_event_log(|log| {
        log.record_model_event(event_type, turn_id, tool_call_id, model_request_id);
    });
}

pub(crate) fn record_tool_event(event_type: ToolEventType, turn_id: &str, tool_call_id: &str, tool_name: &str) {
    let _ = with_session_event_log(|log| {
        log.record_tool_event(event_type, turn_id, tool_call_id, tool_name);
    });
}

pub(crate) fn record_policy_event(event_type: PolicyEventType, turn_id: &str, tool_call_id: Option<&str>, reason: &str) {
    let _ = with_session_event_log(|log| {
        log.record_policy_event(event_type, turn_id, tool_call_id, reason);
    });
}

pub(crate) fn record_evidence_event(turn_id: &str, claim_text: &str, source_artifact: &str) {
    let _ = with_session_event_log(|log| {
        log.record_evidence_event(turn_id, claim_text, source_artifact);
    });
}

pub(crate) fn record_finalization(event_type: FinalizationEventType, turn_id: &str, stop_reason: &str) {
    let _ = with_session_event_log(|log| {
        log.record_finalization(event_type, turn_id, stop_reason);
    });
}

pub(crate) fn persist(session_root: &Path) -> Result<()> {
    if let Ok(lock) = session_event_log().read() {
        if let Some(log) = lock.as_ref() {
            return log.persist(session_root);
        }
    }
    Ok(())
}

pub(crate) fn init_session_event_log(session_id: &str) {
    if let Ok(mut lock) = session_event_log().write() {
        *lock = Some(EventLog::new(session_id));
    }
}

pub(crate) fn get_session_event_log() -> Option<EventLog> {
    session_event_log().read().ok().and_then(|lock| lock.clone())
}

pub(crate) fn with_session_event_log<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut EventLog) -> R,
{
    if let Ok(mut lock) = session_event_log().write() {
        if let Some(log) = lock.as_mut() {
            return Some(f(log));
        }
    }
    None
}

pub(crate) fn clear_session_event_log() {
    if let Ok(mut lock) = session_event_log().write() {
        *lock = None;
    }
}

/// Unique event identifier
pub(crate) type EventId = u64;

/// Session identifier
pub(crate) type SessionId = String;

/// Turn identifier
pub(crate) type TurnId = String;

/// Action-observation event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum AgentEvent {
    Lifecycle(LifecycleEvent),
    Model(ModelEvent),
    Tool(ToolEvent),
    Policy(PolicyEvent),
    Evidence(EvidenceEvent),
    Transcript(TranscriptEvent),
    Finalization(FinalizationEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LifecycleEvent {
    pub event_type: LifecycleEventType,
    pub session_id: SessionId,
    pub turn_id: Option<TurnId>,
    pub timestamp_unix: u64,
    pub sequence: EventId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LifecycleEventType {
    SessionStarted,
    TurnStarted,
    SessionEnded,
    TurnFinished,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModelEvent {
    pub event_type: ModelEventType,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub timestamp_unix: u64,
    pub sequence: EventId,
    pub tool_call_id: Option<String>,
    pub model_request_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ModelEventType {
    ModelRequestStarted,
    ModelToolCallProposed,
    ModelResponseReceived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolEvent {
    pub event_type: ToolEventType,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub timestamp_unix: u64,
    pub sequence: EventId,
    pub tool_call_id: String,
    pub tool_name: String,
    pub input_artifact: Option<String>,
    pub output_artifact: Option<String>,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ToolEventType {
    ToolStarted,
    ToolFinished,
    ToolFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PolicyEvent {
    pub event_type: PolicyEventType,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub timestamp_unix: u64,
    pub sequence: EventId,
    pub tool_call_id: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PolicyEventType {
    PermissionRequested,
    PermissionGranted,
    PermissionDenied,
    PolicyBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EvidenceEvent {
    pub event_type: EvidenceEventType,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub timestamp_unix: u64,
    pub sequence: EventId,
    pub claim_text: String,
    pub source_artifact: String,
    pub support_type: EvidenceSupportType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceEventType {
    EvidenceRecorded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EvidenceSupportType {
    DirectMatch,
    Structural,
    Inference,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TranscriptEvent {
    pub event_type: TranscriptEventType,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub timestamp_unix: u64,
    pub sequence: EventId,
    pub row_type: TranscriptRowType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TranscriptEventType {
    TranscriptRowAppended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TranscriptRowType {
    UserPrompt,
    AssistantResponse,
    ToolCall,
    ToolResult,
    Operational,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FinalizationEvent {
    pub event_type: FinalizationEventType,
    pub session_id: SessionId,
    pub turn_id: TurnId,
    pub timestamp_unix: u64,
    pub sequence: EventId,
    pub stop_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FinalizationEventType {
    StopPolicyTriggered,
    FinalAnswerPrepared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EventLog {
    session_id: SessionId,
    events: Vec<AgentEvent>,
    sequence_counter: EventId,
}

impl EventLog {
    pub(crate) fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_string(),
            events: Vec::new(),
            sequence_counter: 0,
        }
    }

    pub(crate) fn record(&mut self, event: AgentEvent) -> EventId {
        let seq = self.sequence_counter;
        self.sequence_counter += 1;
        self.events.push(event);
        seq
    }

    pub(crate) fn record_lifecycle(&mut self, event_type: LifecycleEventType, turn_id: Option<&str>) -> EventId {
        self.record(AgentEvent::Lifecycle(LifecycleEvent {
            event_type,
            session_id: self.session_id.clone(),
            turn_id: turn_id.map(|t| t.to_string()),
            timestamp_unix: current_unix(),
            sequence: self.sequence_counter,
        }))
    }

    pub(crate) fn record_model_event(&mut self, event_type: ModelEventType, turn_id: &str, tool_call_id: Option<&str>, model_request_id: Option<&str>) -> EventId {
        self.record(AgentEvent::Model(ModelEvent {
            event_type,
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            timestamp_unix: current_unix(),
            sequence: self.sequence_counter,
            tool_call_id: tool_call_id.map(|s| s.to_string()),
            model_request_id: model_request_id.map(|s| s.to_string()),
        }))
    }

    pub(crate) fn record_tool_event(&mut self, event_type: ToolEventType, turn_id: &str, tool_call_id: &str, tool_name: &str) -> EventId {
        self.record(AgentEvent::Tool(ToolEvent {
            event_type,
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            timestamp_unix: current_unix(),
            sequence: self.sequence_counter,
            tool_call_id: tool_call_id.to_string(),
            tool_name: tool_name.to_string(),
            input_artifact: None,
            output_artifact: None,
            duration_ms: None,
        }))
    }

    pub(crate) fn record_policy_event(&mut self, event_type: PolicyEventType, turn_id: &str, tool_call_id: Option<&str>, reason: &str) -> EventId {
        self.record(AgentEvent::Policy(PolicyEvent {
            event_type,
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            timestamp_unix: current_unix(),
            sequence: self.sequence_counter,
            tool_call_id: tool_call_id.map(|s| s.to_string()),
            reason: reason.to_string(),
        }))
    }

    pub(crate) fn record_evidence_event(&mut self, turn_id: &str, claim_text: &str, source_artifact: &str) -> EventId {
        self.record(AgentEvent::Evidence(EvidenceEvent {
            event_type: EvidenceEventType::EvidenceRecorded,
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            timestamp_unix: current_unix(),
            sequence: self.sequence_counter,
            claim_text: claim_text.to_string(),
            source_artifact: source_artifact.to_string(),
            support_type: EvidenceSupportType::DirectMatch,
        }))
    }

    pub(crate) fn record_finalization(&mut self, event_type: FinalizationEventType, turn_id: &str, stop_reason: &str) -> EventId {
        self.record(AgentEvent::Finalization(FinalizationEvent {
            event_type,
            session_id: self.session_id.clone(),
            turn_id: turn_id.to_string(),
            timestamp_unix: current_unix(),
            sequence: self.sequence_counter,
            stop_reason: stop_reason.to_string(),
        }))
    }

    pub(crate) fn events_for_turn(&self, turn_id: &str) -> Vec<&AgentEvent> {
        self.events
            .iter()
            .filter(|e| match e {
                AgentEvent::Lifecycle(le) => le.turn_id.as_deref() == Some(turn_id),
                AgentEvent::Model(me) => me.turn_id == turn_id,
                AgentEvent::Tool(te) => te.turn_id == turn_id,
                AgentEvent::Policy(pe) => pe.turn_id == turn_id,
                AgentEvent::Evidence(ee) => ee.turn_id == turn_id,
                AgentEvent::Transcript(te) => te.turn_id == turn_id,
                AgentEvent::Finalization(fe) => fe.turn_id == turn_id,
            })
            .collect()
    }

    pub(crate) fn latest_stop_event(&self) -> Option<&FinalizationEvent> {
        self.events
            .iter()
            .rev()
            .filter_map(|e| match e {
                AgentEvent::Finalization(fe) => Some(fe),
                _ => None,
            })
            .next()
    }

    pub(crate) fn tool_events_for_call(&self, tool_call_id: &str) -> Vec<&ToolEvent> {
        self.events
            .iter()
            .rev()
            .filter_map(|e| match e {
                AgentEvent::Tool(te) if te.tool_call_id == tool_call_id => Some(te),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn events(&self) -> &[AgentEvent] {
        &self.events
    }

    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    pub(crate) fn persist(&self, session_root: &Path) -> Result<()> {
        crate::session_write::mutate_session_doc(session_root, |doc| {
            let events_json = serde_json::to_value(&self.events).unwrap_or_default();
            doc["events"] = events_json;
        });
        Ok(())
    }

    pub(crate) fn load_from_session(session_root: &Path, session_id: &str) -> Option<Self> {
        let doc = crate::session_write::load_session_doc(session_root);
        let events: Vec<AgentEvent> = doc.get("events")
            .and_then(|e| serde_json::from_value(e.clone()).ok())
            .unwrap_or_default();
        
        let max_seq = events.iter().filter_map(|e| match e {
            AgentEvent::Lifecycle(le) => Some(le.sequence),
            AgentEvent::Model(me) => Some(me.sequence),
            AgentEvent::Tool(te) => Some(te.sequence),
            AgentEvent::Policy(pe) => Some(pe.sequence),
            AgentEvent::Evidence(ee) => Some(ee.sequence),
            AgentEvent::Transcript(te) => Some(te.sequence),
            AgentEvent::Finalization(fe) => Some(fe.sequence),
        }).max().unwrap_or(0);

        Some(Self {
            session_id: session_id.to_string(),
            events,
            sequence_counter: max_seq + 1,
        })
    }
}

/// Get current Unix timestamp
fn current_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_log_sequential_ids() {
        let mut log = EventLog::new("test_session");
        let id1 = log.record(AgentEvent::Lifecycle(LifecycleEvent {
            event_type: LifecycleEventType::TurnStarted,
            session_id: "test_session".to_string(),
            turn_id: Some("turn_1".to_string()),
            timestamp_unix: current_unix(),
            sequence: 0,
        }));
        let id2 = log.record(AgentEvent::Lifecycle(LifecycleEvent {
            event_type: LifecycleEventType::TurnFinished,
            session_id: "test_session".to_string(),
            turn_id: Some("turn_1".to_string()),
            timestamp_unix: current_unix(),
            sequence: 0,
        }));
        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
    }

    #[test]
    fn test_events_for_turn() {
        let mut log = EventLog::new("test_session");
        log.record(AgentEvent::Lifecycle(LifecycleEvent {
            event_type: LifecycleEventType::TurnStarted,
            session_id: "test_session".to_string(),
            turn_id: Some("turn_1".to_string()),
            timestamp_unix: current_unix(),
            sequence: 0,
        }));
        log.record(AgentEvent::Lifecycle(LifecycleEvent {
            event_type: LifecycleEventType::TurnStarted,
            session_id: "test_session".to_string(),
            turn_id: Some("turn_2".to_string()),
            timestamp_unix: current_unix(),
            sequence: 1,
        }));

        let turn1_events = log.events_for_turn("turn_1");
        assert_eq!(turn1_events.len(), 1);
    }

    #[test]
    fn test_latest_stop_event() {
        let mut log = EventLog::new("test_session");
        log.record(AgentEvent::Finalization(FinalizationEvent {
            event_type: FinalizationEventType::StopPolicyTriggered,
            session_id: "test_session".to_string(),
            turn_id: "turn_1".to_string(),
            timestamp_unix: current_unix(),
            sequence: 0,
            stop_reason: "max_tokens".to_string(),
        }));
        log.record(AgentEvent::Finalization(FinalizationEvent {
            event_type: FinalizationEventType::FinalAnswerPrepared,
            session_id: "test_session".to_string(),
            turn_id: "turn_1".to_string(),
            timestamp_unix: current_unix(),
            sequence: 1,
            stop_reason: "done".to_string(),
        }));

        let latest = log.latest_stop_event();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().stop_reason, "done");
    }
}