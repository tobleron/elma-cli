# 240: Shell Injection Fix In Workspace Discovery

## Status
`completed`

## Priority
High — Security: path extracted from raw user text is interpolated into a shell format string.

## Source
Code review finding H-11. In `app_chat_loop.rs::try_workspace_discovery`, the `path` variable (extracted from raw user input via `extract_first_path_from_user_text`) is directly interpolated into a shell command format string:

```rust
let cmd = format!(
    "ls -R '{path}' | head -n 100; echo '---'; file -b '{path}'/* 2>/dev/null | head -n 10"
);
```

A user input of `'; rm -rf /important; echo '` produces a valid shell command that deletes files.

## Objective
Eliminate the shell injection by either quoting the path properly via `shlex::quote` or by replacing the shell command with direct `std::process::Command` invocations that pass path as an argument (not interpolated into a string).

## Scope

### `src/app_chat_loop.rs` — `try_workspace_discovery`

**Option A (minimal — use shlex::quote):**
```rust
fn try_workspace_discovery(runtime: &mut AppRuntime, line: &str) {
    let Some(path) = extract_first_path_from_user_text(line) else { return; };
    let safe_path = shlex::quote(&path).to_string();
    let cmd = format!(
        "ls -R {safe_path} | head -n 100; echo '---'; file -b {safe_path}/* 2>/dev/null | head -n 10"
    );
    let output = crate::workspace::cmd_out(&cmd, &std::path::PathBuf::from("."));
    // ...
}
```

**Option B (preferred — use Command directly):**
Replace the shell string with individual `Command::new("ls")` / `Command::new("file")` calls using `.arg(&path)` so no shell interpolation occurs at all.

### Also apply path validation:
Before building any command, validate that the extracted path does not traverse outside the workspace root:
```rust
let canonical = std::fs::canonicalize(&path).ok();
let workspace_root = std::fs::canonicalize(".").ok();
if let (Some(p), Some(root)) = (canonical, workspace_root) {
    if !p.starts_with(&root) {
        return; // Path escape attempt — silently skip
    }
}
```

## Verification
- `cargo build` passes.
- Manual test: input `'malicious'; echo 'pwned'` as a user message — verify no shell execution occurs with injected content.
- Manual test: input a valid relative path like `src/` — verify workspace discovery still works correctly.
- `cargo test` passes.

## References
- `src/app_chat_loop.rs:421–436` (try_workspace_discovery)
- `shlex` crate is already in `Cargo.toml` (Task 213).
