# Task 729: Noninteractive Piped Input Should Exit After Final Response

## Type

CLI Lifecycle / Headless Mode / Basic Functionality

## Severity

High

## Evidence

Post-round smoke test:

```bash
printf 'Say exactly: smoke-ok\n' | target/debug/elma-cli --debug-trace --no-color
```

Elma rendered the expected response:

```text
Response: smoke-ok
```

but the process remained alive for more than 90 seconds until manually terminated.

Process evidence:

```text
target/debug/elma-cli --debug-trace --no-color
```

was still running after the final response was visible.

## Problem

Piped stdin / noninteractive use must terminate after completing the supplied request. Hanging after a final answer breaks shell scripts, automated prompt suites, CI smoke tests, and user expectations for CLI behavior.

Long autonomous operation should apply to incomplete work, not to a completed one-shot noninteractive request with EOF on stdin.

## Requirements

- Detect noninteractive stdin/EOF mode explicitly.
- After final response or terminal stop state, exit cleanly when there is no interactive input stream to return to.
- Preserve interactive TUI behavior for normal terminal use.
- Add a regression test or harness that runs a simple piped prompt and asserts the process exits within a short timeout after producing the answer.
- Ensure debug/no-color mode does not keep the renderer event loop alive after completion.

## Likely Files

- `src/app_chat_loop.rs`
- `src/app_chat_orchestrator.rs`
- `src/ui/ui_terminal.rs`
- `src/app_bootstrap_modes.rs`
- `src/main.rs`

## Acceptance Criteria

- [ ] `printf 'Say exactly: smoke-ok\n' | target/debug/elma-cli --debug-trace --no-color` exits after the final answer.
- [ ] Interactive terminal sessions remain open for the next user turn.
- [ ] Prompt-suite harness no longer needs external kill logic for completed one-shot runs.

