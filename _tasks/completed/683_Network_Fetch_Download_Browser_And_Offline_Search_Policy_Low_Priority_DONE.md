# Task 683: Network Fetch Download Browser And Offline Search Policy Low Priority

**Status:** pending
**Priority:** LOW
**Type:** Optional Network Feature / Security
**Scope:** `src/tool_calling.rs`, `src/execution_profiles.rs`, future fetch/browser/download modules
**Source:** deferred tasks 485-488, postponed tasks 006/075/274/275

## Summary

Keep network fetch, download/attachment, browser observation, and web search behind explicit security gates; prioritize offline search first.

## Evidence And Gap

- AGENTS.md says offline-first and web access is secondary.
- Existing fetch-related tasks are valuable but lower priority than local architecture and safety.
- Any network feature must avoid silently changing Elma’s local-first trust model.

## Implementation Plan

1. Define network policy states: disabled, localhost-only, prompt, allowlisted, unrestricted.
2. Build offline search/provider behavior before web search.
3. Gate fetch/download/browser tools by policy, user approval, size limits, content-type limits, and artifact storage.
4. Persist URL, policy decision, hashes, and redaction in session artifacts.

## Acceptance Criteria

- [ ] Network tools are disabled by default.
- [ ] Offline search works without internet.
- [ ] Every network access is transcript-visible and policy-backed.
- [ ] Downloads are size/hash/content-type bounded.

## Verification Plan

Use local HTTP fixtures; do not require external internet.

