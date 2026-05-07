# Task 744: Add Timeout And Cancellation To Workspace Discovery Blocking Call

## Type

Reliability / Security / Hardening

## Severity

High

## Scope

Workspace discovery, chat loop, blocking I/O

## Problem

`src/app_chat_loop.rs` contains `try_workspace_discovery()`, a **blocking, unbounded filesystem operation**:

```rust
fn try_workspace_discovery(runtime: &mut AppRuntime, line: &str) {
    let Some(path) = extract_first_path_from_user_text(line) else { return; };

    let canonical_path = match std::fs::canonicalize(&path) { Ok(p) => p, Err(_) => return };
    let workspace_root = match std::fs::canonicalize(".") { Ok(p) => p, Err(_) => return };
    if !canonical_path.starts_with(&workspace_root) { return; }

    let safe_path = quote(&path);
    let cmd = format!(
        "ls -R {safe_path} | head -n 100; echo '---'; file -b {safe_path}/* 2>/dev/null | head -n 10"
    );
    let output = crate::workspace::cmd_out(&cmd, &std::path::PathBuf::from("."));
    if !output.trim().is_empty() {
        runtime.ws = format!("### GROUNDED WORKSPACE DISCOVERY ({path})\n{}\n\n{}", ...);
    }
}
```

Problems:
1. **No timeout**: `ls -R` on a large directory tree can hang indefinitely. A deep `node_modules/` or `.git/` tree can take minutes.
2. **No cancellation**: if the user submits a new message while `ls -R` is running, the operation continues in the background, blocking the thread.
3. **Blocking I/O in async context**: this function is called from `run_chat_loop()`, an async function, but it uses synchronous `std::fs::canonicalize` and `workspace::cmd_out` without `tokio::task::spawn_blocking`.
4. **Bypasses workspace exclusions**: `ls -R` does not respect `workspace_policy.rs` exclusions. It will recurse into `target/`, `node_modules/`, `.git/`, etc.
5. **Shell injection risk**: while `shlex::quote` is used, the path is user-provided and comes from `extract_first_path_from_user_text()`, which may not properly validate paths.
6. **Violates Rule 13**: "Add real timeout mechanisms for blocking I/O instead of relying on model self-correction."

The docs say:
> "Workspace-Only File Access: Core file tools operate within the workspace boundary by default."
> "FileScout handles explicit whole-system read-only discovery as a separate, opt-in capability."

But `try_workspace_discovery` does whole-system discovery implicitly, without opt-in.

## Root Cause

`try_workspace_discovery` was added as a "grounded workspace discovery" feature to help the model understand the workspace. It was never hardened for production use.

## Proposed Solution

### Phase 1 — Add timeout and cancellation

1. Wrap the discovery logic in `tokio::time::timeout`:
   ```rust
   let discovery_future = tokio::task::spawn_blocking(|| {
       // ... blocking fs operations ...
   });
   match tokio::time::timeout(Duration::from_secs(5), discovery_future).await {
       Ok(Ok(output)) => { /* use output */ }
       Ok(Err(_)) | Err(_) => {
           trace(&runtime.args, "workspace_discovery_timeout");
       }
   }
   ```

2. Make `try_workspace_discovery` async so it can be awaited with a timeout.

### Phase 2 — Respect workspace exclusions

1. Use `workspace_policy.rs` exclusions in the `ls -R` command:
   ```rust
   let exclusions = DEFAULT_EXCLUDED_PATHS.join(" --exclude=");
   let cmd = format!("ls -R --exclude={} {safe_path} | head -n 100", exclusions);
   ```

   Or better: use `find` with `-prune` for excluded directories.

2. Alternatively, replace `ls -R` with a Rust-native directory walk that respects exclusions:
   ```rust
   use ignore::WalkBuilder;
   let mut entries = Vec::new();
   for result in WalkBuilder::new(&path)
       .hidden(false)
       .git_ignore(true)
       .max_depth(Some(3))
       .build()
   {
       // collect entries, respecting .gitignore
   }
   ```

### Phase 3 — Remove shell execution

1. Replace `workspace::cmd_out(&cmd, ...)` with native Rust filesystem operations
2. This eliminates shell injection risk entirely
3. Use `std::fs::read_dir` or `ignore::Walk` instead of `ls -R`
4. Cap the total entries at 100 (already done via `head -n 100`, but native code should enforce this)

### Phase 4 — Validate path more strictly

1. After `canonical_path`, verify it's a directory (not a file)
2. Verify it's readable
3. If the path is a single file, skip `ls -R` and just note the file's existence

### Phase 5 — Make discovery opt-in or smarter

1. Instead of running discovery on every message, only run it when:
   - The message contains a directory path
   - AND the path hasn't been discovered in this session
   - AND the message doesn't look like a simple chat greeting
2. Cache discovered paths in `AppRuntime` to avoid redundant `ls -R` calls

## Acceptance Criteria

- [ ] `try_workspace_discovery` has a hard 5-second timeout
- [ ] Discovery runs in `spawn_blocking` or uses async filesystem APIs
- [ ] Discovery respects `DEFAULT_EXCLUDED_PATHS` (no recursion into `.git/`, `target/`, etc.)
- [ ] No shell command is constructed from user input
- [ ] Path validation checks directory existence and readability
- [ ] Discovery result is cached per session
- [ ] If discovery times out, the chat loop continues without blocking
- [ ] `cargo build && cargo test` passes
- [ ] Unit test: large directory tree → discovery times out gracefully
- [ ] Unit test: path with `../` → blocked by workspace boundary check

## Verification Plan

- Unit test: mock large directory → timeout fires within 5 seconds
- Unit test: path in `DEFAULT_EXCLUDED_PATHS` → not recursed
- Unit test: path is a file → no directory listing attempted
- Integration test: user says "read src/main.rs" → no workspace discovery (cached or skipped)
- Security test: user says "read /etc/passwd" → blocked by workspace boundary

## Dependencies

- `src/workspace_policy.rs` (exclusions)
- `src/workspace.rs` (`cmd_out` — may be removed)
- `src/app_chat_loop.rs` (call site)
- `ignore` crate (already in Cargo.toml)

## Notes

This is a **blocking I/O safety** fix. The docs say:

> "Do: add real timeout mechanisms for blocking I/O instead of relying on model self-correction."

The current implementation is a reliability hazard. A user typing "read /" could trigger an unbounded `ls -R /` that hangs the entire agent.

Native Rust filesystem operations are preferred over shell commands for workspace discovery. The shell command was a shortcut that introduced injection risk and exclusion bypass.

If `ignore::WalkBuilder` is used, ensure `.git_ignore(true)` is set so `.gitignore` rules are respected, but also ensure `DEFAULT_EXCLUDED_PATHS` are pruned even if not in `.gitignore`.
