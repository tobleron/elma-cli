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

    let mut s = String::new();
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
    s.trim().to_string()
}

pub(crate) fn gather_workspace_brief(repo_root: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Ok(rd) = std::fs::read_dir(repo_root) {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n != "target" && n != "sessions" && !n.starts_with(".git"))
            .collect();
        names.sort();
        parts.push(format!(
            "top_level: {}",
            names.into_iter().take(24).collect::<Vec<_>>().join(", ")
        ));
    }

    let cargo = repo_root.join("Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(&cargo) {
        let excerpt = text.lines().take(24).collect::<Vec<_>>().join("\n");
        parts.push(format!("Cargo.toml:\n{excerpt}"));
    }

    let src_dir = repo_root.join("src");
    if let Ok(rd) = std::fs::read_dir(&src_dir) {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        parts.push(format!("src_files: {}", names.join(", ")));
    }

    parts.join("\n\n")
}
