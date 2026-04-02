//! @efficiency-role: service-orchestrator
//!
//! Workspace snapshot creation and management.

use crate::*;
use std::collections::HashSet;

const SNAPSHOT_MANIFEST_VERSION: u32 = 1;

pub(crate) fn create_workspace_snapshot(
    session: &SessionPaths,
    repo_root: &Path,
    reason: &str,
    automatic: bool,
) -> Result<SnapshotCreateResult> {
    let snapshot_id = new_snapshot_id(&session.snapshots_dir)?;
    let snapshot_dir = session.snapshots_dir.join(&snapshot_id);
    let files_dir = snapshot_dir.join("files");
    std::fs::create_dir_all(&files_dir)
        .with_context(|| format!("mkdir {}", files_dir.display()))?;

    let (git_aware, scope_mode, files) = collect_snapshot_relative_files(repo_root)?;
    for rel in &files {
        let src = repo_root.join(rel);
        let dst = files_dir.join(rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }

    let manifest = SnapshotManifest {
        version: SNAPSHOT_MANIFEST_VERSION,
        snapshot_id: snapshot_id.clone(),
        created_unix_s: now_unix_s()?,
        automatic,
        reason: reason.trim().to_string(),
        repo_root: repo_root.display().to_string(),
        git_aware,
        scope_mode,
        file_count: files.len() as u64,
        files: files.iter().map(|rel| rel.display().to_string()).collect(),
    };
    let manifest_path = snapshot_dir.join("manifest.toml");
    let body = toml::to_string_pretty(&manifest).context("serialize snapshot manifest")?;
    std::fs::write(&manifest_path, body)
        .with_context(|| format!("write {}", manifest_path.display()))?;

    Ok(SnapshotCreateResult {
        snapshot_id,
        snapshot_dir,
        manifest_path,
        file_count: manifest.file_count,
        automatic,
    })
}

pub(crate) fn rollback_workspace_snapshot(
    session: &SessionPaths,
    repo_root: &Path,
    snapshot_id: &str,
) -> Result<RollbackResult> {
    let snapshot_id = snapshot_id.trim();
    if snapshot_id.is_empty() {
        anyhow::bail!("snapshot id is required");
    }

    let snapshot_dir = session.snapshots_dir.join(snapshot_id);
    let manifest_path = snapshot_dir.join("manifest.toml");
    if !manifest_path.exists() {
        anyhow::bail!("snapshot {} was not found in this session", snapshot_id);
    }
    let manifest = load_snapshot_manifest(&manifest_path)?;
    let files_dir = snapshot_dir.join("files");
    let expected_set = manifest.files.iter().cloned().collect::<HashSet<String>>();

    let mut restored_files = 0u64;
    let mut verified_files = 0u64;
    for rel_str in &manifest.files {
        let rel = PathBuf::from(rel_str);
        let src = files_dir.join(&rel);
        let dst = repo_root.join(&rel);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
        restored_files += 1;
        if file_bytes_equal(&src, &dst)? {
            verified_files += 1;
        }
    }

    let (_, _, current_files) = collect_snapshot_relative_files(repo_root)?;
    let mut removed_files = 0u64;
    for rel in current_files {
        let rel_string = rel.display().to_string();
        if expected_set.contains(&rel_string) {
            continue;
        }
        let path = repo_root.join(&rel);
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
            removed_files += 1;
            cleanup_empty_parents(repo_root, path.parent());
        }
    }

    Ok(RollbackResult {
        snapshot_id: snapshot_id.to_string(),
        manifest_path,
        restored_files,
        removed_files,
        verified_files,
    })
}

