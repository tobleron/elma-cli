//! @efficiency-role: util-pure

use crate::*;

pub(crate) fn cmd_out(cmd: &str, cwd: &Path) -> String {
    let out = std::process::Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(cwd)
        .output();
    match out {
        Ok(o) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&o.stdout));
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            s.trim().to_string()
        }
        Err(_) => String::new(),
    }
}

fn tool_presence(repo_root: &Path, name: &str) -> &'static str {
    let cmd = format!("command -v {name} >/dev/null 2>&1");
    let status = std::process::Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(repo_root)
        .status();
    match status {
        Ok(s) if s.success() => "yes",
        _ => "no",
    }
}

pub(crate) fn gather_workspace_context(repo_root: &Path) -> String {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let os_uname = cmd_out("uname -a", repo_root);
    let sw_vers = cmd_out(
        "command -v sw_vers >/dev/null 2>&1 && sw_vers || true",
        repo_root,
    );
    let whoami = cmd_out("whoami", repo_root);
    let pwd = cmd_out("pwd", repo_root);
    let tty = cmd_out("tty || true", repo_root);
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let tool_names = ["rg", "git", "cargo", "python3", "node", "jq", "fd"];
    let tool_summary = tool_names
        .iter()
        .map(|name| format!("{name}:{}", tool_presence(repo_root, name)))
        .collect::<Vec<_>>()
        .join(", ");

    let mut s = String::new();
    s.push_str(&format!("repo_root: {}\n", repo_root.display()));
    s.push_str(&format!(
        "cwd: {}\n",
        if !pwd.is_empty() {
            pwd
        } else {
            repo_root.display().to_string()
        }
    ));
    if !user.is_empty() {
        s.push_str(&format!("user: {user}\n"));
    } else if !whoami.is_empty() {
        s.push_str(&format!("user: {whoami}\n"));
    }
    if !shell.is_empty() {
        s.push_str(&format!("shell: {shell}\n"));
    }
    if !shell_name.is_empty() {
        s.push_str(&format!("shell_name: {shell_name}\n"));
    }
    if !term.is_empty() {
        s.push_str(&format!("term: {term}\n"));
    }
    if !tty.is_empty() {
        s.push_str(&format!("tty: {tty}\n"));
    }
    if !sw_vers.is_empty() {
        s.push_str(&format!("os: {}\n", sw_vers.replace('\n', " | ")));
    } else if !os_uname.is_empty() {
        s.push_str(&format!("os: {os_uname}\n"));
    }
    s.push_str(&format!("tools: {tool_summary}\n"));
    s.trim().to_string()
}

pub(crate) fn gather_workspace_brief(repo_root: &Path) -> String {
    crate::workspace_tree::WorkspaceTree::new(repo_root)
        .with_max_depth(2)
        .with_max_entries(160)
        .build()
        .unwrap_or_else(|_| crate::workspace_tree::generate_workspace_brief(repo_root))
}
