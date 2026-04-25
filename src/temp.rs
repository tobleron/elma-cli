//! @efficiency-role: util-pure
//!
//! Safe temporary files and directories.

use std::path::Path;
use tempfile::TempDir;

pub struct ScopedTemp {
    dir: TempDir,
}

impl ScopedTemp {
    pub fn new(prefix: &str) -> anyhow::Result<Self> {
        let dir = tempfile::Builder::new().prefix(prefix).tempdir()?;
        Ok(Self { dir })
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }
}
