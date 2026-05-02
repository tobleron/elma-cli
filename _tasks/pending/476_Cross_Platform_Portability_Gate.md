# Task 476: Cross Platform Portability Gate

**Status:** pending
**Priority:** MEDIUM
**Source:** 2026-05-02 full codebase audit
**Related:** postponed Task 073, pending Task 475

## Summary

Add a portability audit for Unix-only APIs, hardcoded temporary paths, shell assumptions, and platform-specific command behavior.

## Evidence From Audit

- `exec_observe` uses `std::os::unix::fs::MetadataExt` in the main code path.
- Several tests use `/tmp` directly.
- Shell preflight and command repair assume POSIX-style shell syntax.
- Tool discovery scans Unix-like paths such as `/usr/local/bin`, `/usr/bin`, `/bin`, and `/opt/homebrew/bin`.
- `trash`, `portable-pty`, and filesystem metadata behavior may differ across platforms.

## User Decision Gate

Ask the user which platforms matter for the next release:

- macOS only.
- macOS and Linux.
- macOS, Linux, and Windows.

Scope the portability gate to that answer.

## Implementation Plan

1. Inventory platform-specific APIs and paths.
2. Add cfg-gated abstractions for metadata, temp dirs, shell syntax, and PATH scans.
3. Replace `/tmp` tests with `tempfile`.
4. Add a no-network portability check to Task 475 release gate.
5. Document unsupported platforms clearly if the user chooses macOS-only.

## Success Criteria

- [ ] The selected platform set is documented.
- [ ] Platform-specific code is cfg-gated or abstracted.
- [ ] Tests avoid hardcoded Unix temp paths unless explicitly platform-gated.
- [ ] Release audit flags portability-sensitive changes.

## Anti-Patterns To Avoid

- Do not pretend Windows support exists if it is not tested.
- Do not remove POSIX shell hardening needed for current users.
- Do not add CI requirements that cannot run offline/local.
