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

pub(crate) fn gather_workspace_context(repo_root: &Path) -> String {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let os_uname = cmd_out("uname -a", repo_root);
    let sw_vers = cmd_out(
        "command -v sw_vers >/dev/null 2>&1 && sw_vers || true",
        repo_root,
    );
    let whoami = cmd_out("whoami", repo_root);
    let pwd = cmd_out("pwd", repo_root);
    let git_branch = cmd_out("git branch --show-current 2>/dev/null || true", repo_root);
    let git_status = cmd_out(
        "git status --short 2>/dev/null | head -5 || true",
        repo_root,
    );

    // Detect platform explicitly for model context
    let platform = if cfg!(target_os = "macos") {
        "macOS (darwin)".to_string()
    } else if cfg!(target_os = "linux") {
        "Linux".to_string()
    } else {
        "unknown".to_string()
    };

    let mut s = String::new();
    s.push_str(&format!("platform: {platform}\n"));
    if cfg!(target_os = "macos") {
        s.push_str("NOTE: This is macOS. Use BSD-style commands: `stat -f` not `stat --format`, `find` has no `-printf`, use `| while read f; do stat -f \"%m %N\" \"$f\"; done` for file dates.\n");
    }
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
    if !sw_vers.is_empty() {
        s.push_str(&format!("os: {}\n", sw_vers.replace('\n', " | ")));
    } else if !os_uname.is_empty() {
        s.push_str(&format!("os: {os_uname}\n"));
    }
    if !git_branch.is_empty() {
        s.push_str(&format!("git_branch: {git_branch}\n"));
    }
    if !git_status.is_empty() {
        s.push_str(&format!("git_status: {git_status}\n"));
    }
    s.trim().to_string()
}

pub(crate) fn gather_workspace_brief(repo_root: &Path) -> String {
    crate::workspace_tree::WorkspaceTree::new(repo_root)
        .with_max_depth(2)
        .with_max_entries(160)
        .build()
        .unwrap_or_else(|_| crate::workspace_tree::generate_workspace_brief(repo_root))
}
