# Task 713: Endpoint Probe Profile Persistence And Provider UI Contract

## Type

Provider Runtime / Configuration UX

## Severity

Medium

## Evidence

Round 5 retry confirms the new runtime probe is visible in session trace:

- `endpoint_probe_ok discovered_model=Huihui-Qwen3.5-4B-Claude-4.6-Opus-abliterated.Q6_K.gguf ctx_max=262144`
- `model_capability_probe provider=openai_compatible kind=thinking thinking=true json_mode=true ctx_max=262144`

The implementation currently keeps the probe mostly in memory and traces it, but it should become a persisted provider profile used by startup, `/provider`, diagnostics, and future dense-model adaptation.

## Problem

Elma should not ask users to configure model ID, context length, or model type. It should discover and persist those endpoint facts. The `/provider` dialog should ask only for main endpoint and optional helper endpoint, then discover models and capabilities itself.

## Requirements

- Persist a provider runtime profile under the model config folder or runtime config.
- Include endpoint URL, discovered model ID, context window, provider family, runtime kind, supports thinking, supports JSON mode, probe timestamp, and probe source.
- Ensure `/provider` only asks for main endpoint and optional helper endpoint.
- Discover helper model ID from the helper endpoint instead of asking the user for it.
- Make `config show` and `config doctor` report discovered model facts and stale probe status.
- Avoid requiring users to set `provider.model` for llama.cpp-compatible endpoints.

## Acceptance Criteria

- [ ] Starting Elma with only an endpoint discovers model ID and context length.
- [ ] `/provider` does not ask for model ID.
- [ ] Trace and config/doctor output agree on discovered model facts.
- [ ] Future dense-model testing can read a persisted runtime kind instead of relying on manual config.

