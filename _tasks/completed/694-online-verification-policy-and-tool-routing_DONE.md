# Task 694: Online Verification Policy And Tool Routing

## Type

Tooling

## Severity

Medium

## Scope

System-wide

## Session Evidence

Prompt 03 requested online verification of documentation references:

- `sessions/s_1778084708_633588000/session.md` shows only `workspace_info`, `glob`, `read`, and `shell`/`grep` style actions.
- There is no `fetch`, browser, or network verification tool event.
- The final answer claims external API and library references were current and secure.

## Problem

Elma must not claim online verification without using an online-capable tool, and offline-first policy should explicitly decide whether online work is allowed, skipped, or deferred.

## Root Cause Hypothesis

Confirmed: finalization made online-verification claims without corresponding network evidence.

Likely: route/tool planning does not bind "online verification" requirements to fetch/network tools or an explicit offline refusal/defer path.

## Proposed Solution

Add an online-verification requirement gate:

- Inspect `src/network_policy.rs`, `elma-tools/src/tools/fetch.rs`, `src/tool_calling.rs`, `src/final_answer.rs`, and `src/continuity.rs`.
- Represent online verification as a capability requirement in the work graph or objective criteria.
- If network is disabled by policy, require the final answer to say verification was not performed and optionally create a local-only report.
- If network is enabled, expose/select the fetch tool and verify cited references with captured evidence.
- Add finalization checks that block "verified online" claims without fetch/network evidence.

## Acceptance Criteria

- [ ] Online verification prompts either use a network-capable tool or explicitly disclose offline limitation.
- [ ] Final answers cannot label references current/secure based only on local grep.
- [ ] Session artifacts include URL, status, timestamp, and summarized evidence for each online check.

## Verification Plan

Replay prompt 03 once with network disabled and once with network enabled. Confirm the outputs differ honestly and the enabled run includes fetch evidence.

## Dependencies

Task 690.

