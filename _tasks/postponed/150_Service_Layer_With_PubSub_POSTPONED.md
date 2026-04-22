# 150 Service Layer With PubSub

## Summary
Implement service layer pattern with pub/sub event bus for loose coupling.

## Reference
- OpenCode: `internal/pubsub/broker.go`, `internal/session/session.go`

## Implementation

### 1. Event Types
File: `src/events.rs` (new)
```rust
pub enum Event {
    SessionCreated(Session),
    SessionUpdated(Session),
    SessionDeleted(String),
    MessageCreated(Message),
    AgentEvent(AgentEvent),
    PermissionRequest(PermissionRequest),
}
```

### 2. Broker/Subscriber
File: `src/pubsub.rs` (new)
```rust
pub struct Broker<T: Clone> {
    subscribers: Vec<flume::Sender<T>>,
}

impl<T: Clone> Broker<T> {
    pub fn subscribe(&self) -> flume::Receiver<T> { ... }
    pub fn publish(&self, event: T) { ... }
}
```

### 3. Service Trait
File: `src/service.rs` (new)
```rust
pub trait Service {
    type Event: Clone;
    fn publisher(&self) -> &Broker<Self::Event>;
}
```

### 4. Integrate
- `SessionService` publishes session events
- `MessageService` publishes message events
- TUI subscribes and updates reactively

## Verification
- [ ] `cargo build` passes
- [ ] Events publish/subscribe correctly
- [ ] Services implement trait