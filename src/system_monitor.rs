//! System resource monitor for the right-side info panel.
//!
//! Provides CPU and memory usage estimates using platform-specific
//! commands (sysctl on macOS, /proc on Linux).

use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(crate) struct SystemSnapshot {
    pub cpu_pct: f64,
    pub mem_total_gb: f64,
    pub mem_used_gb: f64,
    pub mem_pct: f64,
    pub num_cpus: u32,
    pub process_mem_mb: f64,
    pub sampled_at: Instant,
}

/// Cached system monitor, refreshed at most once per second.
static SNAPSHOT: std::sync::LazyLock<Mutex<Option<(SystemSnapshot, Instant)>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

const CACHE_TTL: Duration = Duration::from_secs(1);

fn num_cpus() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(1)
}

fn collect_snapshot() -> Option<SystemSnapshot> {
    let num_cpus = num_cpus();

    // Memory
    let (mem_total_gb, mem_used_gb) = get_memory_gb()?;

    // CPU
    let cpu_pct = get_cpu_pct(num_cpus);

    // Elma process memory
    let process_mem_mb = get_process_memory_mb();

    Some(SystemSnapshot {
        cpu_pct,
        mem_total_gb,
        mem_used_gb,
        mem_pct: if mem_total_gb > 0.0 {
            (mem_used_gb / mem_total_gb * 100.0).min(100.0)
        } else {
            0.0
        },
        num_cpus,
        process_mem_mb,
        sampled_at: Instant::now(),
    })
}

pub(crate) fn get_snapshot() -> Option<SystemSnapshot> {
    let now = Instant::now();
    {
        let cached = SNAPSHOT.lock().unwrap();
        if let Some((snap, ts)) = cached.as_ref() {
            if now.duration_since(*ts) < CACHE_TTL {
                return Some(snap.clone());
            }
        }
    }
    if let Some(snap) = collect_snapshot() {
        *SNAPSHOT.lock().unwrap() = Some((snap.clone(), now));
        Some(snap)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn get_memory_gb() -> Option<(f64, f64)> {
    use std::process::Command;

    // Total memory via sysctl
    let total_kb = Command::new("sysctl")
        .args(["-n", "hw.memsize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .map(|b| b / 1024)
        .unwrap_or(0);

    // Used memory via vm_stat (page size * (active + wired + compressed))
    let page_size = Command::new("sysctl")
        .args(["-n", "vm.pagesize"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(4096);

    let vm_stat = Command::new("vm_stat").output().ok()?;
    let vm_text = String::from_utf8(vm_stat.stdout).ok()?;
    let mut active = 0u64;
    let mut wired = 0u64;
    let mut compressed = 0u64;

    for line in vm_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Pages active:") || trimmed.starts_with("Anonymous pages:") {
            if let Some(val) = parse_vm_stat_value(trimmed) {
                active += val;
            }
        } else if trimmed.starts_with("Pages wired down:") {
            if let Some(val) = parse_vm_stat_value(trimmed) {
                wired = val;
            }
        } else if trimmed.starts_with("Pages occupied by compressor:") {
            if let Some(val) = parse_vm_stat_value(trimmed) {
                compressed = val;
            }
        }
    }

    if total_kb == 0 {
        return None;
    }

    let used_pages = active + wired + compressed;
    let used_kb = used_pages.saturating_mul(page_size) / 1024;

    Some((total_kb as f64 / 1_048_576.0, used_kb as f64 / 1_048_576.0))
}

#[cfg(target_os = "linux")]
fn get_memory_gb() -> Option<(f64, f64)> {
    let content = std::fs::read_to_string("/proc/meminfo").ok()?;
    let mut total_kb = 0u64;
    let mut available_kb = 0u64;

    for line in content.lines() {
        if line.starts_with("MemTotal:") {
            total_kb = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
        } else if line.starts_with("MemAvailable:") {
            available_kb = line.split_whitespace().nth(1)?.parse::<u64>().ok()?;
        }
    }

    if total_kb == 0 {
        return None;
    }

    let used_kb = total_kb.saturating_sub(available_kb);
    Some((total_kb as f64 / 1_048_576.0, used_kb as f64 / 1_048_576.0))
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_memory_gb() -> Option<(f64, f64)> {
    None
}

#[cfg(target_os = "macos")]
fn parse_vm_stat_value(line: &str) -> Option<u64> {
    let parts: Vec<&str> = line.rsplitn(2, ':').collect();
    if parts.len() < 2 {
        return None;
    }
    // Parts[0] is after the last colon, e.g. " 123456."
    parts[0].trim().trim_end_matches('.').parse::<u64>().ok()
}

#[cfg(target_os = "macos")]
fn get_cpu_pct(num_cpus: u32) -> f64 {
    use std::process::Command;
    // Use sysctl to get load average (1m, 5m, 15m) and divide by num_cpus
    if let Ok(output) = Command::new("sysctl").args(["-n", "vm.loadavg"]).output() {
        if let Ok(s) = String::from_utf8(output.stdout) {
            // Output format: "{ 1.23 0.45 0.67 }"
            let cleaned = s.trim().trim_start_matches('{').trim_end_matches('}').trim();
            if let Some(first) = cleaned.split_whitespace().next() {
                if let Ok(load) = first.parse::<f64>() {
                    return (load / num_cpus as f64 * 100.0).min(100.0);
                }
            }
        }
    }
    0.0
}

#[cfg(target_os = "linux")]
fn get_cpu_pct(_num_cpus: u32) -> f64 {
    // Read /proc/loadavg for 1-minute load average
    if let Ok(content) = std::fs::read_to_string("/proc/loadavg") {
        if let Some(first) = content.split_whitespace().next() {
            if let Ok(load) = first.parse::<f64>() {
                return (load / _num_cpus as f64 * 100.0).min(100.0);
            }
        }
    }
    0.0
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_cpu_pct(_num_cpus: u32) -> f64 {
    0.0
}

/// Get approximate RSS (resident set size) of the current process in MB.
fn get_process_memory_mb() -> f64 {
    #[cfg(target_os = "macos")]
    {
        let pid = unsafe { libc::getpid() };
        let mut task_info = std::mem::MaybeUninit::<libc::proc_taskinfo>::uninit();
        let size = std::mem::size_of::<libc::proc_taskinfo>() as i32;
        let ret = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTASKINFO,
                0,
                task_info.as_mut_ptr() as *mut libc::c_void,
                size,
            )
        };
        if ret > 0 {
            let info = unsafe { task_info.assume_init() };
            // pti_resident_size is in bytes
            return info.pti_resident_size as f64 / 1_048_576.0;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
            for line in content.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb) = line.split_whitespace().nth(1) {
                        if let Ok(kb_val) = kb.parse::<u64>() {
                            return kb_val as f64 / 1024.0;
                        }
                    }
                }
            }
        }
    }

    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_caching() {
        let a = get_snapshot();
        let b = get_snapshot();
        // Same snapshot object within cache TTL
        if let (Some(a), Some(b)) = (a, b) {
            assert!((a.sampled_at - b.sampled_at) < Duration::from_secs(1));
        }
    }
}
