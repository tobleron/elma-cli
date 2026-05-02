# Task 048: Add FETCH Step Type (DISABLED for Security)

## Backlog Reconciliation (2026-05-02)

Superseded by Task 485 for current tool-calling architecture. Do not reintroduce a DSL FETCH step; preserve disabled-by-default, security-gated behavior.


## Priority
**P3 - LOW** (Future capability, must be disabled)

## Problem
Elma has no way to access external information (web, APIs, docs). This limits her ability to:
- Look up documentation
- Check package versions
- Access external APIs
- Download resources

However, internet access introduces **CRITICAL security risks**:
- Credential leakage
- Malicious content download
- Rate limiting/abuse
- No sandboxing

## Objective
Add `Step::Fetch` type with **compile-time disable** and security warnings. Implementation exists but is never called.

## Security Constraints (NON-NEGOTIABLE)

1. **DISABLED BY DEFAULT** - Must be explicitly enabled via config
2. **COMPILE-TIME WARNING** - Rust deprecation warning on any use
3. **PANIC ON EXECUTE** - Runtime panic if somehow called
4. **SECURITY AUDIT REQUIRED** - Must pass security review before enabling

## Implementation

### Step Type (`src/types_core.rs`)

```rust
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub(crate) enum Step {
    // NEW: External access (DISABLED)
    #[serde(rename = "fetch")]
    #[deprecated(
        note = "FETCH operation is DISABLED for security. \
                Internet access requires sandboxing, credential handling, \
                rate limiting, and content validation. \
                See security audit #FETCH-001 before enabling."
    )]
    Fetch {
        id: String,
        url: String,            // URL to fetch
        method: String,         // GET/POST/etc
        headers: HashMap<String, String>,  // Request headers
        purpose: String,
        common: StepCommon,
    },
    
    // ...existing types
}
```

### Execution Handler (`src/execution_steps_fetch.rs`)

```rust
//! @efficiency-role: service-orchestrator
//!
//! FETCH Step Execution - DISABLED FOR SECURITY
//!
//! WARNING: This module is intentionally disabled.
//! Internet access requires:
//! - Sandboxed execution environment
//! - Credential management (no hardcoded secrets)
//! - Rate limiting and abuse prevention
//! - Content validation (no malicious downloads)
//! - User consent for external access
//!
//! DO NOT ENABLE without security audit #FETCH-001.

use crate::*;

#[deprecated(
    note = "FETCH execution is DISABLED. See security audit #FETCH-001."
)]
pub(crate) async fn handle_fetch_step(
    _session: &SessionPaths,
    _url: &str,
) -> Result<StepResult> {
    // INTENTIONAL: This should never be reached
    panic!(
        "FETCH operation executed despite being disabled. \
         This indicates a critical security bypass. \
         See security audit #FETCH-001."
    );
}
```

### Execution Integration (`src/execution.rs`)

```rust
#[allow(deprecated)]  // Allow deprecated match arm
for step in program.steps {
    match step {
        Step::Fetch { url, .. } => {
            // This will panic - intentional security measure
            #[allow(deprecated)]
            handle_fetch_step(&session, &url).await?;
        }
        // ...existing
    }
}
```

### Orchestrator Prompt Update

```toml
# In workflow_planner system prompt
# ADD:
"""
Available step types:
- read: Read a file (read-only)
- search: Search for content
- shell: Execute a command
- edit: Create/modify/delete a file
- fetch: Access external URL (DISABLED - do not use)
- reply: Respond to user

NEVER use fetch - it is disabled for security.
"""
```

## Acceptance Criteria
- [ ] `Step::Fetch` type added with `#[deprecated]` attribute
- [ ] Execution handler panics if called
- [ ] Compiler warning on any fetch-related code
- [ ] Orchestrator prompt explicitly says "DISABLED"
- [ ] Unit test confirms panic on execution
- [ ] Security warning in module documentation

## Expected Impact
- **Future capability** - Ready when security audit passes
- **Zero risk** - Disabled by design
- **Clear semantics** - External access is explicit, not hidden in Shell

## Dependencies
- None (additive, disabled)

## Verification
- `cargo build` - Should show deprecation warnings
- `cargo test` - Test confirms panic
- Code review - Confirm no path to execution without panic

## Security Notes

### Before Enabling (Future Work)
1. **Sandboxing** - Run fetch in isolated environment
2. **Credential Management** - No hardcoded secrets, use secret manager
3. **Rate Limiting** - Prevent abuse (max N requests/minute)
4. **Content Validation** - Scan downloads for malware
5. **User Consent** - Explicit approval for external access
6. **Audit Logging** - Log all fetch requests
7. **Domain Whitelist** - Only allow approved domains
8. **Timeout** - Prevent hanging requests

### Security Audit #FETCH-001 Checklist
- [ ] Sandboxed execution implemented
- [ ] Credential management reviewed
- [ ] Rate limiting tested
- [ ] Content validation implemented
- [ ] User consent flow added
- [ ] Audit logging verified
- [ ] Domain whitelist configured
- [ ] Timeout handling tested

## Architecture Alignment
- ✅ Articulate terminology (Fetch clearly defined as external)
- ✅ Security-first design (disabled by default)
- ✅ Future-proof (ready when audit passes)
