# 213: Shell String Parsing via `shlex`

## Status
`pending`

## Crate
`shlex` — Quote and split shell-style strings safely.

## Rationale
Elma executes shell commands and may need to parse user-provided command strings that include quoted arguments, variable expansions, or pipe operators. Raw string splitting (`s.split_whitespace()`) breaks on arguments with spaces or embedded quotes. `shlex` handles POSIX shell quoting conventions safely and is the standard for this in Rust.

## Implementation Boundary
- Add `shlex = "1.3"` to `Cargo.toml`.
- Audit places where Elma parses command strings (e.g., `shell_command` tool, subprocess execution in `tokio::process::Command`, or any user-input command parsing).
- Replace raw whitespace splits with `shlex::split()`:

  ```rust
  use shlex::split;

  pub fn parse_command(input: &str) -> anyhow::Result<Vec<String>> {
      split(input).context("invalid shell quoting in command")
  }
  ```

- Return an error (not panic) on malformed quoting.
- Handle `None` from `shlex::split()` as a parse error with a user-facing message.
- Do NOT expand environment variables or tilde `~` — that is `shellexpand`'s job (deferred to Task 214).
- Do NOT change the shell execution path — only how input strings are tokenized.

## Verification
- `cargo build` passes.
- `parse_command(r#"echo "hello world" 'single quoted' foo"#)` returns `["echo", "hello world", "single quoted", "foo"]`.
- Malformed quoting (e.g., unmatched `"`) returns an error, not panics.
- Existing command execution behavior unchanged.