//! @efficiency-role: domain-logic
//!
//! Shell Execution Policy (Task 658)
//!
//! Parser-backed shell execution policy and permission cache.
//! Classifies commands by danger class, applies sanitization
//! to remove dangerous patterns, and caches permission decisions.

use crate::*;
use std::collections::HashMap;

/// Classification of a shell command based on its first token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellCommandClass {
    Read,
    Write,
    Destructive,
    Network,
    PackageManager,
    Admin,
    Unknown,
}

/// Parser-backed shell execution policy.
pub(crate) struct ShellExecPolicy;

impl ShellExecPolicy {
    /// Classify a command by parsing its first token.
    pub(crate) fn classify(command: &str) -> ShellCommandClass {
        let cmd = command.trim();
        if cmd.is_empty() {
            return ShellCommandClass::Unknown;
        }
        let first = cmd.split_whitespace().next().unwrap_or("");
        let base = first.rsplit('/').next().unwrap_or(first);

        match base {
            "ls" | "cat" | "head" | "tail" | "less" | "more" | "pwd" | "echo" | "printf"
            | "which" | "type" | "command" | "env" | "printenv" | "date" | "cal" | "df" | "du"
            | "stat" | "file" | "wc" | "sort" | "uniq" | "cut" | "tr" | "grep" | "rg" | "egrep"
            | "fgrep" | "find" | "locate" | "tree" | "basename" | "dirname" | "realpath"
            | "readlink" | "time" | "uname" | "whoami" | "id" | "hostname" | "uptime" | "ps"
            | "top" | "htop" | "history" | "help" | "man" | "whatis" | "apropos" | "diff"
            | "comm" | "cmp" | "sha1sum" | "sha256sum" | "md5sum" | "xxd" | "od" | "hexdump"
            | "strings" | "jq" | "yq" | "git" => ShellCommandClass::Read,

            "mkdir" | "touch" | "cp" | "mv" | "chmod" | "chown" | "chgrp" | "ln" | "install"
            | "mkfifo" | "mknod" | "link" | "unlink" | "tee" => ShellCommandClass::Write,

            "rm" | "rmdir" | "dd" | "shred" | "truncate" | "fallocate" | "wipefs" | "mkfs"
            | "fdisk" | "parted" | "gdisk" | "mkswap" => ShellCommandClass::Destructive,

            "curl" | "wget" | "fetch" | "ssh" | "scp" | "sftp" | "rsync" | "nc" | "netcat"
            | "ncat" | "telnet" | "ftp" | "tftp" | "ping" | "ping6" | "traceroute"
            | "traceroute6" | "tracepath" | "nslookup" | "dig" | "host" | "whois" | "nmap" => {
                ShellCommandClass::Network
            }

            "apt" | "apt-get" | "apt-cache" | "dpkg" | "dpkg-reconfigure" | "brew" | "port"
            | "npm" | "yarn" | "pnpm" | "bun" | "pip" | "pip2" | "pip3" | "cargo" | "gem"
            | "snap" | "flatpak" | "pacman" | "yum" | "dnf" | "zypper" | "rpm" | "apk"
            | "emerge" | "pkgin" | "pkg_add" | "pkg" => ShellCommandClass::PackageManager,

            "sudo" | "su" | "doas" | "runuser" | "chroot" | "shutdown" | "reboot" | "halt"
            | "poweroff" | "init" | "systemctl" | "service" | "rc-service" | "rcctl"
            | "launchctl" => ShellCommandClass::Admin,

            _ => ShellCommandClass::Unknown,
        }
    }

    /// Check if a command requires explicit user permission.
    pub(crate) fn requires_permission(command: &str) -> bool {
        let class = Self::classify(command);
        matches!(
            class,
            ShellCommandClass::Destructive | ShellCommandClass::Admin
        )
    }

