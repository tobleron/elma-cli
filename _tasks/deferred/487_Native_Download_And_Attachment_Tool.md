# Task 487: Native Download And Attachment Tool

**Status:** Pending
**Priority:** MEDIUM
**Estimated effort:** 2-3 days
**Dependencies:** Task 485, Task 459
**References:** source-agent parity: Crush download, Hermes media/file delivery

## Objective

Add a native download/artifact tool for controlled retrieval and local artifact creation, with network disabled by default and offline artifact handling available.

## Implementation Plan

1. Add a `download` tool declaration in `elma-tools`.
2. Support local workspace artifact copy/export without network.
3. When URL download is requested, route through Task 485 fetch/network policy.
4. Enforce max bytes, content type policy, filename normalization, and workspace/session artifact boundaries.
5. Return structured artifact metadata: path, size, source, checksum, truncation flag.

## Verification

```bash
cargo test download
cargo test fetch_policy
cargo test tool_calling
cargo build
```

## Done Criteria

- Offline artifact export works without network.
- URL downloads are disabled unless network tools are enabled.
- Files are stored only in approved session/workspace artifact locations.
- Results include stable metadata useful for later evidence references.

