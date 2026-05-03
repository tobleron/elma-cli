# Task 543: Patch Verification Gate Before Proposal

**Status:** pending
**Priority:** MEDIUM
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P7 — High Confidence

## Summary

During the session, Elma proposed a patch for `src/permission_gate.rs` lines 227-229 that referenced `modal_system.is_active()`. The model's own next-step checklist (item 3) admitted it had not verified whether `modal_system` is a valid variable in that scope. The patch was never applied or validated with `cargo check`. The user received a concrete, line-numbered code patch that may not compile.

## Evidence

- `session.md` lines 168-183: patch proposal with `modal_system.is_active()` call
- `session.md` lines 203-208: next-step checklist item 3: "Search for modal_system usage pattern to confirm integration point exists"
- No `cargo check` or `cargo build` output present in session transcript
- Validation command listed as a future step, never executed

## Root Cause

The analysis prompt asked the model to "create a minimal patch only if you can prove it is safe." The model bypassed the safety gate — it proposed the patch before verifying the patch target, then listed the verification as a future step. The system has no enforcement mechanism to ensure a proposed patch was actually verified before being shown to the user.

## Implementation Plan

1. In the audit/analysis intel unit or session prompt, enforce a strict pre-patch checklist:
   - `[ ]` Confirm the target file and line range exist via `read` or `stat`
   - `[ ]` Confirm all referenced symbols (variables, methods, types) exist in scope via `search`
   - `[ ]` Apply the patch and run `cargo check` (not `cargo build --release`) before presenting
2. If any pre-patch step fails, the model must NOT present the patch — instead present a "patch blocked: <reason>" note
3. Add to the analysis prompt: _"Do not propose a patch unless you have confirmed it compiles. Run cargo check immediately after applying any edit."_
4. Consider adding a `patch_verified: bool` field to the risk report output schema so unverified patches are labeled as `[UNVERIFIED DRAFT]`

## Success Criteria

- [ ] No patch is shown to the user without a corresponding `cargo check` success in the transcript
- [ ] Unverified patches are clearly labeled as drafts
- [ ] The model does not list "verify the patch" as a future step after already showing the patch

## Verification

```bash
cargo build
# Run an audit session and request a minimal patch
# Verify cargo check runs before patch is shown in final answer
```
