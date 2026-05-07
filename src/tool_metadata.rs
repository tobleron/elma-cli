//! @efficiency-role: registry
//!
//! Tool metadata policy — defines discoverable metadata for tools and
//! workspace-level info for tool routing decisions.

use crate::*;

/// Category classification for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ToolCategory {
    FileSystem,
    Shell,
    Search,
    Network,
    Code,
    Session,
    UI,
    System,
}

impl ToolCategory {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            ToolCategory::FileSystem => "filesystem",
            ToolCategory::Shell => "shell",
            ToolCategory::Search => "search",
            ToolCategory::Network => "network",
            ToolCategory::Code => "code",
            ToolCategory::Session => "session",
            ToolCategory::UI => "ui",
            ToolCategory::System => "system",
        }
    }
}

/// A single parameter for a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolParam {
    pub(crate) name: String,
    pub(crate) param_type: String,
    pub(crate) required: bool,
    pub(crate) description: String,
}

/// Metadata describing a tool's identity, capabilities, and policies.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ToolMetadata {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) parameters: Vec<ToolParam>,
    pub(crate) requires_permission: bool,
    pub(crate) category: ToolCategory,
    pub(crate) timeout_seconds: u64,
}

/// Discoverable information about the current workspace for tool context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DiscoverableWorkspaceInfo {
    pub(crate) workspace_root: PathBuf,
    pub(crate) git_root: Option<PathBuf>,
    pub(crate) current_branch: Option<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) config_files: Vec<String>,
    pub(crate) has_cargo: bool,
    pub(crate) has_npm: bool,
    pub(crate) has_python: bool,
    pub(crate) top_level_dirs: Vec<String>,
    pub(crate) top_level_files: Vec<String>,
}

impl DiscoverableWorkspaceInfo {
    /// Collect workspace info from the given root path.
    ///
    /// Examines the filesystem to detect git status, programming languages,
    /// config files, and top-level directory structure.
    pub(crate) fn collect(workspace_root: &Path) -> Self {
        let git_root = find_git_root(workspace_root);
        let current_branch = git_root.as_ref().and_then(|r| get_current_branch(r));

        let languages = detect_languages(workspace_root);
        let config_files = detect_config_files(workspace_root);

        let has_cargo = workspace_root.join("Cargo.toml").exists();
        let has_npm = workspace_root.join("package.json").exists();
        let has_python = languages.contains(&"Python".to_string())
            || workspace_root.join("requirements.txt").exists()
            || workspace_root.join("setup.py").exists();

        let (top_level_dirs, top_level_files) = list_top_level(workspace_root);

        Self {
            workspace_root: workspace_root.to_path_buf(),
            git_root,
            current_branch,
            languages,
            config_files,
            has_cargo,
            has_npm,
            has_python,
            top_level_dirs,
            top_level_files,
        }
    }
}

fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Some(dir.to_path_buf());
        }
        current = dir.parent();
    }
    None
}

fn get_current_branch(git_root: &Path) -> Option<String> {
    let head_path = git_root.join(".git").join("HEAD");
    let head_content = std::fs::read_to_string(head_path).ok()?;
    if let Some(ref_line) = head_content.lines().next() {
        if let Some(branch) = ref_line.strip_prefix("ref: refs/heads/") {
            return Some(branch.to_string());
        }
    }
    None
}

fn detect_languages(root: &Path) -> Vec<String> {
    let mut langs = Vec::new();
    if root.join("Cargo.toml").exists() {
        langs.push("Rust".to_string());
    }
    if root.join("package.json").exists() {
        langs.push("JavaScript".to_string());
    }
    if root.join("pyproject.toml").exists()
        || root.join("requirements.txt").exists()
        || root.join("setup.py").exists()
    {
        langs.push("Python".to_string());
    }
    if root.join("go.mod").exists() {
        langs.push("Go".to_string());
    }
    langs
}

fn detect_config_files(root: &Path) -> Vec<String> {
    let common = [
        ".env",
        ".gitignore",
        ".editorconfig",
        ".dockerignore",
        "Cargo.toml",
        "package.json",
        "tsconfig.json",
        "Makefile",
        "Dockerfile",
        "docker-compose.yml",
        ".github",
        ".gitlab-ci.yml",
        "elma.toml",
    ];
    common
        .iter()
        .filter(|name| root.join(name).exists())
        .map(|s| s.to_string())
        .collect()
}

