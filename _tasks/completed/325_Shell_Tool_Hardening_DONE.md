# Task 325: Shell Tool Hardening

**Status:** pending  
**Depends on:** None (enhancements to existing `shell` tool and `shell_preflight.rs`)

## Summary

Harden the existing `shell` tool with ban lists, safe-command classification, timeout enforcement, output truncation, destructive-command gating, and persistent-shell reliability — drawing from Opencode, Codex CLI, and Claude Code's battle-tested shell implementations.

## Why

The shell tool is the most powerful and dangerous tool. The `s_1777380479_751323000` session shows the model repeatedly running redundant `ls -la` and `find` commands (stagnation). Opencode enforces ban lists and timeouts. Codex CLI has sandbox escalation. Claude Code has extensive permissions. Elma needs these safeguards.

## Reference Implementations

### Opencode (`_knowledge_base/_source_code_agents/opencode/internal/llm/tools/bash.go`)
```go
// Banned commands: alias, curl, wget, nc, telnet, lynx, httpie, chrome, firefox, safari
// Safe read-only: 40+ commands (git status/log/diff/show, go version/list/env/doc, etc.)
// Timeout: default 1min, max 10min (in ms)
// Max output: 30,000 chars
// Persistent shell session (state persists between calls)
```

### Codex CLI (`_knowledge_base/_source_code_agents/codex-cli/codex-rs/core/src/tools/handlers/unified_exec.rs`)
```rust
// is_known_safe_command() — classifies commands as safe vs mutating
// Sandbox escalation: attempt → deny → retry with escalated permissions
// Token-budget-aware truncation: effective_max_output_tokens()
// TTY metrics
// Command display via shlex_join for safe rendering
```

### Claude Code (`_knowledge_base/_source_code_agents/claude-code/tools/BashTool/`)
```typescript
// Massive permission system (~1663 lines in bashPermissions.ts)
// HasPermission() checks
// checkCommandOperatorPermissions() for dangerous operators
// Environment sandboxing
```

## Implementation Steps

### Step 1: Ban List (Opencode pattern)

Define a comprehensive ban list for commands that Elma should never run:

```rust
const BANNED_COMMANDS: &[&str] = &[
    // Network clients (Elma is offline-first; use dedicated tools for network)
    "curl", "wget", "aria2c", "axel", "http-prompt", "httpie",
    "links", "lynx", "w3m", "xh",
    // Browsers
    "chrome", "chromium", "firefox", "safari", "edge", "opera",
    // Network tools (dangerous in agent context)
    "nc", "ncat", "netcat", "telnet", "ssh", "scp", "sftp",
    // Privilege escalation
    "sudo", "doas", "su",
    // Package managers (could install malicious software)
    "apt", "apt-get", "apt-cache", "dpkg", "dnf", "yum", "zypper",
    "pacman", "yay", "paru", "pkg", "pkg_add", "pkg_delete",
    "apk", "emerge", "portage", "rpm", "home-manager",
    "opkg", "makepkg",
    // System control (could break the machine)
    "systemctl", "service", "chkconfig", "crontab",
    // Network configuration
    "ifconfig", "ip", "iptables", "route", "netstat",
    "firewall-cmd", "ufw", "pfctl",
    // Disk/filesystem (destructive potential)
    "fdisk", "mkfs", "parted", "mount", "umount",
    // Job scheduling
    "at", "batch",
];
```

**Logic:**
1. Extract the first word (command name) from the shell command
2. Handle `sudo command args` → check `command`, not `sudo`
3. Handle `command args | other` → check BOTH sides of the pipe
4. If banned command detected → block with clear message:
   ```
   "Command blocked: '{cmd}' is not allowed for security reasons. 
   Use dedicated tools (fetch for network, read/write/edit for file operations)."
   ```

### Step 2: Safe-Command Classification (Codex CLI pattern)

Classify commands as safe (read-only) vs potentially mutating:

