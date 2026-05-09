use std::path::{PathBuf};
use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub fn exec_workspace_info(
    workdir: &PathBuf,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    emit_tool_start(&mut tui, "workspace_info", "");
    let mut info = String::new();

    info.push_str(&format!("## Workspace Root\n{}\n\n", workdir.display()));

    info.push_str("## Directory Structure\n");
    if let Ok(entries) = std::fs::read_dir(workdir) {
        let mut items: Vec<String> = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.starts_with('.')
                || name == "target"
                || name == "node_modules"
                || name == "dist"
                || name == "build"
            {
                continue;
            }
            let marker = if path.is_dir() { "/" } else { "" };
            if path.is_dir() {
                let mut sub_items = String::new();
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    let mut subs: Vec<String> = sub_entries
                        .flatten()
                        .filter_map(|e| {
                            let sp = e.path();
                            let sn = sp.file_name()?.to_string_lossy().to_string();
                            if sn.starts_with('.') {
                                return None;
                            }
                            let sm = if sp.is_dir() { "/" } else { "" };
                            Some(format!("    {}{}", sn, sm))
                        })
                        .take(20)
                        .collect();
                    subs.sort();
                    if !subs.is_empty() {
                        sub_items = format!("\n{}", subs.join("\n"));
                    }
                }
                items.push(format!("  {}{}{}", name, marker, sub_items));
            } else {
                items.push(format!("  {}{}", name, marker));
            }
            if items.len() >= 100 {
                break;
            }
        }
        items.sort();
        info.push_str(&items.join("\n"));
    }
    info.push_str("\n\n");

    info.push_str("## Project Type\n");
    let checks: &[(&str, &str)] = &[
        ("Cargo.toml", "Rust"),
        ("package.json", "Node.js/JavaScript/TypeScript"),
        ("pyproject.toml", "Python"),
        ("setup.py", "Python"),
        ("go.mod", "Go"),
        ("Makefile", "Make-based project"),
        ("CMakeLists.txt", "CMake/C++"),
        ("Gemfile", "Ruby"),
        ("composer.json", "PHP"),
        ("pom.xml", "Java/Maven"),
        ("build.gradle", "Java/Gradle"),
        ("requirements.txt", "Python"),
        ("Dockerfile", "Docker container"),
        ("docker-compose.yml", "Docker Compose"),
        (".github/workflows", "GitHub Actions CI"),
    ];

    let mut found = false;
    for (file, label) in checks {
        if workdir.join(file).exists() {
            info.push_str(&format!("- {} ({})\n", label, file));
            found = true;
        }
    }
    if !found {
        info.push_str("- Generic (no recognized project file)\n");
    }

    if workdir.join(".git").exists() {
        info.push_str("\n## Git Status\n");
        let branch = std::process::Command::new("git")
            .args([
                "-C",
                &workdir.display().to_string(),
                "branch",
                "--show-current",
            ])
            .output();
        if let Ok(out) = branch {
            let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !b.is_empty() {
                info.push_str(&format!("Branch: {}\n", b));
            }
        }
        let status = std::process::Command::new("git")
            .args(["-C", &workdir.display().to_string(), "status", "--short"])
            .output();
        if let Ok(out) = status {
            let text = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = text.lines().collect();
            if lines.is_empty() {
                info.push_str("Working tree clean\n");
            } else {
                let modified = lines
                    .iter()
                    .filter(|l| l.starts_with(" M") || l.starts_with("M "))
                    .count();
                let untracked = lines.iter().filter(|l| l.starts_with("??")).count();
                let staged = lines
                    .iter()
                    .filter(|l| l.starts_with("M ") || l.starts_with("A "))
                    .count();
                info.push_str(&format!(
                    "{} staged, {} modified, {} untracked files\n",
                    staged, modified, untracked
                ));
                info.push_str("Recent changes:\n");
                for line in lines.iter().take(20) {
                    info.push_str(&format!("  {}\n", line));
                }
                if lines.len() > 20 {
                    info.push_str(&format!("  ... and {} more\n", lines.len() - 20));
                }
            }
        }
    }

    let guidance_files = [
        ("AGENTS.md", 1600usize),
        ("_tasks/_tasks.md", 1200),
        ("_tasks/_guidelines.md", 1200),
    ];

    let mut guidance_section = String::new();
    for (rel_path, max_chars) in &guidance_files {
        let full_path = workdir.join(rel_path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            let mut chars = content.chars();
            let trimmed: String = chars.by_ref().take(*max_chars).collect();
            guidance_section.push_str(&format!("\n### {}\n```\n{}\n```\n", rel_path, trimmed));
            if chars.next().is_some() {
                guidance_section.push_str("...(truncated)\n");
            }
        }
    }
    let active_dir = workdir.join("_tasks").join("active");
    if active_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&active_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && path.extension().map(|e| e == "md").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        let trimmed: String = content.chars().take(800).collect();
                        let name = path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        guidance_section.push_str(&format!(
                            "\n### Active task: {}\n```\n{}\n```\n",
                            name, trimmed
                        ));
                    }
                    break;
                }
            }
        }
    }
    if !guidance_section.is_empty() {
        info.push_str("\n## Project Guidance\n");
        info.push_str(&guidance_section);
    }

    emit_tool_result(&mut tui, "workspace_info", true, &info);
    ToolExecutionResult::new_ok(call_id, "workspace_info", &info)
}
