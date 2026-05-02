# Task 444: Provider Connectivity And Endpoint Hardening

**Status:** pending
**Priority:** CRITICAL
**Source:** 2026-05-02 full codebase audit
**Related:** pending Task 400, pending Task 431, completed Task 278, completed Task 303

## Summary

Harden LLM provider endpoint construction, connectivity checks, auth behavior, and provider-specific request paths.

## Evidence From Audit

- `bootstrap_app` builds `chat_url` with `/v1/chat/completions`.
- `check_endpoint_connectivity` then formats `"{chat_url}/v1/chat/completions"`, likely duplicating the chat path.
- The bootstrap connectivity check posts a raw `ChatCompletionRequest` directly and does not use provider-specific auth headers.
- `UnifiedLlmClient::check_connectivity` has a separate provider-aware connectivity implementation, creating duplicate behavior.
- Provider detection uses base URL and model-family string hints, which is acceptable for provider families but needs tests for configured overrides and local endpoints.

## User Decision Gate

Ask the user whether startup should:

- Require reachable local endpoints but only warn for remote endpoints.
- Skip active connectivity checks until the first real request.
- Use provider-specific health probes when available.

Record the selected behavior before implementation.

## Implementation Plan

1. Replace duplicate connectivity logic with a single provider-aware function.
2. Fix endpoint path composition so paths are not duplicated.
3. Include configured auth headers for remote provider checks without logging secrets.
4. Add tests for localhost, OpenAI-compatible, Anthropic, OpenAI, Groq, and Azure URL construction.
5. Surface connectivity failures as transcript or startup rows with actionable messages.

## Success Criteria

- [ ] Connectivity checks hit the intended endpoint exactly once.
- [ ] Remote provider checks use the same auth/header path as real requests.
- [ ] Error messages do not leak API keys.
- [ ] Local-first behavior matches the user-approved policy.
- [ ] Provider URL tests cover path and trailing-slash combinations.

## Anti-Patterns To Avoid

- Do not add internet dependency to normal tests.
- Do not print auth headers or API keys.
- Do not solve provider routing from arbitrary user-message keywords.
