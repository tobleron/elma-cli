//! @efficiency-role: domain-logic
//!
//! File watcher, AI comment, and autosave workflow (Task 682).
//! Provides primitive file-watching infrastructure and autosave/recovery.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchKind {
    Modified,
    Created,
    Deleted,
}

#[derive(Debug, Clone)]
pub(crate) struct FileWatcherEvent {
    pub(crate) path: PathBuf,
    pub(crate) kind: WatchKind,
}

struct WatchedPath {
    path: PathBuf,
    last_modified: Option<std::time::SystemTime>,
}

pub(crate) struct FileWatcher {
    watched: Arc<Mutex<Vec<WatchedPath>>>,
    events: Arc<Mutex<Vec<FileWatcherEvent>>>,
}

impl FileWatcher {
    pub(crate) fn new() -> Self {
        Self {
            watched: Arc::new(Mutex::new(Vec::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub(crate) fn watch(&self, path: &Path) -> anyhow::Result<()> {
        if !path.exists() {
            anyhow::bail!("path does not exist: {}", path.display());
        }
        let mut watched = self
            .watched
            .lock()
            .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        if watched.iter().any(|w| w.path == path) {
            anyhow::bail!("already watching: {}", path.display());
        }
        let last_modified = path.metadata().ok().and_then(|m| m.modified().ok());
        watched.push(WatchedPath {
            path: path.to_path_buf(),
            last_modified,
        });
        Ok(())
    }

    pub(crate) fn unwatch(&self, path: &Path) -> anyhow::Result<()> {
        let mut watched = self
            .watched
            .lock()
            .map_err(|e| anyhow::anyhow!("lock: {}", e))?;
        let len_before = watched.len();
        watched.retain(|w| w.path != path);
        if watched.len() == len_before {
            anyhow::bail!("not watching: {}", path.display());
        }
        Ok(())
    }

    pub(crate) fn poll_events(&self) -> Vec<FileWatcherEvent> {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        let result = events.clone();
        events.clear();
        result
    }
}

pub(crate) struct AutosaveService;

impl AutosaveService {
    pub(crate) fn autosave(path: &Path, content: &str) -> anyhow::Result<PathBuf> {
        let autosave_path = path.with_extension("autosave");
        std::fs::write(&autosave_path, content)?;
        Ok(autosave_path)
    }

    pub(crate) fn recover(path: &Path) -> Option<String> {
        let autosave_path = path.with_extension("autosave");
        if autosave_path.exists() {
            std::fs::read_to_string(&autosave_path).ok()
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watch_and_unwatch() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.txt");
        fs::write(&file, "hello").unwrap();

        let watcher = FileWatcher::new();
        watcher.watch(&file).unwrap();
        watcher.unwatch(&file).unwrap();
    }

    #[test]
    fn test_watch_nonexistent_fails() {
        let watcher = FileWatcher::new();
        assert!(watcher.watch(Path::new("/nonexistent_file_12345")).is_err());
    }

    #[test]
    fn test_unwatch_nonexistent_fails() {
        let watcher = FileWatcher::new();
        assert!(watcher.unwatch(Path::new("/nonexistent")).is_err());
    }

    #[test]
    fn test_poll_events_empty() {
        let watcher = FileWatcher::new();
        assert!(watcher.poll_events().is_empty());
    }

    #[test]
    fn test_autosave_and_recover() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("doc.md");
        AutosaveService::autosave(&file, "autosaved content").unwrap();

        let autosave_path = file.with_extension("autosave");
        assert!(autosave_path.exists());

        let recovered = AutosaveService::recover(&file);
        assert_eq!(recovered, Some("autosaved content".to_string()));
    }

    #[test]
    fn test_recover_no_autosave() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("no_backup.txt");
        assert_eq!(AutosaveService::recover(&file), None);
    }

    #[test]
    fn test_autosave_overwrites_previous() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("overwrite.md");
        AutosaveService::autosave(&file, "first").unwrap();
        AutosaveService::autosave(&file, "second").unwrap();
        let recovered = AutosaveService::recover(&file);
        assert_eq!(recovered, Some("second".to_string()));
    }

    #[test]
    fn test_watch_duplicate_fails() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("dup.txt");
        fs::write(&file, "data").unwrap();

        let watcher = FileWatcher::new();
        watcher.watch(&file).unwrap();
        assert!(watcher.watch(&file).is_err());
    }

    #[test]
    fn test_watch_kind_equality() {
        assert_eq!(WatchKind::Modified, WatchKind::Modified);
        assert_ne!(WatchKind::Created, WatchKind::Deleted);
    }

    #[test]
    fn test_file_watcher_event_debug() {
        let ev = FileWatcherEvent {
            path: PathBuf::from("/a/b.rs"),
            kind: WatchKind::Created,
        };
        let s = format!("{:?}", ev);
        assert!(s.contains("Created"));
        assert!(s.contains("/a/b.rs"));
    }
}
