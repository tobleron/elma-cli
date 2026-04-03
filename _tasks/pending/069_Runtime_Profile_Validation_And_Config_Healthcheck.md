# Task 069: Runtime Profile Validation And Config Healthcheck

## Priority
**P1 - OPERATIONAL HARDENING**

## Objective
Add startup validation for profiles, prompts, grammars, and related config so Elma fails early and clearly when the runtime configuration is inconsistent.

## Why This Exists
Recent CLI issues showed that config shape mismatches and profile drift can break startup or create hidden runtime instability. A premium-quality local product needs immediate, explainable config validation.

## Scope
- Validate profile schema compatibility at startup.
- Detect non-profile TOML files being loaded as profiles.
- Validate grammar mappings against live profile expectations.
- Validate managed prompt sync coverage.
- Report actionable startup diagnostics instead of late failures.
- Persist a startup health summary in the session when useful.

## Deliverables
- A runtime config healthcheck path.
- Early validation errors with clear explanations.
- Tests covering mixed-schema and missing-asset failures.

## Acceptance Criteria
- Bad config states fail early and clearly.
- Healthy startup produces a concise validation summary.
- `cargo build` and `cargo test` remain green.
