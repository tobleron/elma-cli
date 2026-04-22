# Task 157: Interactive Permission System

## Summary

Implement an interactive permission system that allows users to approve or deny dangerous operations (shell execution, file writes, etc.) with session-based allowlisting.

## Motivation

Elma needs user consent for potentially destructive operations:
- Shell command execution
- File writes and edits
- Network requests
- System modifications

This provides safety without blocking normal workflows.

## Source

Crush's permission package at `_stress_testing/_crush/internal/permission/permission.go`

## Implementation

### Types

```go
type CreatePermissionRequest struct {
    SessionID   string `json:"session_id"`
    ToolCallID  string `json:"tool_call_id"`
    ToolName    string `json:"tool_name"`
    Description string `json:"description"`
    Action      string `json:"action"`
    Params      any    `json:"params"`
    Path        string `json:"path"`
}

type PermissionRequest struct {
    ID          string `json:"id"`
    SessionID   string `json:"session_id"`
    ToolCallID  string `json:"tool_call_id"`
    ToolName    string `json:"tool_name"`
    Description string `json:"description"`
    Action      string `json:"action"`
    Params      any    `json:"params"`
    Path        string `json:"path"`
}

type PermissionNotification struct {
    ToolCallID string `json:"tool_call_id"`
    Granted    bool   `json:"granted"`
    Denied     bool   `json:"denied"`
}
```

### Service Interface

```go
pub Subscriber[PermissionRequest]
GrantPersistent(permission PermissionRequest)
Grant(permission PermissionRequest)
Deny(permission PermissionRequest)
Request(ctx context.Context, opts CreatePermissionRequest) (bool, error)
AutoApproveSession(sessionID string)
SetSkipRequests(skip bool)
SkipRequests() bool
SubscribeNotifications(ctx context.Context) <-chan pubsub.Event[PermissionNotification]
```

### Features

1. **Allowlist** - Tool:action or tool-only patterns that auto-approve
2. **Session Auto-Approve** - Mark sessions as trusted once
3. **Path-based** - Permissions track the file path being operated on
4. **Persistent Grants** - Remember approval within a session
5. **Non-blocking for allowed tools** - Allowlist checked first

### Integration Points

- Shell tool: Request permission before executing commands
- Edit tool: Request permission for writes
- Tool descriptors can specify required permission level
- UI subscribes to permission notifications

### Configuration

```toml
[permission]
skip = false  # Never skip, always ask
allowed_tools = ["read:read", "glob:glob", "search:search"]
```

## Verification

- Permission requests triggered for shell execution
- Allowlist allows tools without prompting
- Session auto-approve works
- Persistent grants persist within session

## Dependencies

- PubSub (from task 158 or existing implementation)
- UI notification system
- Session management

## Notes

- Different from simple confirm() dialogs - this tracks tool+action+path
- Pub/sub-driven for UI integration
- Allows both one-time and persistent grants