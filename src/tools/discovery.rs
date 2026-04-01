//! Tool Discovery - Scans for available tools and capabilities

use crate::tools::cache::{CachedTool, ToolCache, compute_path_hash, get_cache_path, verify_tool_exists};
use std::path::{Path, PathBuf};
use std::process::Command;
use which::which;

#[derive(Debug, Clone)]
pub struct ToolCapability {
    pub name: String,
    pub description: String,
    pub command_template: String,
    pub availability: ToolAvailability,
    pub category: ToolCategory,
}

#[derive(Debug, Clone)]
pub enum ToolAvailability {
    AlwaysAvailable,
    ContextDependent(String),
    RequiresPermission,
}

#[derive(Debug, Clone)]
pub enum ToolCategory {
    CliTool,
    ProjectSpecific,
    CustomScript,
    Builtin,
}

pub async fn discover_available_tools(workspace: &Path) -> Vec<ToolCapability> {
    let mut tools = Vec::new();

    // Try to load from cache first
    let cache_path = get_cache_path();
    let path_hash = compute_path_hash();

    if let Ok(mut cache) = load_tool_cache(&cache_path, &path_hash) {
        // Use cached tools, verify they still exist
        for cached in &cache.tools {
            if verify_tool_exists(&cached.path) {
                tools.push(cached_tool_to_capability(cached));
            }
        }

        // Incremental update: scan for new tools in common directories
        let new_tools = scan_common_directories();
        for tool in &new_tools {
            if !cache.tools.iter().any(|t| t.name == tool.name) {
                cache.add_tool(tool.clone());
                tools.push(cached_tool_to_capability(tool));
            }
        }

        // Save updated cache
        let _ = cache.save(&cache_path);
    } else {
        // Full scan on first run or cache invalid
        tools = perform_full_scan(workspace);

        // Save to cache
        let mut cache = ToolCache::new();
        cache.path_hash = path_hash;
        for tool in &tools {
            cache.add_tool(tool_to_cached(tool));
        }
        let _ = cache.save(&cache_path);
    }

    tools
}

fn load_tool_cache(cache_path: &Path, path_hash: &str) -> Result<ToolCache, &'static str> {
    let cache = ToolCache::load(cache_path).ok_or("Cache not found")?;

    if !cache.is_valid(path_hash) {
        return Err("Cache invalid");
    }

    Ok(cache)
}

fn perform_full_scan(workspace: &Path) -> Vec<ToolCapability> {
    let mut tools = Vec::new();

    // Discover CLI tools using `which` crate
    tools.extend(discover_cli_tools());

    // Discover project-specific tools
    tools.extend(discover_project_tools(workspace));

    // Discover custom scripts
    tools.extend(discover_custom_scripts(workspace));

    tools
}

fn discover_cli_tools() -> Vec<ToolCapability> {
    let mut tools = Vec::new();

    // Common CLI tools to check - using `which` for fast lookup
    let cli_tools = vec![
        ("git", "Version control operations", "git <args>"),
        ("rg", "Fast text search with ripgrep", "rg <pattern> <path>"),
        ("grep", "Text search", "grep <pattern> <file>"),
        ("find", "Find files and directories", "find <path> <expression>"),
        ("jq", "JSON processing", "jq <filter> <json>"),
        ("curl", "HTTP requests", "curl <url>"),
        ("cat", "Display file contents", "cat <file>"),
        ("ls", "List directory contents", "ls <options> <path>"),
        ("cp", "Copy files", "cp <source> <destination>"),
        ("mv", "Move files", "mv <source> <destination>"),
        ("rm", "Remove files", "rm <options> <file>"),
        ("mkdir", "Create directories", "mkdir <path>"),
        ("touch", "Create empty files", "touch <file>"),
        ("head", "Show first lines of file", "head <file>"),
        ("tail", "Show last lines of file", "tail <file>"),
        ("wc", "Word count", "wc <file>"),
        ("sort", "Sort lines", "sort <file>"),
        ("uniq", "Filter duplicate lines", "uniq <file>"),
        ("sed", "Stream editor", "sed <expression> <file>"),
        ("awk", "Pattern scanning and processing", "awk <program> <file>"),
        // Additional tools
        ("python3", "Python 3 interpreter", "python3 <script>"),
        ("python", "Python interpreter", "python <script>"),
        ("node", "Node.js JavaScript runtime", "node <script>"),
        ("npm", "Node.js package manager", "npm <command>"),
        ("yarn", "Yarn package manager", "yarn <command>"),
        ("docker", "Container platform", "docker <command>"),
        ("ssh", "SSH client", "ssh <user@host>"),
        ("rsync", "Fast file copying", "rsync <options> <source> <dest>"),
        ("wget", "Network downloader", "wget <url>"),
        ("make", "Build automation", "make <target>"),
        ("cmake", "Cross-platform build", "cmake <options>"),
        ("cargo", "Rust package manager", "cargo <command>"),
        ("rustc", "Rust compiler", "rustc <file>"),
        ("go", "Go programming language", "go <command>"),
        ("java", "Java runtime", "java <options>"),
        ("javac", "Java compiler", "javac <file>"),
    ];

    for (name, description, template) in cli_tools {
        if which(name).is_ok() {
            tools.push(ToolCapability {
                name: name.to_string(),
                description: description.to_string(),
                command_template: template.to_string(),
                availability: ToolAvailability::AlwaysAvailable,
                category: ToolCategory::CliTool,
            });
        }
    }

    tools
}

