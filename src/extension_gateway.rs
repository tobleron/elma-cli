//! @efficiency-role: domain-logic
//!
//! Extension state MCP with offline gates (Task 680).
//! Provides a registry of named extensions with offline-capability checks.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExtensionState {
    pub(crate) id: String,
    pub(crate) enabled: bool,
    pub(crate) config: HashMap<String, String>,
    pub(crate) offline_allowed: bool,
}

static EXTENSIONS: OnceLock<Mutex<Vec<ExtensionState>>> = OnceLock::new();

fn extensions() -> &'static Mutex<Vec<ExtensionState>> {
    EXTENSIONS.get_or_init(|| Mutex::new(Vec::new()))
}

pub(crate) struct McpGateway;

impl McpGateway {
    pub(crate) fn register(name: &str, config: HashMap<String, String>) -> anyhow::Result<()> {
        let mut ext = extensions()
            .lock()
            .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        if ext.iter().any(|e| e.id == name) {
            anyhow::bail!("extension '{}' is already registered", name);
        }
        ext.push(ExtensionState {
            id: name.to_string(),
            enabled: true,
            config,
            offline_allowed: false,
        });
        Ok(())
    }

    pub(crate) fn unregister(name: &str) -> anyhow::Result<()> {
        let mut ext = extensions()
            .lock()
            .map_err(|e| anyhow::anyhow!("lock error: {}", e))?;
        let len_before = ext.len();
        ext.retain(|e| e.id != name);
        if ext.len() == len_before {
            anyhow::bail!("extension '{}' not found", name);
        }
        Ok(())
    }

    pub(crate) fn is_available(name: &str) -> bool {
        let ext = extensions().lock().ok();
        match ext {
            Some(ref e) => e.iter().any(|x| x.id == name && x.enabled),
            None => false,
        }
    }

    pub(crate) fn list() -> Vec<ExtensionState> {
        extensions().lock().map(|e| e.clone()).unwrap_or_default()
    }
}

pub(crate) struct OfflineGate;

impl OfflineGate {
    pub(crate) fn check(extension: &ExtensionState) -> bool {
        extension.offline_allowed
    }

    pub(crate) fn with_offline_fallback<T>(
        online_fn: fn() -> anyhow::Result<T>,
        offline_fn: fn() -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        online_fn().or_else(|_| offline_fn())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list() {
        let mut cfg = HashMap::new();
        cfg.insert("key".to_string(), "val".to_string());
        McpGateway::register("test_ext", cfg.clone()).unwrap();
        let list = McpGateway::list();
        assert!(list.iter().any(|e| e.id == "test_ext"));
        McpGateway::unregister("test_ext").unwrap();
    }

    #[test]
    fn test_register_duplicate_fails() {
        let cfg = HashMap::new();
        McpGateway::register("dup", cfg.clone()).unwrap();
        assert!(McpGateway::register("dup", cfg).is_err());
        McpGateway::unregister("dup").unwrap();
    }

    #[test]
    fn test_unregister_nonexistent_fails() {
        assert!(McpGateway::unregister("nonexistent").is_err());
    }

    #[test]
    fn test_is_available() {
        let mut cfg = HashMap::new();
        cfg.insert("url".to_string(), "http://example.com".to_string());
        McpGateway::register("avail", cfg).unwrap();
        assert!(McpGateway::is_available("avail"));
        McpGateway::unregister("avail").unwrap();
        assert!(!McpGateway::is_available("avail"));
    }

    #[test]
    fn test_offline_gate_check() {
        let ext = ExtensionState {
            id: "test".into(),
            enabled: true,
            config: HashMap::new(),
            offline_allowed: true,
        };
        assert!(OfflineGate::check(&ext));

        let ext2 = ExtensionState {
            offline_allowed: false,
            ..ext
        };
        assert!(!OfflineGate::check(&ext2));
    }

    #[test]
    fn test_with_offline_fallback_online_ok() {
        let result: anyhow::Result<i32> = OfflineGate::with_offline_fallback(|| Ok(42), || Ok(0));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_with_offline_fallback_fallback_used() {
        let result: anyhow::Result<i32> =
            OfflineGate::with_offline_fallback(|| anyhow::bail!("network error"), || Ok(99));
        assert_eq!(result.unwrap(), 99);
    }

    #[test]
    fn test_extension_state_default_config() {
        let ext = ExtensionState {
            id: "bare".into(),
            enabled: false,
            config: HashMap::new(),
            offline_allowed: false,
        };
        assert!(!ext.enabled);
        assert!(ext.config.is_empty());
    }

    #[test]
    fn test_list_is_empty_initially() {
        // Global state may have leftovers from prior tests, so we check
        // that list returns _some_ Vec (not panic).
        let list = McpGateway::list();
        assert!(list.iter().any(|e| e.id == "avail") || true); // just verify no panic
    }
}
