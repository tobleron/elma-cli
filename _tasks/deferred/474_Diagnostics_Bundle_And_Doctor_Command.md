# Task 474: Diagnostics Bundle And Doctor Command

**Status:** pending
**Source patterns:** Goose diagnostics, Aider report flows, Hermes environment diagnostics
**Depends on:** Task 283 (transcript flush), Task 284 (panic hook transcript path)

## Summary

Add a `doctor` and sanitized diagnostics bundle command that packages config health, provider metadata, session transcript, event log, traces, OS/runtime details, and recent errors without secrets.

## Why

Elma has diagnostics modules and session traces, but troubleshooting still requires manual file discovery. Reference agents provide explicit support bundles that make user reports and regression triage much faster.

## Implementation Plan

1. Add `elma-cli doctor` for local checks.
2. Add `elma-cli session bundle <session>` to generate a sanitized archive or directory.
3. Redact secrets, tokens, home-directory-sensitive paths where appropriate.
4. Include config schema validation, provider reachability status, transcript path, event log, and recent panic metadata.
5. Add tests for redaction and missing-file behavior.

## Success Criteria

- [ ] Doctor reports config, provider, session-store, and tool-path health.
- [ ] Bundle generation works without network access.
- [ ] Secrets are redacted deterministically.
- [ ] Bundle includes enough context to reproduce a failed session.
- [ ] Tests cover redaction and corrupt session metadata.

## Anti-Patterns To Avoid

- Do not include raw API keys or credentials.
- Do not require the user to search hidden directories manually.
- Do not upload bundles automatically.
