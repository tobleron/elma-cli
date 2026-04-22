# Task 159: Generic Pub/Sub Broker

## Summary

Implement a generic pub/sub pattern for decoupled communication between components.

## Motivation

Crush uses a clean pub/sub pattern for:
- Skill discovery events
- Permission requests/notifications
- File tracker updates
- Session events

Elma needs to decouple components without direct dependencies.

## Source

Crush's pubsub package at `_stress_testing/_crush/internal/pubsub/broker.go`

## Implementation

### Types

```rust
pub struct Event<T> {
    pub ty: EventType,
    pub payload: T,
}

pub struct Broker<T> {
    subs: HashMap<chan Event<T>, ()>,
    mu: RwLock<()>,
    done: chan (),
    sub_count: usize,
    max_events: usize,
}
```

### API

```rust
// Create broker with custom buffer sizes
fn new_broker_with_options<T>(channel_buffer_size: usize, max_events: usize) -> Broker<T>
fn new_broker<T>() -> Broker<T>  // defaults: buffer 64, max 1000

// Subscribe - returns channel that receives events
fn subscribe<T>(ctx: Context) -> chan Event<T>

// Publish event to all subscribers
fn publish<T>(ty: EventType, payload: T)

// Get subscriber count
fn get_subscriber_count<T>() -> usize

// Shutdown broker and close all channels
fn shutdown<T>()
```

### Event Types

```rust
const CreatedEvent EventType = "created"
const UpdatedEvent EventType = "updated"
const DeletedEvent EventType = "deleted"
```

### Features

- Non-blocking publish (skip slow subscribers)
- Context-based subscription cleanup
- Thread-safe
- Shutdown support
- Configurable buffer sizes

### Usage Example

```rust
let broker = Broker::<PermissionNotification>::new();
let rx = broker.subscribe(ctx);

// In permission service:
broker.publish(CreatedEvent, notif);

// In UI:
loop {
    select! {
        event <- rx => handle(event),
        <- ctx.done() => break,
    }
}
```

## Verification

- Subscribers receive published events
- Multiple subscribers work
- Context cancellation cleans up subscriptions
- Shutdown closes channels

## Dependencies

- None (pure Rust)

## Notes

- Generic over payload type
- Channel-based (not async)
- Simple but effective