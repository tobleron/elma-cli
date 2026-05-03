# Task 548: Fix Cargo.toml Leading-Space Formatting

**Status:** pending
**Priority:** LOW
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P14 — Medium Confidence (confirmed by reading file)

## Summary

Two dependency entries in `Cargo.toml` have a leading space character, violating TOML manifest formatting conventions. While `cargo` tolerates this at parse time, it is a hygiene defect that causes inconsistent formatting and can confuse manifest-editing tools like `toml_edit`.

## Evidence

Cargo.toml lines (as read during session):
```toml
 flate2 = "1.0"
 tar = "0.4"
```
Both have a leading space before the key name. All other dependency lines do not.

## Implementation Plan

1. Open `Cargo.toml`
2. Remove the leading space from the `flate2` line
3. Remove the leading space from the `tar` line
4. Run `cargo build` to verify no regression
5. Commit: `fix: remove leading spaces from flate2 and tar entries in Cargo.toml`

## Success Criteria

- [ ] `flate2` and `tar` lines have no leading whitespace
- [ ] `cargo build` succeeds after the edit
- [ ] No other manifest entries have leading spaces (verify with `grep '^ [a-z]' Cargo.toml`)

## Verification

```bash
grep '^ [a-z]' Cargo.toml   # should return no output after fix
cargo build
```