    /// Sanitize a command by removing dangerous patterns.
    /// Strips pipes to rm, dangerous redirects, and destructive chaining.
    pub(crate) fn sanitize(command: &str) -> String {
        let cmd = command.trim();

        let mut result = cmd.to_string();

        let dangerous_patterns = [
            "| xargs rm",
            "| xargs rm -rf",
            "| xargs rm -fr",
            "| xargs shred",
            "| xargs truncate",
            "| while read",
            "|while read",
        ];

        for pattern in &dangerous_patterns {
            if let Some(pos) = result.find(pattern) {
                result.truncate(pos);
                result = result.trim_end().to_string();
                break;
            }
        }

        let redirect_patterns = [
            "> /dev/null",
            "> /dev/null 2>&1",
            ">& /dev/null",
            "2>&1",
            "2>/dev/null",
            "> /dev/null 2>/dev/null",
        ];

        for pattern in &redirect_patterns {
            while let Some(pos) = result.find(pattern) {
                result.replace_range(pos..pos + pattern.len(), "");
            }
        }

        while let Some(pos) = result.find(">> ") {
            let before = result[..pos].trim_end().to_string();
            result = before;
        }
        while let Some(pos) = result.find("> ") {
            let before = result[..pos].trim_end().to_string();
            result = before;
        }

        while let Some(pos) = result.find("&& rm") {
            result.replace_range(pos..pos + 5, "&& ");
        }
        while let Some(pos) = result.find("; rm") {
            result.replace_range(pos..pos + 4, "; ");
        }

        result.trim().to_string()
    }
}

/// Session-scoped permission cache for shell commands.
pub(crate) struct PermissionCache {
    pub(crate) cache: HashMap<String, bool>,
}

impl PermissionCache {
    pub(crate) fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Look up a cached permission decision for a command.
    /// Returns `None` if no decision has been cached.
    pub(crate) fn check(&mut self, command: &str) -> Option<bool> {
        self.cache.get(command).copied()
    }

    /// Cache a permission decision for a command.
    pub(crate) fn set(&mut self, command: &str, allowed: bool) {
        self.cache.insert(command.to_string(), allowed);
    }

    /// Clear all cached permission decisions.
    pub(crate) fn clear(&mut self) {
        self.cache.clear();
    }

    /// Number of cached entries.
    pub(crate) fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the cache is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for PermissionCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ShellCommandClass tests ──

    #[test]
    fn test_classify_read_commands() {
        assert_eq!(ShellExecPolicy::classify("ls -la"), ShellCommandClass::Read);
        assert_eq!(
            ShellExecPolicy::classify("cat file.txt"),
            ShellCommandClass::Read
        );
        assert_eq!(
            ShellExecPolicy::classify("head -20 data.csv"),
            ShellCommandClass::Read
        );
        assert_eq!(
            ShellExecPolicy::classify("tail -f log.txt"),
            ShellCommandClass::Read
        );
        assert_eq!(ShellExecPolicy::classify("pwd"), ShellCommandClass::Read);
        assert_eq!(
            ShellExecPolicy::classify("echo hello"),
            ShellCommandClass::Read
        );
        assert_eq!(
            ShellExecPolicy::classify("grep pattern file"),
            ShellCommandClass::Read
        );
        assert_eq!(
            ShellExecPolicy::classify("rg 'foo' src/"),
            ShellCommandClass::Read
        );
        assert_eq!(
            ShellExecPolicy::classify("find . -name '*.rs'"),
            ShellCommandClass::Read
        );
        assert_eq!(ShellExecPolicy::classify("date"), ShellCommandClass::Read);
        assert_eq!(ShellExecPolicy::classify("whoami"), ShellCommandClass::Read);
        assert_eq!(
            ShellExecPolicy::classify("git status"),
            ShellCommandClass::Read
        );
    }

    #[test]
    fn test_classify_write_commands() {
        assert_eq!(
            ShellExecPolicy::classify("mkdir build"),
            ShellCommandClass::Write
        );
        assert_eq!(
            ShellExecPolicy::classify("touch main.rs"),
            ShellCommandClass::Write
        );
        assert_eq!(
            ShellExecPolicy::classify("cp src/a dest/b"),
            ShellCommandClass::Write
        );
        assert_eq!(
            ShellExecPolicy::classify("mv old new"),
            ShellCommandClass::Write
        );
        assert_eq!(
            ShellExecPolicy::classify("chmod +x script.sh"),
            ShellCommandClass::Write
        );
        assert_eq!(
            ShellExecPolicy::classify("ln -s target link"),
            ShellCommandClass::Write
        );
    }

