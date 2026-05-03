# Task 485: Web Fetch Tool Security-Gated HTTP

**Status:** pending
**Priority:** high
**Primary surfaces:** `elma-tools/src/tools/fetch.rs`, `src/tool_calling.rs`, `src/permission_gate.rs`, `src/types_core.rs`
**Related tasks:** completed Task 339 (tool metadata policy), Task 488 (browser observation), Task 459 (sandboxed execution profiles)

## Objective

Finish the `fetch` tool as a secure, permission-gated HTTP text retrieval tool. It must preserve Elma's offline-first behavior, block SSRF/private-network access, cap resource usage, and avoid becoming a hidden browser or downloader.

## Current Code Reality

- `elma-tools/src/tools/fetch.rs` already registers a non-deferred `fetch` tool.
- `src/tool_calling.rs::execute_tool_call` does not execute `fetch`; a model calling it receives `Unknown tool: fetch`.
- Root `Cargo.toml` already includes `reqwest`, `url`, `html2text`, `tokio-stream`, and `futures`.
- There is no CLI or config flag that explicitly enables network tools.
- `permission_gate.rs` is command-oriented and must be generalized or wrapped for non-shell tool permissions.
- `src/prompt_core.rs` already mentions `fetch`; do not modify it for this task.

## Design Requirements

### Offline-First Default

`fetch` must not be silently available as a network-capable tool.

Add one explicit enablement path:

- CLI flag such as `--allow-network-tools`, or
- config/env setting such as `ELMA_ALLOW_NETWORK_TOOLS=true`

Without explicit enablement, `fetch` must either be hidden by registry availability metadata or return a clear disabled result before network resolution.

### Tool Schema

Fix the existing schema so only `url` is required. `format` must have a default of `text`.

Supported inputs:

- `url`: required string
- `timeout`: optional integer seconds, default 30, max 120
- `format`: optional enum `text`, `markdown`, `html`, default `text`
- `max_bytes`: optional integer, default 100000, hard max 250000

### Security Policy

Add a module such as `src/fetch_policy.rs` with pure validation functions and tests.

Required URL protections:

- allow only `http` and `https`
- reject empty host
- reject userinfo credentials in URL
- reject localhost names
- reject private, loopback, link-local, multicast, documentation, and unspecified IP ranges
- resolve DNS before connecting and validate every resolved IP
- validate every redirect target with the same policy
- cap redirect count at 5
- default allowed ports: 80, 443, 8080, 8443
- reject URLs longer than 2048 bytes

DNS rebinding protection must validate the final socket target used for the request, not just the original hostname. If `reqwest` cannot expose that directly, use a custom resolve/connect strategy or conservatively disable redirects until safe validation is implemented.

### Permission Gate

Network access must be independently permissioned from shell commands.

Add a generic permission request path or a `check_tool_permission` wrapper that supports:

- tool name
- target URL/domain
- risk label
- once-per-session approval cache by normalized origin
- non-interactive behavior
- safe mode behavior

Default behavior:

- safe mode `On`: ask or deny if non-interactive
- safe mode `Ask`: ask unless origin is explicitly allowed by config
- safe mode `Off`: still block private-network and invalid URLs

### Fetch Execution

Add `exec_fetch` in `src/tool_calling.rs` or a dedicated `src/fetch_tool.rs` module.

Execution requirements:

- use the existing `reqwest::Client` passed into `execute_tool_call` where feasible
- set request timeout from validated input
- stream response bytes and stop at `max_bytes`
- inspect content type before reading large bodies when the header exists
- reject non-text content types
- reject binary-looking bodies even if content type lies
- decode UTF-8 losslessly when possible, otherwise return a clear unsupported encoding message
- convert HTML to text or markdown with `html2text` for non-`html` output
- do not persist fetched content unless later evidence storage already stores tool results under normal session rules

### Result Format

Successful output must include:

- final URL after redirects
- HTTP status
- content type
- bytes read
- truncation flag
- body text

Failure output must identify the blocked policy, not just `request failed`.

## Implementation Steps

1. Add explicit network-tool enablement to args/config.
2. Fix `elma-tools/src/tools/fetch.rs` schema and availability behavior.
3. Add pure URL/IP/content-type validation in `src/fetch_policy.rs`.
4. Add generic network permission gating.
5. Add `fetch` executor wiring in `src/tool_calling.rs`.
6. Add streaming response truncation and HTML conversion.
7. Use completed Task 339 metadata: network, read-only, not workspace-filesystem, not concurrency-safe unless permission cache proves no prompt is needed.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools fetch
cargo test fetch_policy
cargo test tool_calling
cargo test permission_gate
cargo build
```

Use local test servers only. Do not require internet access in tests.

Required coverage:

- fetch disabled by default
- explicit enablement allows validation to proceed
- invalid URL fails
- blocked scheme fails
- URL with credentials fails
- localhost hostname fails
- loopback IP fails
- private IPv4 fails
- private IPv6 fails
- blocked port fails
- DNS failure returns clear error
- redirect to private IP fails
- redirect loop fails
- text content succeeds
- HTML to text succeeds
- HTML raw output succeeds for `format=html`
- JSON succeeds as text
- non-text content type fails
- binary-looking body fails
- response larger than `max_bytes` truncates without buffering full body
- timeout returns clear error
- non-interactive permission denial is deterministic
- `execute_tool_call` no longer returns `Unknown tool: fetch` when enabled

Manual probes:

```bash
rg -n '"fetch" =>|Unknown tool: fetch|allow-network|fetch_policy' src elma-tools/src
cargo test fetch_policy -- --nocapture
```

## Done Criteria

- All verification commands pass.
- No test depends on external internet.
- Fetch is disabled unless explicitly enabled.
- Private-network and local-file access are blocked regardless of user permission.
- Source prompt remains untouched.

## Anti-Patterns

- Do not use shell commands such as `curl` or `wget`.
- Do not follow redirects without validating every target.
- Do not buffer the full response before enforcing size limits.
- Do not execute JavaScript.
- Do not allow `file://`, `data:`, `ftp:`, or browser-only schemes.
- Do not treat safe mode `Off` as permission to access private networks.
