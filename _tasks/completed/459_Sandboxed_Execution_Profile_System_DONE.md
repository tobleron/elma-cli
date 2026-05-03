# Task 459: Sandboxed Execution Profile System

**Status:** pending
**Source patterns:** OpenHands Docker sandbox, Qwen-code sandbox command, Hermes environment backends
**Depends on:** completed Task 325 (shell hardening), completed Task 339 (tool metadata policy)

## Summary

Add configurable execution profiles for shell and code-running tools: local default, restricted workspace, containerized, and future remote backends. Profiles must remain local-first and permission-gated.

## Why

Elma has shell preflight and permission checks, but reference agents improve reliability and safety by separating execution environments from agent logic. A profile system can support stricter sandboxes without forcing every user into Docker.

## Implementation Plan

1. Define execution profile metadata in config.
2. Route shell/code execution through a profile abstraction.
3. Start with current local execution as the default profile.
4. Add a restricted workspace profile before adding container support.
5. Emit profile selection and escalation decisions as visible events.

## Success Criteria

- [ ] Existing local shell behavior remains compatible.
- [ ] Restricted profile prevents writes outside configured roots.
- [ ] Container profile is optional and unavailable gracefully when dependencies are missing.
- [ ] Permission gate still controls risky operations.
- [ ] Tests cover profile selection and blocked writes.

## Anti-Patterns To Avoid

- Do not require Docker for normal Elma use.
- Do not bypass shell preflight inside a sandbox.
- Do not hide profile selection in debug logs only.