```rust
const SAFE_COMMANDS: &[&str] = &[
    // File reading
    "cat", "head", "tail", "less", "more",
    // File search
    "find", "locate", "which", "whereis",
    // Text search
    "grep", "egrep", "fgrep", "rg", "ag",
    // File listing
    "ls", "dir", "tree",
    // File info
    "file", "stat", "wc", "du", "df",
    // Git (read-only subcommands only)
    "git",
    // Version info
    "rustc", "cargo", "go", "node", "python", "python3", "npm", "yarn",
    // System info
    "uname", "hostname", "whoami", "id", "date", "env", "printenv",
    // Text processing (read-only)
    "echo", "printf", "sort", "uniq", "cut", "tr", "awk", "sed",
    // Checksums
    "md5sum", "sha1sum", "sha256sum", "cksum",
    // Process listing
    "ps", "top", "htop",
];
```

**Logic:**
- If command starts with a safe command → mark as read-only, skip destructive preflight
- BUT: check for redirection operators `>` and `>>` even in safe commands
- `git` is special: only `status`, `log`, `diff`, `show`, `branch`, `tag`, `blame`, `ls-files`, `rev-parse` are safe
- `git commit`, `git push`, `git merge`, `git rebase` are mutating

### Step 3: Timeout Enforcement (Opencode pattern)

```rust
// Default timeout: 60 seconds (Opencode default)
// Max timeout: 600 seconds (10 minutes — Opencode max)
// Configurable via tool parameter: timeout_ms (optional)
// Background processes: auto-move to background after timeout
```

**Implementation:**
1. Spawn the shell command with `timeout` wrapper or tokio::time::timeout
2. On timeout → kill the process, return:
   ```
   "Command timed out after {N} seconds. Output captured before timeout:\n{partial_output}\n\n
   ⚠️ Consider breaking this into smaller steps or using a more specific command."
   ```
3. For background processes: implement `run_in_background` flag (Codex CLI pattern)
   - Return a `shell_id` that can be used with `shell_output` to read later
   - Track background shells in a session-level registry
   - Auto-kill background shells on session end

### Step 4: Output Truncation

- **Max output length**: 30,000 chars (Opencode limit)
- **Truncation message**: `"... [output truncated at 30,000 chars. {N} chars omitted]"` 
- **stderr handling**: If stderr is non-empty and stdout is empty, return stderr. If both have output, show stdout with stderr appended.
- **Binary output detection**: If output contains null bytes or >10% non-printable chars, return:
  ```
  "[Binary output detected ({N} bytes). Use a different approach to inspect this data.]"
  ```

### Step 5: Enhance Destructive Command Preflight

Current preflight in `src/shell_preflight.rs` already detects dangerous patterns (`rm`, `mv`, `>`, etc.). Enhance:

1. **`2>/dev/null` bypass** (already fixed in Task 312-B, verify it's still working)
2. **Protected path whitelist**: Add `project_tmp/` to non-protected paths (the user puts temp files there)
3. **Unscoped glob detection**: `rm *.rs` is dangerous; `rm src/main.rs` is not. Current detection should flag `rm *` without a specific directory prefix.
4. **Chained destructive commands**: `cd /important && rm -rf *` — detect `cd` + destructive combo

### Step 6: Persistent Shell Reliability

- **PS1/PS2**: Already set to empty in Task 312-E. Verify.
- **Command echo suppression**: Already fixed with `stty -echo` in Task 312-E. Verify.
- **Marker-based output capture**: Use unique markers to capture output boundaries reliably
- **Shell crash recovery**: If the persistent shell dies (EOF), auto-restart with a fresh shell

## Success Criteria

- [ ] Ban list blocks all listed dangerous commands
- [ ] Banned commands in pipes also blocked
- [ ] Safe commands classified correctly (skips destructive preflight)
- [ ] Git subcommand classification (read vs mutating)
- [ ] Timeout: default 60s, max 600s, configurable
- [ ] Output truncated at 30,000 chars with clear message
- [ ] Binary output detected and blocked
- [ ] Background shell support with `shell_id`
- [ ] Persistent shell PS1/PS2 empty (already fixed, verify)
- [ ] Shell crash auto-recovery
- [ ] Protected path updates (project_tmp/ whitelisted)
- [ ] `cargo build` succeeds, all shell tests pass

## Anti-Patterns To Avoid

- **Do NOT ban `ls`, `find`, `grep`** — these are essential for exploration; use dedicated tools for these instead
- **Do NOT add an allowlist** — ban list only; allowlists break too easily across different systems
- **Do NOT block pipes/redirections wholesale** — `2>/dev/null` and `| head` are essential patterns
- **Do NOT add network calls in the shell tool** — use a dedicated `fetch` tool for network access
