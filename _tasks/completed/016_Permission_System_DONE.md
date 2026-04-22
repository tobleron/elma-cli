# 152 Permission System

## Summary
Implement permission approval system for dangerous operations.

## Reference
- OpenCode: `internal/permission/permission.go`

## Implementation

### 1. Permission Types
File: `src/permission.rs` (new)
```rust
pub enum Permission {
    Read(String),       // Read file
    Write(String),     // Write file
    Execute(String),  // Run command
    Delete(String),   // Delete file
    Network(String),  // Network request
}

pub enum Decision {
    Allow,
    AllowSession,  // Remember for session
    Deny,
}
```

### 2. Permission Request
```rust
pub struct PermissionRequest {
    pub tool: String,
    pub action: Permission,
    pub details: String,
    pub deadline: Option<tokio::time::Duration>,
}
```

### 3. Permission Service
File: `src/permission/service.rs` (new)
- `request(perm) -> Decision` - blocks until decision
- `cached(session_id) -> Option<Decision>` - session cache
- Check auto-approve for non-interactive mode

### 4. Integration
File: `src/tools/mod.rs`
- Each tool calls `permission.request()` before execution
- Return error if denied

### 5. UI Dialog
File: `src/ui/permission_modal.rs` (new)
- Show permission dialog in TUI
- Options: Allow, Allow always, Deny

## Verification
- [ ] `cargo build` passes
- [ ] Permission blocks tool execution
- [ ] Caching works