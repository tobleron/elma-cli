# 629 Move System Monitor Off Render Thread

## Summary
`system_monitor::get_snapshot()` spawns 3 subprocesses on macOS from inside the render pipeline. Even with 1s caching, the first call each second blocks the render thread during fork+exec.

## Affected Files
- `src/system_monitor.rs:78` — macOS: spawns `sysctl` (×2) and `vm_stat` subprocesses
- `src/system_monitor.rs:24` — 1-second cache TTL but Mutex lock per call
- `src/claude_ui/claude_render.rs:1934` — `get_snapshot()` called from `render_right_panel_info` every frame

## Current Behavior
- Every frame: `system_monitor::get_snapshot()` locks a Mutex, checks 1s TTL
- If TTL expired: spawns 3 subprocesses (`sysctl -n hw.memsize`, `sysctl -n vm.pagesize`, `vm_stat`), blocks render thread
- Linux: reads `/proc/meminfo` and `/proc/loadavg` instead of subprocesses (less blocking, but still I/O)

## Proposed Fix
- Spawn a background thread that refreshes snapshot once per second
- Publish snapshot via `Arc<AtomicU64>` or `parking_lot::RwLock<MemorySnapshot>`
- Render thread reads pre-computed snapshot without blocking
- Keep 1s refresh, but decoupled from render pipeline
- Linux: use a background tokio task with `tokio::fs::read_to_string` for /proc reads

## Estimated CPU Savings
Eliminates render-thread blocking spikes; negligible sustained CPU change

## Status
PENDING
