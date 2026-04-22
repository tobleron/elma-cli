use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct Shutdown {
    cancel_tx: broadcast::Sender<()>,
    is_shutting_down: Arc<AtomicBool>,
}

impl Shutdown {
    pub fn new() -> Self {
        let (cancel_tx, _) = broadcast::channel(1);
        Self {
            cancel_tx,
            is_shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.cancel_tx.subscribe()
    }

    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::SeqCst)
    }

    pub async fn shutdown(&self) {
        if self.is_shutting_down.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = self.cancel_tx.send(());
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shutdown_signal() {
        let shutdown = Arc::new(Shutdown::new());

        assert!(!shutdown.is_shutting_down());

        shutdown.shutdown().await;

        assert!(shutdown.is_shutting_down());
    }

    #[tokio::test]
    async fn test_subscriber_receives() {
        let shutdown = Arc::new(Shutdown::new());
        let mut rx = shutdown.subscribe();

        shutdown.shutdown().await;

        rx.recv().await.expect("should receive shutdown signal");
    }

    #[tokio::test]
    async fn test_idempotent_shutdown() {
        let shutdown = Arc::new(Shutdown::new());

        shutdown.shutdown().await;
        shutdown.shutdown().await;

        assert!(shutdown.is_shutting_down());
    }
}
