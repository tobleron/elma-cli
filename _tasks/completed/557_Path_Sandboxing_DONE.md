# 557 — Add Path Sandboxing for Shell Tool Targets

- **Priority**: High
- **Category**: Security
- **Depends on**: 552 (split tool_calling.rs for clean extraction)
- **Blocks**: None

## Problem Statement

The `exec_shell` function in `tool_calling.rs` passes command strings directly to the operating system shell after preflight and permission checks. Unlike the `read`, `edit`, `write`, `ls`, `glob` tools which explicitly reject absolute paths and resolve relative paths against the workspace root, the `shell` tool has no path sandboxing.

When a model issues `shell` with a command like `cat /etc/passwd` or `ls /Users/other/project`, the shell preflight may classify it as "Safe" (read-only) and the permission gate may auto-approve it in certain safe modes. The command then executes with full filesystem access.

The `resolve_path()` function in `shell_preflight.rs` is used only for dry-run previews (mv/cp/rm) and accepts absolute paths verbatim.

## Why This Matters for Small Local LLMs

Small models are prone to:
- Hallucinating absolute paths from training data (`/home/user/project`, `/etc/config`)
- Confusing the current workspace with training-data paths
- Not understanding that the shell runs in a specific working directory
- Using paths from previous sessions or different workspaces

Without path sandboxing, a small model can accidentally read or modify files outside the intended workspace.

## Current Behavior

```rust
// tool_calling.rs - exec_shell
let command = av["command"].as_str().unwrap_or("").to_string();
// ... preflight ...
match run_shell_persistent(&command, workdir).await { ... }
```

The command string is passed to `run_shell_persistent` which executes it in `workdir`. There's no inspection of the command's path arguments.

## Recommended Target Behavior

Add a path sandboxing layer that:

1. **Extracts file paths from shell commands** using shell-aware parsing (shlex)
2. **Checks each path against workspace boundaries** — paths must be within workspace or explicitly whitelisted
3. **Resolves relative paths** against workspace root
4. **Blocks absolute paths** outside the workspace by default (configurable)
5. **Provides clear error messages** to the model about which paths were blocked
6. **Integrates with `workspace_policy`** for per-project allowlists

### Configuration

```toml
# config/runtime.toml
[sandbox]
allow_absolute_paths = false              # block /etc, /home, etc.
allow_paths_outside_workspace = false     # block ../ parent traversal
whitelist_read_paths = ["/usr/bin", "/usr/local/bin"]  # allow reading system bins
whitelist_readonly = true                 # whitelist is read-only
```

## Source Files That Need Modification

- `src/tool_calling.rs` (or `src/tools/exec_shell.rs` after Task 552) — Add path sandboxing to `exec_shell`
- `src/shell_preflight.rs` — Add `sandbox_paths()` function, integrate into `preflight_command()`
- `src/persistent_shell.rs` — Optionally add cwd locking
- `src/workspace_policy.rs` — Add sandbox configuration
- `config/runtime.toml` — Add `[sandbox]` section

## New Files/Modules

- `src/shell_sandbox.rs` — Path extraction from shell commands, workspace boundary checking

## Step-by-Step Implementation Plan

1. Create `src/shell_sandbox.rs` with:
   ```rust
   pub struct ShellSandbox {
       workspace_root: PathBuf,
       allow_absolute: bool,
       allow_outside_workspace: bool,
       whitelist_read: Vec<PathBuf>,
   }
   
   impl ShellSandbox {
       pub fn check_command(&self, command: &str) -> Result<(), Vec<SandboxViolation>> {
           // 1. Parse with shlex to get argument tokens
           // 2. Identify tokens that look like paths (contain / or common extensions)
           // 3. Check each path against workspace boundaries
           // 4. Return violations
       }
   }
   ```
2. Integrate into `shell_preflight::preflight_command()` as an additional check step
3. Add sandbox violations to `PreflightResult`:
   ```rust
   pub struct PreflightResult {
       pub risk: RiskLevel,
       pub error_guidance: Option<String>,
       pub dry_run_preview: Option<String>,
       pub sandbox_violations: Vec<SandboxViolation>,  // NEW
   }
   ```
4. Load sandbox config from `[sandbox]` section in runtime config
5. Add workspace boundary detection (determine workspace root from session)
6. Handle edge cases:
   - Pipes: `find /etc | xargs cat` — detect `/etc` path
   - Redirects: `ls > /tmp/output` — detect `/tmp/output`
   - Shell variables: `cat $HOME/file` — warn about unresolved variables
   - Backticks: `` `cat /etc/passwd` `` — detect path inside backticks

## Recommended Crates

- `shlex` — already a dependency, used for shell tokenization
- `camino` — optional, for UTF-8 path handling

## Validation/Sanitization Strategy

- All path tokens are checked after shlex parsing
- Paths are canonicalized before comparison to prevent `../../` traversal
- Symlinks are NOT followed for path boundary checks (followed at execution time)
- Whitelist paths are cached per session
- Unknown tokens (not clearly paths) are allowed through — default-permit for safety

## Testing Plan

1. Unit tests for path extraction from various shell command patterns
2. Test that `cat /etc/passwd` is blocked (absolute, not whitelisted)
3. Test that `ls /usr/bin` is allowed (whitelisted read path)
4. Test that `cat ../../other-project/file` is blocked (outside workspace)
5. Test that `find . -name '*.rs' | xargs cat` passes (all workspace-relative)
6. Test that shell variables are warned about
7. Integration test: blocked command returns structured error to model

## Acceptance Criteria

- Absolute paths outside workspace are blocked by default
- Whitelisted read paths work for read-only operations
- Workspace-relative paths work normally
- Preflight integration returns clear error messages for blocked paths
- Existing shell tool behavior unchanged for valid commands
- Config file supports `[sandbox]` section

## Risks and Migration Notes

- **Breaking change**: Existing users who rely on Elma to access system files via shell will need to whitelist paths. Consider a deprecation window with warnings before hard-blocking.
- **Path extraction is heuristic**: Not all shell arguments that look like paths are paths (e.g., `git branch -d feature/login` — "feature/login" is a branch name, not a path). Start with conservative detection (only block clear paths) and refine.
- **Performance**: Path canonicalization on every shell command may add overhead. Cache workspace boundary checks.
- Pair with Task 552 (split tool_calling.rs) to add sandboxing to the extracted `exec_shell.rs` module directly.
