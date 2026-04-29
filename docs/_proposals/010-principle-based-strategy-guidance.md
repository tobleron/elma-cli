# Proposal 010: Principle-Based Strategy Guidance

**Status:** Accepted  
**Author:** Elma (research-driven)  
**Date:** 2026-04-28  
**Related Tasks:** T303  

## Summary

Replace `suggest_alternatives()` hardcoded match arms in `stop_policy.rs:431-449` with principle-based guidance derived from the structured evidence already recorded by the stop policy, plus a generic fallback that teaches the model *how* to think about strategy shifts rather than listing specific alternative commands.

## Motivation

**Current state:** `suggest_alternatives()` (`stop_policy.rs:431-449`) has hardcoded match arms for specific command strategies:
```rust
"find_other" | "find_count" | "find_mtime" | "find_ls" => {
    "Alternative approaches:\n- Use `rg` (ripgrep)...\n- Use `fd` for faster file finding..."
}
"find_du_aggregate" => { ... }
"stat_loop" => { ... }
"du_aggregate" => { ... }
_ => { ... }
```

**Problems:**
1. **Violates Elma constraint #1** — hardcoded pattern matching on command strategies. If a new strategy class emerges (e.g. `xargs_loop`), it falls to the catch-all which gives generic advice that may not apply.
2. **Brittle to tool evolution** — if `fd` gets installed/removed, if `rg` gets aliased, the hardcoded suggestions become wrong.
3. **Teaches the model to pattern-match rather than reason** — the model learns "when I get this hint, try these commands" instead of "when my strategy fails, think about what evidence I'm missing and what tool would find it."

**Elma philosophy:** Prompts must describe reasoning principles, not list examples. The strategy shift hint should teach the model *how* to think about strategy shifts, not *what* commands to try.

## Design

### Replace hardcoded match arms with principle-based guidance

Instead of:
```rust
"find_other" => "Use `rg`... Use `fd`..."
```

Generate guidance from:
1. **The failed strategy name** — to acknowledge what was tried
2. **The error class** (already available from `classify_error`) — to suggest root cause
3. **The scope classification** (narrow/medium/wide) — to suggest scope adjustment
4. **Principle-based generic advice** — that works regardless of tool availability

```rust
fn suggest_alternatives(failed_strategy: &str, error_class: &str, scope: &str) -> String {
    let error_guidance = match error_class {
        "timeout" | "killed_sigkill" => 
            "This command exceeded time/memory limits. Consider: narrowing the scope with specific paths or -maxdepth, breaking into per-directory steps, or using a lighter-weight tool.",
        "permission_denied" =>
            "This command hit a permission barrier. Consider: targeting specific accessible directories, or using read tool for known files.",
        "not_found" | "no_such_file" =>
            "The target doesn't exist at the expected path. Consider: listing the parent directory first, checking the workspace tree, or trying alternative path patterns.",
        "command_not_found" =>
            "The command is not available on this system. Consider: using a different tool (read, search), or checking what shell utilities are available with `which`.",
        _ =>
            "The command failed for an unexpected reason. Consider: using a different tool type (read/search instead of shell), or breaking the task into smaller steps.",
    };

    let scope_guidance = match scope {
        "wide" => "This was a wide-scope operation. Try narrowing with -maxdepth, specific base paths, or file-type filters (-name '*.ext').",
        "medium" => "Try further narrowing the scope, or split into per-subdirectory passes.",
        "narrow" => "Even with narrow scope this failed. The issue may be tool choice rather than scope — consider read or search tools.",
        _ => "",
    };

    format!(
        "Strategy '{}' failed with error class '{}'.\n\n{}\n\n{}",
        failed_strategy, error_class, error_guidance, scope_guidance
    )
}
```

### How error_class and scope arrive

After Proposal 007, `classify_error_class()` returns typed classes (not string-matched). `estimate_command_scope()` already exists (`stop_policy.rs:~410`) and returns narrow/medium/wide. Both are already tracked per shell failure in `record_shell_failure()`.

### Why this satisfies constraint #1

The suggestion is generated from **structured evidence** (`error_class` enum, `scope` classification) not from **command-name string matching**. It describes *what went wrong* and *principles for recovery* rather than listing specific alternative commands. The model must reason about which tool to use, not pattern-match a hint to a command list.

### Module Changes

| Module | Change |
|--------|--------|
| `src/stop_policy.rs:431-449` | Replace `suggest_alternatives()` with principle-based version |
| `src/stop_policy.rs:251-265` | Update `strategy_shift_hint()` to pass error_class and scope |

## Alternatives Considered

| Alternative | Why Rejected |
|-------------|-------------|
| Keep hardcoded map, add more entries | Violates constraint #1. Never converges — new strategies emerge. |
| Remove `suggest_alternatives` entirely, just say "try different approach" | Too vague for small models. They need specific *types* of alternatives, not just "try something else." |
| Use model to generate alternatives dynamically | Expensive (extra API call). Defeats purpose of fast strategy shift hints. |
| Generate alternatives from `tool_search` results | Better but complex. The model can call `tool_search` itself when it reads the hint. |

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Generic guidance may be less actionable than hardcoded `"Use rg"` | Medium — small models may struggle without explicit tool names | Test against `prompt_01`. If small models can't generalize, layer in a "discovered tools" section from the tool registry. |
| Error class coverage may miss edge cases | Low — the `_` catch-all handles unknown errors generically | Monitor for new error classes, add arms as they're discovered |

## Acceptance Criteria

- [ ] `suggest_alternatives()` takes `(strategy: &str, error_class: &str, scope: &str)` instead of just `(failed_strategy: &str)`
- [ ] No string matching on command names — all guidance derived from error class and scope
- [ ] `strategy