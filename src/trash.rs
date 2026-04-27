use std::path::{Path, PathBuf};
use trash::Error;

pub fn trash_file(path: &Path) -> Result<(), Error> {
    trash::delete(path)
}

pub fn trash_batch(paths: &[PathBuf]) -> Vec<Result<(), Error>> {
    paths.iter().map(|p| trash_file(p.as_path())).collect()
}

pub fn trash_file_fallback(path: &Path) {
    if let Err(e) = trash_file(path) {
        tracing::warn!(
            "trash failed for {}, falling back to permanent delete: {}",
            path.display(),
            e
        );
        let _ = std::fs::remove_file(path);
    }
}
