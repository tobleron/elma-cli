# Task 379: DSL Path Command And Edit Safety

**Status:** pending
**Priority:** critical
**Suite:** Compact DSL Model-Output Migration
**Depends on:** Task 377; Tasks 337 and 340 are preferred but can start with minimal local equivalents if needed
**Blocks:** Task 378
**Absorbs/Reframes:** Task 326 for DSL `E` semantics

## Objective

Centralize safety validation for DSL file paths, shell/verification commands, and exact edits. This task ensures the new compact protocol is safer than the old JSON/native tool-call path, not merely easier for models to emit.

## Required Deliverables

- Shared workspace path validator.
- Shared command policy executor for DSL `X`.
- Shared exact edit engine for DSL `E`.
- Focused tests for path, command, symlink, and edit failure behavior.

## Path Safety Requirements

- Validate paths before any filesystem operation.
- Reject absolute paths.
- Reject `..`, root, and platform prefix components.
- Canonicalize the workspace root.
- Canonicalize existing targets and parent directories safely.
- Reject symlink escapes.
- Reject writes outside project root.
- Return model-facing `UNSAFE_PATH` repair observations with the bad path.

For non-existing edit targets, canonicalize the nearest existing parent and ensure it remains inside the root.

## Command Safety Requirements

Define:

```rust
pub(crate) enum CommandPolicy {
    Strict,
    AskBeforeUnsafe,
    Disabled,
}
```

Initial implementation uses `Strict` for DSL `X`.

Allowed command families:

- `cargo check`
- `cargo test`
- `cargo fmt`
- `cargo clippy`
- `git diff`
- `git status`
- `ls`
- `rg`
- `grep`

Rules:

- Parse with `shlex::split`.
- Execute with `std::process::Command`.
- Never call `sh -c` with model output.
- Reject shell control operators unless a future explicit policy allows them.
- Reject dangerous commands by default: `rm`, `sudo`, `su`, `chmod`, `chown`, `curl | sh`, `wget | sh`, `mkfs`, `dd`, `kill -9`, `launchctl unload`, `systemctl disable`.
- Return `UNSAFE_COMMAND` with allowed command summary.

## Edit Safety Requirements

For DSL `E`:

1. Validate path.
2. Require prior read if the file context tracker exists; otherwise add a minimal session-local read fingerprint gate.
3. Read file as text with encoding checks.
4. Reject binary or oversized files.
5. Check OLD exists exactly once.
6. If OLD occurs zero times: return `INVALID_EDIT` and suggest `R path="..."`.
7. If OLD occurs multiple times: return `INVALID_EDIT` and ask for a larger unique OLD block.
8. Apply replacement to a temp buffer first.
9. Create automatic snapshot before first edit in a turn/session where existing snapshot behavior supports it.
10. Write atomically through a temp file in the same directory.
11. Preserve line endings and supported BOMs where practical.
12. Return compact diff/summary observation.

## Implementation Steps

1. Add path validation helpers and use them from read/list/search/edit/command paths.
2. Add command policy parser and allowlist matcher.
3. Add exact edit engine or adapt the existing pending edit-engine plan around DSL `E`.
4. Wire path/command/edit errors into the shared DSL repair renderer.
5. Add regression tests for all unsafe and invalid cases before `run_tool_loop` cutover.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test program_utils
cargo test edit_engine
cargo test agent_protocol
cargo test shell_preflight
cargo check --all-targets
```

Required coverage:

- absolute path rejected
- `../` escape rejected
- symlink escape rejected
- missing parent handled safely
- allowed `X` commands execute with direct args
- disallowed `X` commands rejected
- shell pipelines rejected in strict mode
- OLD not found rejected
- OLD multiple matches rejected
- exact single edit succeeds atomically
- edit failure leaves file unchanged

## Done Criteria

- All DSL filesystem and command operations share one validation path.
- `E` failures are precise enough for small models to recover.
- No DSL command can bypass workspace or shell safety by malformed text.

## Anti-Patterns

- Do not use shell string interpolation for search or command execution.
- Do not silently normalize paths into safe-looking alternatives.
- Do not replace the first OLD match when there are multiple matches.
