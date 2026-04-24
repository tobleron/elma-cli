# 210: Shell Completions via `clap_complete`

## Status
`pending`

## Crate
`clap_complete` — Generates shell completions for Bash/Zsh/Fish/PowerShell.

## Rationale
Elma already uses `clap` (with derive). `clap_complete` generates production-quality shell completions from the existing `clap` app with zero extra work. Users get tab completion for commands, flags, and subcommands without manual completion script maintenance.

## Implementation Boundary
- Add `clap_complete = "4.5"` to `[dependencies]` in `Cargo.toml`.
- Identify the root `clap::Command` in `src/main.rs` or wherever `clap` is initialized.
- Add a `--generate-completion <shell>` flag (or `completion` subcommand) to the CLI:

  ```rust
  use clap_complete::{Generator, shells};
  use std::io;

  fn generate_completion<G: Generator>(cmd: &mut clap::Command, name: &str) {
      let mut buf = io::Cursor::new(Vec::new());
      clap_complete::generate(G, cmd, name, &mut buf);
      println!("{}", String::from_utf8(buf.into_inner()).unwrap());
  }
  ```

- Support at minimum: `bash`, `zsh`, `fish`, `powershell`.
- Provide a `elma completion <shell>` subcommand or `--generate-completion <shell>` flag — prefer subcommand if Elma already has subcommands.
- Add a section to `README.md` (or inline `--help` text) explaining how to install completions (e.g., `elma completion bash >> ~/.bashrc`).
- Do NOT hardcode completion data — generate it dynamically at runtime.
- Do NOT modify the clap app structure; just wire the generator to the existing app.

## Verification
- `cargo build` passes.
- `elma completion bash` outputs valid bash completion script.
- `elma completion fish` outputs valid fish completion script.
- `elma --help` shows the new flag/subcommand.