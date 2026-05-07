//! @efficiency-role: domain-logic
//!
//! Remote daemon channel and notification integrations (Task 684).
//! Provides a mock remote daemon lifecycle and notification dispatch.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct DaemonConfig {
    pub(crate) port: u16,
    pub(crate) host: String,
    pub(crate) auth_token: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            port: 9876,
            host: "127.0.0.1".to_string(),
            auth_token: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NotificationChannel {
    Stdout,
    File(PathBuf),
    Webhook(String),
    None,
}

static DAEMON_RUNNING: AtomicBool = AtomicBool::new(false);

pub(crate) struct RemoteDaemon;

impl RemoteDaemon {
    pub(crate) fn start(config: DaemonConfig) -> anyhow::Result<()> {
        if DAEMON_RUNNING.load(Ordering::SeqCst) {
            anyhow::bail!("daemon is already running");
        }
        if config.port == 0 {
            anyhow::bail!("port must be non-zero");
        }
        if config.host.is_empty() {
            anyhow::bail!("host must not be empty");
        }
        DAEMON_RUNNING.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub(crate) fn stop() -> anyhow::Result<()> {
        if !DAEMON_RUNNING.load(Ordering::SeqCst) {
            anyhow::bail!("daemon is not running");
        }
        DAEMON_RUNNING.store(false, Ordering::SeqCst);
        Ok(())
    }

    pub(crate) fn is_running() -> bool {
        DAEMON_RUNNING.load(Ordering::SeqCst)
    }

    pub(crate) fn send_notification(
        channel: &NotificationChannel,
        message: &str,
    ) -> anyhow::Result<()> {
        match channel {
            NotificationChannel::Stdout => {
                println!("[elma-daemon] {}", message);
                Ok(())
            }
            NotificationChannel::File(path) => {
                let entry = format!(
                    "[{}] {}\n",
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    message
                );
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                    .map_err(|e| anyhow::anyhow!("cannot open notification file: {}", e))
                    .and_then(|mut f| {
                        use std::io::Write;
                        f.write_all(entry.as_bytes())
                            .map_err(|e| anyhow::anyhow!("write error: {}", e))
                    })
            }
            NotificationChannel::Webhook(url) => {
                let _ = url;
                anyhow::bail!(
                    "webhook notification not implemented (dry run for url: {})",
                    url
                );
            }
            NotificationChannel::None => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_start_stop() {
        let _ = RemoteDaemon::stop();
        let config = DaemonConfig::default();
        assert!(!RemoteDaemon::is_running());

        RemoteDaemon::start(config).unwrap();
        assert!(RemoteDaemon::is_running());

        RemoteDaemon::stop().unwrap();
        assert!(!RemoteDaemon::is_running());
    }

    #[test]
    fn test_double_start_fails() {
        let _ = RemoteDaemon::stop();
        let config = DaemonConfig::default();
        RemoteDaemon::start(config).unwrap();
        assert!(RemoteDaemon::start(DaemonConfig::default()).is_err());
        RemoteDaemon::stop().unwrap();
    }

    #[test]
    fn test_stop_not_running_fails() {
        assert!(RemoteDaemon::is_running() == false || RemoteDaemon::stop().is_err());
        // ensure stopped
        let _ = RemoteDaemon::stop();
        assert!(RemoteDaemon::stop().is_err());
    }

    #[test]
    fn test_start_zero_port_fails() {
        let _ = RemoteDaemon::stop();
        let config = DaemonConfig {
            port: 0,
            ..Default::default()
        };
        assert!(RemoteDaemon::start(config).is_err());
    }

    #[test]
    fn test_start_empty_host_fails() {
        let _ = RemoteDaemon::stop();
        let config = DaemonConfig {
            host: "".to_string(),
            ..Default::default()
        };
        assert!(RemoteDaemon::start(config).is_err());
    }

    #[test]
    fn test_send_notification_none() {
        assert!(RemoteDaemon::send_notification(&NotificationChannel::None, "test").is_ok());
    }

    #[test]
    fn test_send_notification_stdout() {
        assert!(RemoteDaemon::send_notification(&NotificationChannel::Stdout, "hello").is_ok());
    }

    #[test]
    fn test_send_notification_file() {
        let dir = TempDir::new().unwrap();
        let log = dir.path().join("daemon.log");
        assert!(RemoteDaemon::send_notification(
            &NotificationChannel::File(log.clone()),
            "test message"
        )
        .is_ok());
        let contents = std::fs::read_to_string(&log).unwrap();
        assert!(contents.contains("test message"));
    }

    #[test]
    fn test_send_notification_webhook_returns_error() {
        let result = RemoteDaemon::send_notification(
            &NotificationChannel::Webhook("http://hook.example.com".into()),
            "ping",
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not implemented"));
    }

    #[test]
    fn test_daemon_config_default() {
        let config = DaemonConfig::default();
        assert_eq!(config.port, 9876);
        assert_eq!(config.host, "127.0.0.1");
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_notification_channel_equality() {
        assert_eq!(NotificationChannel::Stdout, NotificationChannel::Stdout);
        assert_ne!(
            NotificationChannel::File(PathBuf::from("a.log")),
            NotificationChannel::File(PathBuf::from("b.log"))
        );
    }

    #[test]
    fn test_config_with_auth() {
        let config = DaemonConfig {
            auth_token: Some("s3cret".into()),
            ..Default::default()
        };
        assert_eq!(config.auth_token.as_deref(), Some("s3cret"));
    }
}
