# Task 728: Shell Idle Timeout Recovery Should Split Compound Verification Commands

## Type

Shell Execution / Verification Robustness / Offline Autonomy

## Severity

Medium

## Evidence

Round 8 prompt 08 completed the backup safely, but the model then issued a compound shell verification command:

```text
echo "=== Source directory file count ==="; find . -type f | wc -l; echo "=== Backup directory file count ==="; ls project_tmp/backup_20260507_130459/ | wc -l
```

The shell tool failed:

```text
Shell command idle timeout after 30s of no output (30s total) — the command may be stalled. Try a safer approach.
```

Session:

- `sessions/s_1778148239_66899000/session.md`

## Problem

The backup tool already returned `verification_ok=true`, but the model attempted a broad and noisy shell verification anyway. The command was also too broad because `find .` can traverse generated/archive trees. Elma should either trust structured tool verification or repair shell verification into narrow, platform-safe commands.

## Requirements

- Prefer structured backup verification results over redundant shell verification.
- When shell verification is still useful, split compound commands into bounded single-purpose commands.
- Apply workspace default excludes to generated/archive trees for shell preflight suggestions.
- After a shell idle timeout, produce a strategy-shift packet with a safer command, not just a generic error.
- Add tests for compound shell command detection and safer verification alternatives.

## Likely Files

- `src/shell_preflight.rs`
- `src/tool_calling.rs`
- `src/stop_policy.rs`
- `src/tool_repair.rs`
- `src/finalization_verifier.rs`

## Acceptance Criteria

- [ ] Prompt 08 does not run broad `find .` after backup tool verification succeeds.
- [ ] Compound shell verification commands are split or rejected with a safe replacement.
- [ ] Shell idle timeout recovery includes a concrete narrow retry strategy.

