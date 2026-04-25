# 218: Man Page Generation via `clap_mangen`

## Status
`pending`

## Crate
`clap_mangen` — Generates man pages from your `clap` app.

## Rationale
Elma already uses `clap`. `clap_mangen` produces valid `groff`/`man` format man pages directly from the existing `clap` app definition — zero additional documentation work. Users on Linux/macOS who prefer `man elma` to `--help` get professional man page output. Pairs naturally with `clap_complete` (Task 210).

## Implementation Boundary
- Add `clap_mangen = "0.12"` to `Cargo.toml`.
- Add a `--generate-man-page` flag or `elma man` subcommand to the CLI root:

  ```rust
  use clap_mangen::Man;

  fn generate_man(cmd: &clap::Command) -> anyhow::Result<()> {
      let mut buf = Vec::new();
      let man = Man::new(cmd);
      man.render_to(&mut buf)?;
      std::io::Write::all(&mut buf, std::stdout())?;
      Ok(())
  }
  ```

- Output valid man page to stdout on invocation.
- Support `--section <N>` option to specify man section (default section 1 for CLI tools).
- Document the generation command in `--help` output and README: `elma manpage > elma.1`.
- Do NOT maintain a static man page — generate it dynamically from the clap app.
- Do NOT add a runtime man page installation step — that's distribution-specific.

## Verification
- `cargo build` passes.
- `elma --generate-man-page` outputs valid `groff` format (pipe to `man -l -` to test locally).
- `groff -man -Tutf8 <(elma --generate-man-page)` renders correctly.
- Man page includes all subcommands and flags from the existing clap app.