    #[test]
    fn test_classify_destructive_commands() {
        assert_eq!(
            ShellExecPolicy::classify("rm file.txt"),
            ShellCommandClass::Destructive
        );
        assert_eq!(
            ShellExecPolicy::classify("rm -rf /tmp/build"),
            ShellCommandClass::Destructive
        );
        assert_eq!(
            ShellExecPolicy::classify("rmdir emptydir"),
            ShellCommandClass::Destructive
        );
        assert_eq!(
            ShellExecPolicy::classify("dd if=/dev/zero of=/tmp/out bs=1M count=1"),
            ShellCommandClass::Destructive
        );
        assert_eq!(
            ShellExecPolicy::classify("shred secret.txt"),
            ShellCommandClass::Destructive
        );
        assert_eq!(
            ShellExecPolicy::classify("truncate -s 0 log.txt"),
            ShellCommandClass::Destructive
        );
    }

    #[test]
    fn test_classify_network_commands() {
        assert_eq!(
            ShellExecPolicy::classify("curl https://example.com"),
            ShellCommandClass::Network
        );
        assert_eq!(
            ShellExecPolicy::classify("wget https://example.com/file"),
            ShellCommandClass::Network
        );
        assert_eq!(
            ShellExecPolicy::classify("ssh user@host"),
            ShellCommandClass::Network
        );
        assert_eq!(
            ShellExecPolicy::classify("ping 8.8.8.8"),
            ShellCommandClass::Network
        );
        assert_eq!(
            ShellExecPolicy::classify("nslookup example.com"),
            ShellCommandClass::Network
        );
        assert_eq!(
            ShellExecPolicy::classify("dig +short A example.com"),
            ShellCommandClass::Network
        );
    }

    #[test]
    fn test_classify_package_manager_commands() {
        assert_eq!(
            ShellExecPolicy::classify("apt install curl"),
            ShellCommandClass::PackageManager
        );
        assert_eq!(
            ShellExecPolicy::classify("brew install node"),
            ShellCommandClass::PackageManager
        );
        assert_eq!(
            ShellExecPolicy::classify("npm install"),
            ShellCommandClass::PackageManager
        );
        assert_eq!(
            ShellExecPolicy::classify("pip install requests"),
            ShellCommandClass::PackageManager
        );
        assert_eq!(
            ShellExecPolicy::classify("cargo build"),
            ShellCommandClass::PackageManager
        );
        assert_eq!(
            ShellExecPolicy::classify("gem install rails"),
            ShellCommandClass::PackageManager
        );
    }

    #[test]
    fn test_classify_admin_commands() {
        assert_eq!(
            ShellExecPolicy::classify("sudo rm /etc/hosts"),
            ShellCommandClass::Admin
        );
        assert_eq!(
            ShellExecPolicy::classify("su - root"),
            ShellCommandClass::Admin
        );
        assert_eq!(
            ShellExecPolicy::classify("shutdown -h now"),
            ShellCommandClass::Admin
        );
        assert_eq!(
            ShellExecPolicy::classify("systemctl restart nginx"),
            ShellCommandClass::Admin
        );
        assert_eq!(
            ShellExecPolicy::classify("reboot"),
            ShellCommandClass::Admin
        );
    }

    #[test]
    fn test_classify_unknown() {
        assert_eq!(ShellExecPolicy::classify(""), ShellCommandClass::Unknown);
        assert_eq!(ShellExecPolicy::classify("   "), ShellCommandClass::Unknown);
        assert_eq!(
            ShellExecPolicy::classify("mycustomtool do-thing"),
            ShellCommandClass::Unknown
        );
        assert_eq!(
            ShellExecPolicy::classify("./deploy.sh"),
            ShellCommandClass::Unknown
        );
    }

    #[test]
    fn test_classify_with_full_path() {
        assert_eq!(
            ShellExecPolicy::classify("/bin/ls -la"),
            ShellCommandClass::Read
        );
        assert_eq!(
            ShellExecPolicy::classify("/usr/bin/curl https://x.com"),
            ShellCommandClass::Network
        );
        assert_eq!(
            ShellExecPolicy::classify("/usr/local/bin/brew install"),
            ShellCommandClass::PackageManager
        );
    }

    // ── requires_permission tests ──

