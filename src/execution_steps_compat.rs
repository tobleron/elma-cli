//! @efficiency-role: util-pure
//!
//! Execution Steps - Command Compatibility and Probing

use crate::*;

#[derive(Debug, Clone)]
pub(crate) struct CommandCompatibilityFacts {
    pub(crate) primary_bin: String,
    pub(crate) command_exists: bool,
    pub(crate) command_lookup: String,
    pub(crate) os_family: String,
    pub(crate) shell_path: String,
}

fn primary_shell_command(cmd: &str) -> String {
    let mut rest = cmd.trim();
    while !rest.is_empty() {
        let token = rest.split_whitespace().next().unwrap_or("").trim();
        if token.is_empty() {
            break;
        }
        rest = rest[token.len()..].trim_start();
        let stripped = token.trim_matches(|c| c == '"' || c == '\'');
        if stripped.eq_ignore_ascii_case("env") {
            continue;
        }
        let looks_like_assignment = stripped.contains('=')
            && !stripped.starts_with('/')
            && stripped
                .chars()
                .next()
                .map(|c| c.is_ascii_alphabetic() || c == '_')
                .unwrap_or(false);
        if looks_like_assignment {
            continue;
        }
        return stripped.rsplit('/').next().unwrap_or(stripped).to_string();
    }
    String::new()
}

pub(crate) fn probe_command_compatibility(cmd: &str, workdir: &Path) -> CommandCompatibilityFacts {
    let primary_bin = primary_shell_command(cmd);
    let os_family = std::env::consts::OS.to_string();
    let shell_path = std::env::var("SHELL").unwrap_or_default();
    if primary_bin.is_empty() {
        return CommandCompatibilityFacts {
            primary_bin,
            command_exists: true,
            command_lookup: String::new(),
            os_family,
            shell_path,
        };
    }

    let probe = Command::new("sh")
        .current_dir(workdir)
        .arg("-lc")
        .arg(format!("command -v -- {}", shell_quote(&primary_bin)))
        .output();

    match probe {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            CommandCompatibilityFacts {
                primary_bin,
                command_exists: output.status.success(),
                command_lookup: if !stdout.is_empty() { stdout } else { stderr },
                os_family,
                shell_path,
            }
        }
        Err(error) => CommandCompatibilityFacts {
            primary_bin,
            command_exists: false,
            command_lookup: error.to_string(),
            os_family,
            shell_path,
        },
    }
}

pub(crate) fn shell_output_indicates_command_not_found(output: &str) -> bool {
    let lower = output.to_lowercase();
    lower.contains("command not found")
        || lower.contains("not recognized as an internal or external command")
}

pub(crate) fn command_is_unavailable(facts: &CommandCompatibilityFacts, output: &str) -> bool {
    (!facts.primary_bin.trim().is_empty() && !facts.command_exists)
        || shell_output_indicates_command_not_found(output)
}

pub(crate) fn unavailable_summary(facts: &CommandCompatibilityFacts, output: &str) -> String {
    let mut parts = vec![format!(
        "command_unavailable: {} on {}",
        if facts.primary_bin.trim().is_empty() {
            "unknown-command"
        } else {
            facts.primary_bin.trim()
        },
        facts.os_family.trim()
    )];
    if !facts.command_lookup.trim().is_empty() {
        parts.push(format!("lookup: {}", facts.command_lookup.trim()));
    }
    if !output.trim().is_empty() {
        parts.push(output.trim().to_string());
    }
    parts.join("\n")
}

pub(crate) fn unavailable_reply_instructions(facts: &CommandCompatibilityFacts) -> String {
    format!(
        "Explain briefly that the command `{}` is not available in this environment (os: {}, shell: {}). Do not retry the same command. If the user's underlying goal could be met with a platform-appropriate alternative, mention that and ask whether to use it.",
        if facts.primary_bin.trim().is_empty() {
            "the requested command"
        } else {
            facts.primary_bin.trim()
        },
        facts.os_family.trim(),
        if facts.shell_path.trim().is_empty() {
            "unknown"
        } else {
            facts.shell_path.trim()
        }
    )
}
