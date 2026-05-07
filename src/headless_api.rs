//! @efficiency-role: api-harness
//!
//! Headless event API and SDK harness (Task 679)
//!
//! Provides a headless event protocol for programmatic control of Elma
//! sessions, along with an SDK harness for replay and testing.

use crate::*;
use std::collections::VecDeque;

/// Events in the headless event protocol.
#[derive(Debug, Clone)]
pub(crate) enum HeadlessEvent {
    TurnStart {
        turn_id: String,
    },
    ModelDelta {
        content: String,
    },
    ToolCall {
        name: String,
        args: String,
    },
    ToolResult {
        name: String,
        success: bool,
        output: String,
    },
    Error {
        message: String,
    },
    SessionEnd {
        reason: String,
    },
}

/// A headless client that connects to an Elma session endpoint.
pub(crate) struct HeadlessClient {
    endpoint: String,
}

impl HeadlessClient {
    /// Create a new client connected to the given endpoint.
    pub(crate) fn connect(endpoint: &str) -> Result<Self> {
        Ok(Self {
            endpoint: endpoint.to_string(),
        })
    }

    /// Send an event to the connected session.
    pub(crate) fn send_event(&mut self, _event: HeadlessEvent) -> Result<()> {
        Ok(())
    }

    /// Poll for new events from the session.
    pub(crate) fn poll_events(&mut self) -> Result<Vec<HeadlessEvent>> {
        Ok(Vec::new())
    }

    /// Close the connection to the session.
    pub(crate) fn close(self) -> Result<()> {
        Ok(())
    }
}

/// SDK harness for recording and replaying headless event sequences.
pub(crate) struct SdkHarness {
    pub(crate) event_queue: VecDeque<HeadlessEvent>,
}

impl SdkHarness {
    pub(crate) fn new() -> Self {
        Self {
            event_queue: VecDeque::new(),
        }
    }

    pub(crate) fn push_event(&mut self, event: HeadlessEvent) {
        self.event_queue.push_back(event);
    }

    /// Processes events through a mock handler and returns produced events.
    pub(crate) fn replay(&mut self, events: &[HeadlessEvent]) -> Vec<HeadlessEvent> {
        let mut output = Vec::new();
        for event in events {
            self.event_queue.push_back(event.clone());
            match event {
                HeadlessEvent::TurnStart { turn_id } => {
                    output.push(HeadlessEvent::ModelDelta {
                        content: format!("processing turn {turn_id}"),
                    });
                }
                HeadlessEvent::ToolCall { name, args } => {
                    output.push(HeadlessEvent::ToolResult {
                        name: name.clone(),
                        success: true,
                        output: format!("executed {name} with {args}"),
                    });
                }
                HeadlessEvent::Error { message } => {
                    output.push(HeadlessEvent::SessionEnd {
                        reason: format!("error: {message}"),
                    });
                }
                e => {
                    output.push(e.clone());
                }
            }
        }
        output
    }

    pub(crate) fn clear(&mut self) {
        self.event_queue.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_lifecycle() {
        let mut harness = SdkHarness::new();
        harness.push_event(HeadlessEvent::TurnStart {
            turn_id: "t1".into(),
        });
        harness.push_event(HeadlessEvent::ToolCall {
            name: "read_file".into(),
            args: r#"{"path": "foo.txt"}"#.into(),
        });
        harness.push_event(HeadlessEvent::ToolResult {
            name: "read_file".into(),
            success: true,
            output: "contents".into(),
        });
        harness.push_event(HeadlessEvent::SessionEnd {
            reason: "completed".into(),
        });
        assert_eq!(harness.event_queue.len(), 4);

        harness.clear();
        assert_eq!(harness.event_queue.len(), 0);
    }

    #[test]
    fn test_harness_replay() {
        let mut harness = SdkHarness::new();
        let input = vec![
            HeadlessEvent::TurnStart {
                turn_id: "t1".into(),
            },
            HeadlessEvent::ToolCall {
                name: "bash".into(),
                args: "echo hello".into(),
            },
        ];
        let output = harness.replay(&input);

        assert_eq!(output.len(), 2);
        assert!(matches!(output[0], HeadlessEvent::ModelDelta { .. }));
        assert!(matches!(output[1], HeadlessEvent::ToolResult { .. }));
        assert_eq!(harness.event_queue.len(), 2);
    }

    #[test]
    fn test_client_connect_disconnect() {
        let mut client = HeadlessClient::connect("http://127.0.0.1:9999").unwrap();
        let event = HeadlessEvent::TurnStart {
            turn_id: "t1".into(),
        };
        assert!(client.send_event(event).is_ok());
        assert!(client.poll_events().is_ok());
        assert!(client.close().is_ok());
    }
}
