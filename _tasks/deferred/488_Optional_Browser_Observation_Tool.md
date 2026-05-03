# Task 488: Optional Browser Observation Tool

**Status:** pending
**Priority:** medium
**Primary surfaces:** `elma-tools/src/tools/`, `src/tool_calling.rs`, new `src/browser_observation.rs`
**Depends on:** Task 485 (web fetch), Task 459 (sandboxed execution profiles), completed Task 339 (tool metadata policy)

## Objective

Add an optional browser-backed observation tool for web pages that cannot be inspected with plain HTTP fetch. It must be disabled by default, local-first, permission-gated, resource-bounded, and built behind an abstraction that is testable without launching a real browser.

## Current Code Reality

- There is no browser driver dependency or browser runtime module.
- `fetch` is not executable yet until Task 485 is complete.
- Tool declarations are in `elma-tools`, executors are in `src/tool_calling.rs`.
- Elma's safety model is offline-first and must not start browser/network activity implicitly.
- No task should add browser status to the footer.

## Scope For This Task

Implement observation only:

- load URL
- extract visible text
- return compact DOM outline
- optionally save screenshot artifact metadata when a real driver is configured
- close session

Do not implement form filling, clicking, file upload/download, credential handling, or arbitrary browser automation in this task.

## Design Requirements

### Disabled By Default

Browser observation must require all of:

- network tools enabled from Task 485
- browser tools enabled by config/env/CLI
- a configured browser driver or supported local browser binary
- permission approval for the target origin

Without these, the tool must be hidden by metadata or return a clear disabled result before launching anything.

### Reuse Fetch Security

All URL validation must reuse the policy from Task 485:

- no private IPs
- no localhost
- no file/data/ftp schemes
- redirect validation
- port policy
- URL length cap

Browser observation must not become a workaround around fetch restrictions.

### Driver Abstraction

Add a trait-based boundary:

```rust
#[async_trait::async_trait]
pub(crate) trait BrowserDriver {
    async fn open(&mut self, url: &Url, timeout: Duration) -> Result<BrowserPage>;
    async fn visible_text(&mut self) -> Result<String>;
    async fn dom_outline(&mut self, max_nodes: usize) -> Result<String>;
    async fn screenshot(&mut self, path: &Path) -> Result<()>;
    async fn close(&mut self) -> Result<()>;
}
```

Tests must use a fake driver. Real browser support can be behind a feature flag or runtime discovery.

### Tool Schema

Add a deferred tool, for example `browser_observe`.

Inputs:

- `url`: required
- `mode`: optional enum `text`, `dom`, `screenshot`, `text_dom`, default `text`
- `timeout_ms`: optional, default 5000, max 15000
- `max_text_chars`: optional, default 20000, hard max 50000
- `max_dom_nodes`: optional, default 200, hard max 1000

### Resource Limits

The browser runtime must enforce:

- one browser session per tool call at first
- hard timeout for launch and navigation
- hard timeout for extraction
- child process cleanup on timeout/failure
- screenshot stored only under session artifacts
- no downloads
- no persistent profile unless explicitly configured later

### Result Format

Successful result must include:

- URL
- final URL after redirects
- mode
- elapsed time
- text/dom truncation metadata
- screenshot artifact path when applicable

Failure result must state whether failure happened during policy validation, permission, launch, navigation, extraction, or cleanup.

## Implementation Steps

1. Add browser tool declaration in `elma-tools` as disabled/deferred until executable.
2. Add `src/browser_observation.rs` with policy glue, driver trait, fake driver, and optional real-driver placeholder.
3. Wire `browser_observe` in `src/tool_calling.rs`.
4. Reuse Task 485 URL validation and network permission.
5. Store artifacts under the current session's artifacts directory only.
6. Use metadata from completed Task 339: network, external process, read-only from workspace perspective, not concurrency-safe initially.

## Verification

Required commands:

```bash
cargo fmt --check
cargo test -p elma-tools browser
cargo test browser_observation
cargo test fetch_policy
cargo test tool_calling
cargo build
```

Tests must not require Chrome, Chromium, Playwright, or internet access.

Required coverage:

- browser tool hidden/disabled by default
- network disabled blocks before driver launch
- private URL blocks before driver launch
- fake driver returns visible text
- fake driver returns DOM outline
- fake screenshot writes only inside session artifacts
- navigation timeout cleans up driver
- extraction timeout cleans up driver
- max text truncation is reported
- max DOM node truncation is reported
- permission denial returns deterministic result
- `execute_tool_call` dispatches `browser_observe` when enabled

Optional ignored real-driver probe:

```bash
ELMA_RUN_REAL_BROWSER_TESTS=1 cargo test browser_observation_real -- --ignored
```

This must stay ignored by default.

## Done Criteria

- All non-ignored verification tests pass offline.
- Browser observation cannot run unless explicitly enabled.
- Browser policy is at least as strict as fetch policy.
- Child processes are cleaned up on every failure path covered by tests.
- No footer changes and no prompt-core changes are included.

## Anti-Patterns

- Do not add autonomous clicking or form interaction in this task.
- Do not use browser tools to bypass fetch restrictions.
- Do not require a real browser in CI/default tests.
- Do not keep persistent browser profiles by default.
- Do not store screenshots outside session artifacts.
