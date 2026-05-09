# Task 673: Cross Platform Portability Gate

**Status:** pending
**Priority:** HIGH
**Type:** Test Coverage / Portability
**Scope:** `src/dirs.rs`, `src/persistent_shell.rs`, `src/execution_profiles.rs`, `src/program_utils.rs`, CI/scripts
**Source:** deferred task 476, postponed task 073, user requirement: every condition and platform

## Summary

Add a portability gate covering macOS, Linux, Windows/Powershell path and command behavior, terminal capability differences, and filesystem semantics.

## Evidence And Gap

- The current environment notes warn about macOS BSD command differences.
- `persistent_shell.rs` has powershell branches but needs platform-specific tests.
- Enterprise-grade CLI behavior requires deterministic platform handling.

## Implementation Plan

1. Add portability checks for path normalization, newline handling, executable lookup, shell quoting, temp dirs, permissions, and terminal capability.
2. Add platform-specific unit tests with cfg gates and fake shell/parser fixtures where real platforms are unavailable.
3. Document unsupported features with clear runtime messages.
4. Integrate into release verification.

## Acceptance Criteria

- [ ] Platform assumptions are explicit and tested.
- [ ] macOS/Linux/Windows command differences do not corrupt shell policy or path validation.
- [ ] Unsupported platform features fail with structured messages.
- [ ] Portability gate is part of masterplan verification.

## Verification Plan

Run platform unit tests locally and CI matrix where available.