fn scan_common_directories() -> Vec<CachedTool> {
    let mut tools = Vec::new();

    // Common binary directories to scan
    let bin_dirs = vec![
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
        "/opt/homebrew/bin",
        "/home/linuxbrew/.linuxbrew/bin",
    ];

    // Add user-specific dirs
    if let Ok(home) = std::env::var("HOME") {
        let user_dirs = vec![
            format!("{}/.cargo/bin", home),
            format!("{}/.npm-global/bin", home),
            format!("{}/.local/bin", home),
            format!("{}/.yarn/bin", home),
        ];
        for dir in user_dirs {
            scan_directory(&PathBuf::from(dir), &mut tools, "cli");
        }
    }

    // Scan standard dirs (limit to avoid too many tools)
    let mut count = 0;
    for dir_str in bin_dirs {
        if count > 100 {
            break; // Limit total tools
        }
        let dir = PathBuf::from(dir_str);
        if dir.exists() {
            count += scan_directory(&dir, &mut tools, "cli");
        }
    }

    tools
}

fn scan_directory(dir: &Path, tools: &mut Vec<CachedTool>, category: &str) -> usize {
    let mut count = 0;

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && is_executable(&path) {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip common non-tool files
                    if should_skip_tool(name) {
                        continue;
                    }

                    tools.push(CachedTool {
                        name: name.to_string(),
                        path: path.to_string_lossy().to_string(),
                        category: category.to_string(),
                        description: format!("System command: {}", name),
                    });
                    count += 1;
                }
            }
        }
    }

    count
}

fn is_executable(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        path.metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(windows)]
    {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|e| ["exe", "bat", "cmd", "ps1"].contains(&e.to_lowercase().as_str()))
            .unwrap_or(false)
    }
}

fn should_skip_tool(name: &str) -> bool {
    let skip = [
        ".", "..", ".DS_Store", "Thumbs.db",
        "python3-config", "python-config",
        "nodejs", "npm.cmd", "yarn.cmd",
        "cargo-clippy", "cargo-fmt", "cargo-miri",
    ];
    skip.contains(&name) || name.ends_with(".d")
}

