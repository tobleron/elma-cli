# Task 303: Offline-First Architecture

**Status:** completed
**References:** Directive 004

## Objective

Make Elma fully functional offline by default. Add connectivity detection, default to local endpoints when no remote is configured, and provide clear, actionable errors when a remote API is unreachable.

## Scope

1. Add `check_connectivity()` to `llm_provider.rs` that tests endpoint reachability on provider creation
2. Default to `OpenAICompatible` with `http://localhost:8080` when no profile is specified
3. Add startup warning when configured for remote but endpoint is unreachable
4. Replace opaque connection errors with actionable messages ("Start your local model server or configure a remote endpoint")
5. Document offline capabilities and limitations in `docs/CONFIGURATION.md` and `docs/README.md`

## Verification

```bash
cargo build
cargo test
# Manual: run without a local server, verify helpful error
```
