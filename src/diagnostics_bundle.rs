//! @efficiency-role: domain-logic
//!
//! Diagnostics bundle and doctor command (Task 665).
//! Collects system health information and produces actionable recommendations.

use crate::*;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModelCheck {
    pub name: String,
    pub reachable: bool,
    pub latency_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DiagnosticsReport {
    pub elma_version: String,
    pub os_info: String,
    pub rust_version: String,
    pub disk_space: String,
    pub session_count: usize,
    pub config_status: String,
    pub model_status: Vec<ModelCheck>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionHealth {
    pub total: usize,
    pub corrupted: usize,
    pub largest_kb: u64,
    pub oldest_days: u64,
}

pub(crate) struct DiagnosticsCollector;

impl DiagnosticsCollector {
    pub(crate) fn collect() -> DiagnosticsReport {
        let elma_version = env!("CARGO_PKG_VERSION").to_string();
        let os_info = std::env::consts::OS.to_string();
        if let Ok(os_extra) = std::process::Command::new("uname").arg("-a").output() {
            if let Ok(s) = String::from_utf8(os_extra.stdout) {
                let _ = s;
            }
        }

        let rust_version = std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let disk_space = Self::get_disk_space();
        let session_count = 0;
        let config_status = Self::check_config().join("; ");
        let model_status = Vec::new();
        let warnings = Self::check_config();

        DiagnosticsReport {
            elma_version,
            os_info,
            rust_version,
            disk_space,
            session_count,
            config_status,
            model_status,
            warnings,
        }
    }

    pub(crate) fn check_config() -> Vec<String> {
        let mut warnings = Vec::new();
        let config_paths = [
            elma_config_path().ok().map(|p| p.join("elma.toml")),
            elma_config_path().ok().map(|p| p.join("config.toml")),
        ];
        for path in config_paths.iter().flatten() {
            if !path.exists() {
                warnings.push(format!("Missing config: {}", path.display()));
            } else if let Ok(meta) = path.metadata() {
                if meta.len() == 0 {
                    warnings.push(format!("Empty config: {}", path.display()));
                }
            }
        }
        warnings
    }

    pub(crate) fn check_sessions(sessions_root: &Path) -> SessionHealth {
        let mut total = 0usize;
        let mut corrupted = 0usize;
        let mut largest_kb = 0u64;
        let mut oldest_secs = u64::MAX;

        if let Ok(entries) = std::fs::read_dir(sessions_root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    total += 1;
                    let session_file = path.join("session.json");
                    if !session_file.exists() {
                        corrupted += 1;
                    }
                    if let Ok(meta) = path.metadata() {
                        let kb = meta.len() / 1024;
                        if kb > largest_kb {
                            largest_kb = kb;
                        }
                    }
                    if let Ok(meta) = path.metadata() {
                        if let Ok(modified) = meta.modified() {
                            if let Ok(dur) = modified.duration_since(SystemTime::UNIX_EPOCH) {
                                let secs = dur.as_secs();
                                if secs < oldest_secs {
                                    oldest_secs = secs;
                                }
                            }
                        }
                    }
                }
            }
        }

        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let oldest_days = if oldest_secs != u64::MAX && now > oldest_secs {
            (now - oldest_secs) / 86400
        } else {
            0
        };

        SessionHealth {
            total,
            corrupted,
            largest_kb,
            oldest_days,
        }
    }

    fn get_disk_space() -> String {
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            if let Ok(output) = std::process::Command::new("df").arg("-h").arg("/").output() {
                if let Ok(s) = String::from_utf8(output.stdout) {
                    for line in s.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 4 {
                            return format!("{} used / {} total", parts[2], parts[1]);
                        }
                    }
                }
            }
        }
        "unknown".to_string()
    }
}

pub(crate) struct DoctorCommand;

impl DoctorCommand {
    pub(crate) fn run(report: &DiagnosticsReport) -> Vec<String> {
        let mut recommendations = Vec::new();
        for warning in &report.warnings {
            if warning.contains("Missing config") {
                recommendations.push(format!(
                    "Create default config file: {}",
                    warning.replace("Missing config: ", "")
                ));
            } else if warning.contains("Empty config") {
                recommendations.push(format!(
                    "Populate config file: {}",
                    warning.replace("Empty config: ", "")
                ));
            }
        }
        recommendations
    }

    pub(crate) fn auto_fix(warnings: &[String]) -> Vec<String> {
        let mut fixes = Vec::new();
        for w in warnings {
            if w.contains("Missing config") {
                let path = w.replace("Missing config: ", "");
                fixes.push(format!("touch {}", path));
            }
        }
        fixes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_returns_report() {
        let report = DiagnosticsCollector::collect();
        assert!(!report.elma_version.is_empty());
        assert!(!report.os_info.is_empty());
    }

    #[test]
    fn test_check_config_returns_warnings() {
        let warnings = DiagnosticsCollector::check_config();
        assert!(warnings.is_empty() || warnings.iter().any(|w| w.contains("Missing")));
    }

    #[test]
    fn test_check_sessions_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let health = DiagnosticsCollector::check_sessions(tmp.path());
        assert_eq!(health.total, 0);
        assert_eq!(health.corrupted, 0);
        assert_eq!(health.largest_kb, 0);
    }

    #[test]
    fn test_check_sessions_with_corrupted() {
        let tmp = tempfile::tempdir().unwrap();
        let session_dir = tmp.path().join("session_001");
        std::fs::create_dir_all(&session_dir).unwrap();
        let health = DiagnosticsCollector::check_sessions(tmp.path());
        assert_eq!(health.total, 1);
        assert_eq!(health.corrupted, 1);
    }

    #[test]
    fn test_doctor_run_no_warnings() {
        let report = DiagnosticsReport {
            elma_version: "1.0".into(),
            os_info: "test".into(),
            rust_version: "1.80".into(),
            disk_space: "10G / 100G".into(),
            session_count: 0,
            config_status: "ok".into(),
            model_status: Vec::new(),
            warnings: Vec::new(),
        };
        let recs = DoctorCommand::run(&report);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_doctor_run_with_warnings() {
        let report = DiagnosticsReport {
            elma_version: "1.0".into(),
            os_info: "test".into(),
            rust_version: "1.80".into(),
            disk_space: "10G / 100G".into(),
            session_count: 0,
            config_status: "warn".into(),
            model_status: Vec::new(),
            warnings: vec!["Missing config: /tmp/elma.toml".into()],
        };
        let recs = DoctorCommand::run(&report);
        assert!(!recs.is_empty());
        assert!(recs[0].contains("Create default config"));
    }

    #[test]
    fn test_doctor_auto_fix() {
        let warnings = vec!["Missing config: /tmp/elma.toml".into()];
        let fixes = DoctorCommand::auto_fix(&warnings);
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].starts_with("touch"));
    }
}
