# Task 130: Always-On Service Management

## Priority
**Postponed (Tier B — Future Capability)**
**Created:** 2026-04-05
**Status:** Postponed
**Depends on:** Tasks 126 (Daemon Mode), 127 (Telegram Bot), 129 (Background Sessions)

## Overview

Run Elma daemon as a persistent system service that survives reboots and terminal closures. macOS launchd / Linux systemd integration.

## Scope

### 1. `elma service` Subcommand
- `elma service install` — create launchd plist (macOS) or systemd service (Linux)
- `elma service start` / `stop` / `restart` / `status` / `uninstall`
- Service runs as current user, not root
- Logs to `~/.elma/daemon.log`

### 2. Service Configuration
- Config file location: `~/.elma/service.toml`
- Configurable: port, channels enabled, auto-start on boot
- `auto_start: true` by default after `service install`

### 3. Health Monitoring
- Service health check via `GET /health` endpoint
- Auto-restart on crash (launchd/systemd handles this)
- `elma service status` shows: running/stopped, uptime, active sessions, last error

### 4. Integration Points
- `src/service.rs` (new) — service management commands
- Template files: `config/elma.plist` (macOS), `config/elma.service` (Linux)
- `src/app.rs` — `service` subcommand group

### 5. Design Constraints
- **No Docker** — runs as native system service
- **Single instance** — lock file prevents duplicate daemons
- **User-level** — no root/sudo required

## Estimated Effort
~300 lines + 2 template files. 1 day focused work.

## Verification
1. `cargo build` clean
2. `elma service install && elma service start` → daemon running
3. Reboot → daemon auto-starts, Telegram bot responds
4. `elma service status` → shows uptime, active sessions
5. Kill daemon process → auto-restarts within 10 seconds
