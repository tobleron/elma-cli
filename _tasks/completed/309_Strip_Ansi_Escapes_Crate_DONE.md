# Task 309: Wire `strip-ansi-escapes` Crate

**Status:** completed
**References:** Proposal 004

## Objective

The `strip-ansi-escapes` crate is declared in `Cargo.toml` but never imported. Integrate it to strip ANSI escape sequences from shell command output before storage, ensuring clean evidence entries and transcript text.

## Scope

1. Import `strip-ansi-escapes` in `execution_steps_shell_exec.rs` and/or `evidence_ledger.rs`
2. Strip ANSI escapes from shell output before adding evidence entries
3. Strip ANSI escapes from displayed output in transcript rendering
4. Write tests verifying ANSI-stripped output

## Verification

```bash
cargo build
cargo test strip_ansi
```
