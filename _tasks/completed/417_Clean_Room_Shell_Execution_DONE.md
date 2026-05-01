# Task 417: Clean Room Shell Execution

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 1-2 days
**Dependencies:** Task 387, Task 406
**References:** completed Task 288 persistent guarded shell; AGENTS.md Rule 7 input sanitization

## Objective

Harden shell execution so shell fallback output is clean, bounded, and free of login-profile noise.

## Problem

Even with rust-native tools preferred, shell remains necessary for user-requested commands and unsupported operations. Shell output that includes login prompts, startup banners, ANSI control sequences, or corrupted carriage-return behavior increases small-model cognitive load and can cause false tool results.

## Implementation Plan

1. Audit current shell launch paths in:
   - `src/execution_steps_shell.rs`
   - `src/execution_steps_shell_exec.rs`
   - `src/persistent_shell.rs`
2. Ensure non-interactive commands avoid loading user login profiles unless explicitly required.
3. Inject a minimal environment and preserve essential `PATH`.
4. Sanitize ANSI/control sequences before model-visible output.
5. Preserve raw output as a session artifact when needed for debugging.
6. Add transcript rows when shell is used as fallback instead of a rust-native tool.

## Non-Scope

- Do not remove shell support.
- Do not replace rust-native tools with shell wrappers.
- Do not rely on model self-correction for blocking I/O or noisy output.

## Verification

```bash
cargo test shell
cargo test persistent_shell
cargo build
```

Manual probes:

- `date`
- `printf 'hello\n'`
- a command with ANSI color output

## Done Criteria

- Shell command output reflects command output, not shell profile noise.
- ANSI/control sanitization is tested.
- Shell fallback remains permissioned, bounded, and transcript-visible.