fn list_top_level(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut dirs = Vec::new();
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            if entry.file_type().map_or(false, |t| t.is_dir()) {
                dirs.push(name);
            } else {
                files.push(name);
            }
        }
    }
    dirs.sort();
    files.sort();
    (dirs, files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_tool_category_labels() {
        assert_eq!(ToolCategory::FileSystem.label(), "filesystem");
        assert_eq!(ToolCategory::Shell.label(), "shell");
        assert_eq!(ToolCategory::Search.label(), "search");
        assert_eq!(ToolCategory::Network.label(), "network");
        assert_eq!(ToolCategory::Code.label(), "code");
        assert_eq!(ToolCategory::Session.label(), "session");
        assert_eq!(ToolCategory::UI.label(), "ui");
        assert_eq!(ToolCategory::System.label(), "system");
    }

    #[test]
    fn test_tool_metadata_construction() {
        let meta = ToolMetadata {
            name: "read".to_string(),
            description: "Read file content".to_string(),
            parameters: vec![ToolParam {
                name: "path".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Path to file".to_string(),
            }],
            requires_permission: false,
            category: ToolCategory::FileSystem,
            timeout_seconds: 30,
        };
        assert_eq!(meta.name, "read");
        assert_eq!(meta.parameters.len(), 1);
        assert!(meta.parameters[0].required);
    }

    #[test]
    fn test_discoverable_info_empty_workspace() {
        let tmp = TempDir::new().unwrap();
        let info = DiscoverableWorkspaceInfo::collect(tmp.path());
        assert_eq!(info.workspace_root, tmp.path());
        assert!(info.languages.is_empty());
        assert!(info.config_files.is_empty());
        assert!(info.top_level_files.is_empty());
    }

    #[test]
    fn test_discoverable_info_with_cargo() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();

        let info = DiscoverableWorkspaceInfo::collect(tmp.path());
        assert!(info.has_cargo);
        assert!(info.languages.contains(&"Rust".to_string()));
        assert!(info.config_files.contains(&"Cargo.toml".to_string()));
        assert!(info.top_level_dirs.contains(&"src".to_string()));
    }

    #[test]
    fn test_discoverable_info_with_npm() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("package.json"), "{}").unwrap();

        let info = DiscoverableWorkspaceInfo::collect(tmp.path());
        assert!(info.has_npm);
        assert!(info.languages.contains(&"JavaScript".to_string()));
    }

    #[test]
    fn test_discoverable_info_python_detection() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "flask\n").unwrap();

        let info = DiscoverableWorkspaceInfo::collect(tmp.path());
        assert!(info.has_python);
        assert!(info.languages.contains(&"Python".to_string()));
    }

    #[test]
    fn test_discoverable_info_hidden_dirs_skipped() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".hidden")).unwrap();
        std::fs::create_dir_all(tmp.path().join("visible")).unwrap();

        let info = DiscoverableWorkspaceInfo::collect(tmp.path());
        assert!(!info.top_level_dirs.contains(&".hidden".to_string()));
        assert!(info.top_level_dirs.contains(&"visible".to_string()));
    }

    #[test]
    fn test_git_root_detection() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        let sub = tmp.path().join("subdir");
        std::fs::create_dir_all(&sub).unwrap();

        let root = find_git_root(&sub);
        assert_eq!(root, Some(tmp.path().to_path_buf()));
    }

    #[test]
    fn test_git_root_no_git() {
        let tmp = TempDir::new().unwrap();
        let root = find_git_root(tmp.path());
        assert!(root.is_none());
    }

    #[test]
    fn test_git_branch_detection() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(
            tmp.path().join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .unwrap();

        let branch = get_current_branch(tmp.path());
        assert_eq!(branch, Some("main".to_string()));
    }

    #[test]
    fn test_git_branch_detached_head() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join(".git")).unwrap();
        std::fs::write(tmp.path().join(".git").join("HEAD"), "abc123def\n").unwrap();

        let branch = get_current_branch(tmp.path());
        assert!(branch.is_none());
    }

    #[test]
    fn test_config_files_detection() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".gitignore"), "").unwrap();
        std::fs::write(tmp.path().join("Makefile"), "").unwrap();
        std::fs::write(tmp.path().join("elma.toml"), "").unwrap();

        let configs = detect_config_files(tmp.path());
        assert!(configs.contains(&".gitignore".to_string()));
        assert!(configs.contains(&"Makefile".to_string()));
        assert!(configs.contains(&"elma.toml".to_string()));
    }

    #[test]
    fn test_language_detection_go() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("go.mod"), "module test\n").unwrap();

        let langs = detect_languages(tmp.path());
        assert!(langs.contains(&"Go".to_string()));
    }

    #[test]
    fn test_top_level_listing() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("src")).unwrap();
        std::fs::create_dir_all(tmp.path().join("tests")).unwrap();
        std::fs::write(tmp.path().join("README.md"), "").unwrap();

        let (dirs, files) = list_top_level(tmp.path());
        assert!(dirs.contains(&"src".to_string()));
        assert!(dirs.contains(&"tests".to_string()));
        assert!(files.contains(&"README.md".to_string()));
    }

    #[test]
    fn test_tool_metadata_serde_roundtrip() {
        let meta = ToolMetadata {
            name: "bash".to_string(),
            description: "Execute shell command".to_string(),
            parameters: vec![ToolParam {
                name: "cmd".to_string(),
                param_type: "string".to_string(),
                required: true,
                description: "Command to run".to_string(),
            }],
            requires_permission: true,
            category: ToolCategory::Shell,
            timeout_seconds: 60,
        };

        let json = serde_json::to_string(&meta).unwrap();
        let back: ToolMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "bash");
        assert!(back.requires_permission);
        assert_eq!(back.category, ToolCategory::Shell);
    }
}
