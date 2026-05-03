//! @efficiency-role: util-pure
//! Injectable abstractions for testable tool execution.
//!
//! Provides traits for IO, time, and HTTP dependencies so components
//! can be tested with mock implementations.

use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// Abstract filesystem for tool execution.
pub trait FileSystem: Send + Sync {
    fn read_to_string(&self, path: &Path) -> io::Result<String>;
    fn write(&self, path: &Path, content: &str) -> io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
}

/// Real filesystem implementation using std::fs.
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn read_to_string(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn write(&self, path: &Path, content: &str) -> io::Result<()> {
        std::fs::write(path, content)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        }
    }
}

/// Abstract clock for time-dependent operations.
pub trait Clock: Send + Sync {
    fn now(&self) -> SystemTime;
    fn elapsed(&self, start: SystemTime) -> Duration;
}

/// Real clock implementation.
pub struct RealClock;

impl Clock for RealClock {
    fn now(&self) -> SystemTime {
        SystemTime::now()
    }

    fn elapsed(&self, start: SystemTime) -> Duration {
        SystemTime::now().duration_since(start).unwrap_or_default()
    }
}