    #[test]
    fn test_requires_permission_true() {
        assert!(ShellExecPolicy::requires_permission("rm file.txt"));
        assert!(ShellExecPolicy::requires_permission("sudo apt update"));
        assert!(ShellExecPolicy::requires_permission(
            "dd if=/dev/zero of=/tmp/out"
        ));
        assert!(ShellExecPolicy::requires_permission("shutdown -h now"));
    }

    #[test]
    fn test_requires_permission_false() {
        assert!(!ShellExecPolicy::requires_permission("ls -la"));
        assert!(!ShellExecPolicy::requires_permission("cat file.txt"));
        assert!(!ShellExecPolicy::requires_permission("curl https://x.com"));
        assert!(!ShellExecPolicy::requires_permission("npm install"));
        assert!(!ShellExecPolicy::requires_permission("mv a b"));
    }

    // ── sanitize tests ──

    #[test]
    fn test_sanitize_strips_xargs_rm() {
        let result = ShellExecPolicy::sanitize("find . -type f | xargs rm");
        assert_eq!(result, "find . -type f");
    }

    #[test]
    fn test_sanitize_strips_while_read() {
        let result =
            ShellExecPolicy::sanitize("find . -name '*.log' | while read f; do rm \"$f\"; done");
        assert_eq!(result, "find . -name '*.log'");
    }

    #[test]
    fn test_sanitize_strips_redirects() {
        let result = ShellExecPolicy::sanitize("echo hello > /dev/null");
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_sanitize_strips_output_redirect() {
        let result = ShellExecPolicy::sanitize("ls -la > output.txt");
        assert_eq!(result, "ls -la");
    }

    #[test]
    fn test_sanitize_handles_append_redirect() {
        let result = ShellExecPolicy::sanitize("echo log >> log.txt");
        assert_eq!(result, "echo log");
    }

    #[test]
    fn test_sanitize_strips_chained_rm() {
        let result = ShellExecPolicy::sanitize("test -f file && rm file");
        assert!(
            !result.contains("rm"),
            "sanitize should remove rm: got {result}"
        );
        assert!(result.contains("test -f file"), "should preserve prefix");
    }

    #[test]
    fn test_sanitize_preserves_safe_commands() {
        let result = ShellExecPolicy::sanitize("ls -la");
        assert_eq!(result, "ls -la");
    }

    #[test]
    fn test_sanitize_preserves_pipe_to_grep() {
        let result = ShellExecPolicy::sanitize("ps aux | grep python");
        assert_eq!(result, "ps aux | grep python");
    }

    #[test]
    fn test_sanitize_empty_command() {
        let result = ShellExecPolicy::sanitize("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_whitespace_command() {
        let result = ShellExecPolicy::sanitize("   ");
        assert_eq!(result, "");
    }

    // ── PermissionCache tests ──

    #[test]
    fn test_permission_cache_new_is_empty() {
        let cache = PermissionCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_permission_cache_set_and_check() {
        let mut cache = PermissionCache::new();
        cache.set("rm -rf /tmp/build", true);
        cache.set("sudo shutdown", false);

        assert_eq!(cache.check("rm -rf /tmp/build"), Some(true));
        assert_eq!(cache.check("sudo shutdown"), Some(false));
    }

    #[test]
    fn test_permission_cache_check_missing() {
        let mut cache = PermissionCache::new();
        assert_eq!(cache.check("rm file.txt"), None);
    }

    #[test]
    fn test_permission_cache_overwrite() {
        let mut cache = PermissionCache::new();
        cache.set("rm file.txt", true);
        cache.set("rm file.txt", false);
        assert_eq!(cache.check("rm file.txt"), Some(false));
    }

    #[test]
    fn test_permission_cache_clear() {
        let mut cache = PermissionCache::new();
        cache.set("rm file.txt", true);
        cache.set("sudo apt update", false);
        assert!(!cache.is_empty());

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_permission_cache_exact_match() {
        let mut cache = PermissionCache::new();
        cache.set("rm file.txt", true);
        assert_eq!(cache.check("rm file.txt"), Some(true));
        assert_eq!(cache.check("rm other.txt"), None);
    }

    #[test]
    fn test_permission_cache_default() {
        let cache = PermissionCache::default();
        assert!(cache.is_empty());
    }
}
