# 241: Remove `atty` Crate — Use `std::io::IsTerminal`

## Status
`completed`

## Priority
High — `atty 0.2` has a known soundness advisory (RUSTSEC-2021-0145). `std::io::IsTerminal` (stable since Rust 1.70) is already imported in the codebase and is a direct drop-in replacement.

## Source
Code review architecture note. `Cargo.toml` depends on `atty = "0.2"` which has a known unsoundness in rare signal-handling scenarios. `std::io::IsTerminal` is already used in `ui_terminal.rs:21` and is the correct modern replacement.

## Objective
Remove the `atty` crate entirely and replace all call sites with `std::io::IsTerminal` or equivalent.

## Scope

### `Cargo.toml`
- Remove `atty = "0.2"` from `[dependencies]`.

### `src/ui/ui_terminal.rs`
- Line 68–70: replace `atty::is(atty::Stream::Stdin)` with `std::io::stdin().is_terminal()`.
  ```rust
  #[cfg(unix)]
  fn is_stdin_tty() -> bool {
      std::io::stdin().is_terminal()
  }
  ```

### `src/permission_gate.rs`
- Line 56: replace `!atty::is(atty::Stream::Stdin)` with `!std::io::stdin().is_terminal()`.
- Line 122: same replacement.

### Search for remaining `atty` uses:
```bash
rg 'atty::' src/
```
Replace every occurrence with the `IsTerminal` equivalent. The `IsTerminal` trait is in scope via `use std::io::IsTerminal` (already in `ui_terminal.rs`).

### `src/main.rs`
- Remove any `extern crate atty` or `use atty` lines if present.

## Verification
- `cargo build` passes with zero `atty` references in `src/`.
- `cargo audit` no longer reports RUSTSEC-2021-0145.
- `rg 'atty' Cargo.toml src/` returns zero matches.
- TTY detection still works: run `elma --help` piped vs direct — non-interactive mode correctly detected.

## References
- `Cargo.toml:28`
- `src/ui/ui_terminal.rs:67–75`
- `src/permission_gate.rs:56, 122`
- RUSTSEC-2021-0145: https://rustsec.org/advisories/RUSTSEC-2021-0145