fn load_snapshot_manifest(path: &Path) -> Result<SnapshotManifest> {
    let bytes = std::fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let text = String::from_utf8(bytes).context("snapshot manifest is not valid UTF-8")?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

fn new_snapshot_id(snapshots_dir: &Path) -> Result<String> {
    Ok(format!("snap_{:03}", next_snapshot_seq(snapshots_dir)?))
}

fn next_snapshot_seq(snapshots_dir: &Path) -> Result<u32> {
    let mut max_n = 0u32;
    if !snapshots_dir.exists() {
        return Ok(1);
    }
    for ent in std::fs::read_dir(snapshots_dir)
        .with_context(|| format!("read_dir {}", snapshots_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("snap_") {
            if let Ok(n) = rest.parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

fn collect_snapshot_relative_files(repo_root: &Path) -> Result<(bool, String, Vec<PathBuf>)> {
    if is_git_workspace(repo_root) {
        let files = collect_git_snapshot_files(repo_root)?;
        return Ok((true, "git_ls_files".to_string(), files));
    }
    let mut files = Vec::new();
    walk_snapshot_files(repo_root, repo_root, &mut files)?;
    files.sort();
    files.dedup();
    Ok((false, "workspace_walk".to_string(), files))
}

fn is_git_workspace(repo_root: &Path) -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .current_dir(repo_root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn collect_git_snapshot_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(repo_root)
        .output()
        .with_context(|| format!("git ls-files in {}", repo_root.display()))?;
    if !output.status.success() {
        anyhow::bail!("git ls-files failed in {}", repo_root.display());
    }

    let mut files = Vec::new();
    for chunk in output.stdout.split(|byte| *byte == 0) {
        if chunk.is_empty() {
            continue;
        }
        let rel_text = String::from_utf8_lossy(chunk).trim().to_string();
        if rel_text.is_empty() {
            continue;
        }
        let rel = PathBuf::from(rel_text);
        if snapshot_path_excluded(&rel) {
            continue;
        }
        let abs = repo_root.join(&rel);
        if abs.is_file() {
            files.push(rel);
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn walk_snapshot_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for ent in std::fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let ent = ent?;
        let path = ent.path();
        let rel = path
            .strip_prefix(root)
            .with_context(|| format!("strip_prefix {} from {}", root.display(), path.display()))?;
        if snapshot_path_excluded(rel) {
            continue;
        }
        let file_type = ent
            .file_type()
            .with_context(|| format!("file_type {}", path.display()))?;
        if file_type.is_dir() {
            walk_snapshot_files(root, &path, out)?;
        } else if file_type.is_file() {
            out.push(rel.to_path_buf());
        }
    }
    Ok(())
}

fn snapshot_path_excluded(rel: &Path) -> bool {
    let components = rel
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    if components.is_empty() {
        return false;
    }
    if components[0].starts_with(".git") || components[0] == "target" || components[0] == "sessions"
    {
        return true;
    }
    if components.len() >= 3
        && components[0] == "config"
        && matches!(
            components[2].as_str(),
            "baseline" | "fallback" | "tune" | "formula_memory"
        )
    {
        return true;
    }
    components
        .last()
        .map(|name| name == ".DS_Store")
        .unwrap_or(false)
}

fn file_bytes_equal(a: &Path, b: &Path) -> Result<bool> {
    let a_bytes = std::fs::read(a).with_context(|| format!("read {}", a.display()))?;
    let b_bytes = std::fs::read(b).with_context(|| format!("read {}", b.display()))?;
    Ok(a_bytes == b_bytes)
}

fn cleanup_empty_parents(repo_root: &Path, mut dir: Option<&Path>) {
    while let Some(path) = dir {
        if path == repo_root {
            break;
        }
        let is_empty = std::fs::read_dir(path)
            .ok()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !is_empty {
            break;
        }
        let _ = std::fs::remove_dir(path);
        dir = path.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_test_root(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("elma_cli_{label}_{unique}"))
    }

    #[test]
    fn snapshot_create_and_rollback_restores_workspace() -> Result<()> {
        let repo = temp_test_root("snapshot");
        std::fs::create_dir_all(repo.join("src"))?;
        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\n")?;
        std::fs::write(repo.join("src").join("main.rs"), "fn main() {}\n")?;
        std::fs::create_dir_all(repo.join("target"))?;
        std::fs::write(repo.join("target").join("ignored.txt"), "ignored\n")?;

        let session = ensure_session_layout(&repo.join("sessions"))?;
        let created = create_workspace_snapshot(&session, &repo, "test snapshot", false)?;
        assert_eq!(created.snapshot_id, "snap_001");
        assert!(created.manifest_path.exists());

        std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"changed\"\n")?;
        std::fs::write(repo.join("new.txt"), "created after snapshot\n")?;

        let rollback = rollback_workspace_snapshot(&session, &repo, "snap_001")?;
        assert_eq!(rollback.snapshot_id, "snap_001");
        assert_eq!(
            std::fs::read_to_string(repo.join("Cargo.toml"))?,
            "[package]\nname = \"demo\"\n"
        );
        assert!(!repo.join("new.txt").exists());
        assert!(rollback.restored_files >= 2);
        assert!(rollback.verified_files >= 2);

        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }
}
