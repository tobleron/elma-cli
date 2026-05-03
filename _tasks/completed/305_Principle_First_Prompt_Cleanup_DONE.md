# Task 305: Principle-First Prompt Cleanup

**Status:** Pending
**References:** Directive 006

## Objective

Remove all example-driven prompt passages from Elma's system prompts and router TOML files. Replace concrete examples with principle-based behavioral descriptions.

## Scope

1. **`src/orchestration_core.rs` lines 87-96**: Replace 7 concrete user-request→tool-action mapping examples with principle descriptions (e.g., "Gather evidence before asserting facts" instead of "User asks 'what time is it?' → Use tool_search then shell with date command")
2. **`config/defaults/router.toml`**: Replace inline examples with principled routing criteria
3. **`config/defaults/speech_act.toml`**: Replace inline examples with principled distinguishing criteria
4. Run E2E routing probes to verify no regression after rewrite

## Verification

```bash
cargo build
cargo test
# Run routing calibration probes to verify accuracy
./reliability_probe.sh
```
