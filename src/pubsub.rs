use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

pub struct Event<T: Clone + Send + 'static> {
    pub ty: EventType,
    pub payload: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventType(pub &'static str);

pub const CREATED_EVENT: EventType = EventType("created");
pub const UPDATED_EVENT: EventType = EventType("updated");
pub const DELETED_EVENT: EventType = EventType("deleted");

pub struct Broker<T: Clone + Send + 'static> {
    subscriptions: Arc<RwLock<HashMap<String, broadcast::Sender<T>>>>,
    event_type: EventType,
    max_subscribers: usize,
}

impl<T: Clone + Send + 'static> Broker<T> {
    pub fn new(event_type: EventType) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_type,
            max_subscribers: 64,
        }
    }

    pub fn with_capacity(event_type: EventType, max_subscribers: usize) -> Self {
        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_type,
            max_subscribers,
        }
    }

    pub async fn subscribe(&self, id: &str) -> broadcast::Receiver<T> {
        let mut subs = self.subscriptions.write().await;

        if subs.len() >= self.max_subscribers {
            return broadcast::channel(64).1;
        }

        let (tx, rx) = broadcast::channel(64);
        subs.insert(id.to_string(), tx);
        rx
    }

    pub async fn unsubscribe(&self, id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(id);
    }

    pub async fn publish(&self, payload: T) {
        let subs = self.subscriptions.read().await;

        for sender in subs.values() {
            let _ = sender.send(payload.clone());
        }
    }

    pub async fn subscriber_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    pub async fn shutdown(&self) {
        let mut subs = self.subscriptions.write().await;
        subs.clear();
    }
}

impl<T: Clone + Send + 'static> Default for Broker<T> {
    fn default() -> Self {
        Self::new(EventType("default"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_single_subscriber() {
        let broker: Broker<String> = Broker::new(CREATED_EVENT);

        let mut rx = broker.subscribe("sub1").await;

        broker.publish("hello".to_string()).await;

        let received = rx.recv().await.unwrap();
        assert_eq!(received, "hello");
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broker: Broker<String> = Broker::new(UPDATED_EVENT);

        let mut rx1 = broker.subscribe("sub1").await;
        let mut rx2 = broker.subscribe("sub2").await;

        broker.publish("test".to_string()).await;

        assert_eq!(rx1.recv().await.unwrap(), "test");
        assert_eq!(rx2.recv().await.unwrap(), "test");
    }

    #[tokio::test]
    async fn test_unsubscribe() {
        let broker: Broker<i32> = Broker::new(DELETED_EVENT);

        broker.subscribe("sub1").await;
        broker.subscribe("sub2").await;

        assert_eq!(broker.subscriber_count().await, 2);

        broker.unsubscribe("sub1").await;

        assert_eq!(broker.subscriber_count().await, 1);
    }

    #[tokio::test]
    async fn test_shutdown() {
        let broker: Broker<String> = Broker::new(CREATED_EVENT);

        broker.subscribe("sub1").await;
        broker.subscribe("sub2").await;

        assert_eq!(broker.subscriber_count().await, 2);

        broker.shutdown().await;

        assert_eq!(broker.subscriber_count().await, 0);
    }
}