fn discover_project_tools(workspace: &Path) -> Vec<ToolCapability> {
    let mut tools = Vec::new();

    // Rust project
    if workspace.join("Cargo.toml").exists() && which("cargo").is_ok() {
        tools.push(ToolCapability {
            name: "cargo".to_string(),
            description: "Rust package manager - build, test, run".to_string(),
            command_template: "cargo <command>".to_string(),
            availability: ToolAvailability::ContextDependent("Rust project detected".to_string()),
            category: ToolCategory::ProjectSpecific,
        });
    }

    // Node.js project
    if workspace.join("package.json").exists() {
        if which("npm").is_ok() {
            tools.push(ToolCapability {
                name: "npm".to_string(),
                description: "Node.js package manager".to_string(),
                command_template: "npm <command>".to_string(),
                availability: ToolAvailability::ContextDependent("Node.js project detected".to_string()),
                category: ToolCategory::ProjectSpecific,
            });
        }
        if which("yarn").is_ok() {
            tools.push(ToolCapability {
                name: "yarn".to_string(),
                description: "Fast Node.js package manager".to_string(),
                command_template: "yarn <command>".to_string(),
                availability: ToolAvailability::ContextDependent("Node.js project detected".to_string()),
                category: ToolCategory::ProjectSpecific,
            });
        }
        if which("pnpm").is_ok() {
            tools.push(ToolCapability {
                name: "pnpm".to_string(),
                description: "Fast, disk space efficient package manager".to_string(),
                command_template: "pnpm <command>".to_string(),
                availability: ToolAvailability::ContextDependent("Node.js project detected".to_string()),
                category: ToolCategory::ProjectSpecific,
            });
        }
    }

    // Python project
    if workspace.join("requirements.txt").exists()
        || workspace.join("setup.py").exists()
        || workspace.join("pyproject.toml").exists()
    {
        if which("pip").is_ok() || which("pip3").is_ok() {
            tools.push(ToolCapability {
                name: "pip".to_string(),
                description: "Python package installer".to_string(),
                command_template: "pip <command>".to_string(),
                availability: ToolAvailability::ContextDependent("Python project detected".to_string()),
                category: ToolCategory::ProjectSpecific,
            });
        }
        if which("python").is_ok() || which("python3").is_ok() {
            tools.push(ToolCapability {
                name: "python".to_string(),
                description: "Python interpreter".to_string(),
                command_template: "python <script>".to_string(),
                availability: ToolAvailability::ContextDependent("Python project detected".to_string()),
                category: ToolCategory::ProjectSpecific,
            });
        }
    }

    // Go project
    if workspace.join("go.mod").exists() && which("go").is_ok() {
        tools.push(ToolCapability {
            name: "go".to_string(),
            description: "Go programming language tools".to_string(),
            command_template: "go <command>".to_string(),
            availability: ToolAvailability::ContextDependent("Go project detected".to_string()),
            category: ToolCategory::ProjectSpecific,
        });
    }

    tools
}

fn discover_custom_scripts(workspace: &Path) -> Vec<ToolCapability> {
    let mut tools = Vec::new();

    // Common script directories
    let script_dirs = vec![
        workspace.join("scripts"),
        workspace.join("bin"),
        workspace.join(".scripts"),
    ];

    for script_dir in script_dirs {
        if script_dir.exists() && script_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&script_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && is_executable_script(&path) {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            let rel_path = path
                                .strip_prefix(workspace)
                                .unwrap_or(&path)
                                .to_string_lossy();

                            tools.push(ToolCapability {
                                name: name.to_string(),
                                description: format!("Custom script: {}", rel_path),
                                command_template: format!("./{}", rel_path),
                                availability: ToolAvailability::AlwaysAvailable,
                                category: ToolCategory::CustomScript,
                            });
                        }
                    }
                }
            }
        }
    }

    // Check for Makefile
    if workspace.join("Makefile").exists() && which("make").is_ok() {
        tools.push(ToolCapability {
            name: "make".to_string(),
            description: "Build automation from Makefile".to_string(),
            command_template: "make <target>".to_string(),
            availability: ToolAvailability::ContextDependent("Makefile found".to_string()),
            category: ToolCategory::ProjectSpecific,
        });
    }

    tools
}

fn is_executable_script(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "sh" | "bash" | "py" | "pl" | "rb" | "js" | "ts")
        || path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| !n.contains('.'))
            .unwrap_or(false)
}

fn cached_tool_to_capability(cached: &CachedTool) -> ToolCapability {
    ToolCapability {
        name: cached.name.clone(),
        description: cached.description.clone(),
        command_template: format!("{} <args>", cached.name),
        availability: ToolAvailability::AlwaysAvailable,
        category: match cached.category.as_str() {
            "project" => ToolCategory::ProjectSpecific,
            "script" => ToolCategory::CustomScript,
            _ => ToolCategory::CliTool,
        },
    }
}

fn tool_to_cached(tool: &ToolCapability) -> CachedTool {
    CachedTool {
        name: tool.name.clone(),
        path: which(&tool.name)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| tool.name.clone()),
        category: match tool.category {
            ToolCategory::CliTool => "cli".to_string(),
            ToolCategory::ProjectSpecific => "project".to_string(),
            ToolCategory::CustomScript => "script".to_string(),
            ToolCategory::Builtin => "builtin".to_string(),
        },
        description: tool.description.clone(),
    }
}
