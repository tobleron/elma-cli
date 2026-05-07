//! @efficiency-role: domain-logic
//!
//! Network fetch/download/browser and offline search policy (Task 683).
//! Provides policy gates for URL access, credential sanitization, and download control.

use url::Url;

#[derive(Debug, Clone)]
pub(crate) struct NetworkPolicy {
    pub(crate) allow_http: bool,
    pub(crate) allow_downloads: bool,
    pub(crate) timeout_secs: u64,
    pub(crate) allowed_domains: Vec<String>,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allow_http: false,
            allow_downloads: false,
            timeout_secs: 30,
            allowed_domains: Vec::new(),
        }
    }
}

pub(crate) struct NetworkGate;

impl NetworkGate {
    pub(crate) fn check(url_str: &str, policy: &NetworkPolicy) -> anyhow::Result<()> {
        let parsed = Url::parse(url_str).map_err(|e| anyhow::anyhow!("invalid URL: {}", e))?;

        if !policy.allow_http && parsed.scheme() == "http" {
            anyhow::bail!("HTTP traffic not allowed by policy");
        }

        if !policy.allowed_domains.is_empty() {
            let host = parsed.host_str().unwrap_or("");
            let allowed = policy
                .allowed_domains
                .iter()
                .any(|d| host == d || host.ends_with(&format!(".{}", d)));
            if !allowed {
                anyhow::bail!("domain '{}' not in allowed list", host);
            }
        }

        Ok(())
    }

    pub(crate) fn sanitize_url(url_str: &str) -> Option<String> {
        let parsed = Url::parse(url_str).ok()?;
        if parsed.password().is_some() || parsed.username() != "" {
            let mut sanitized = parsed.clone();
            sanitized.set_username("").ok()?;
            sanitized.set_password(None).ok()?;
            Some(sanitized.to_string())
        } else {
            Some(url_str.to_string())
        }
    }
}

pub(crate) struct DownloadManager;

impl DownloadManager {
    pub(crate) fn can_download(url_str: &str, policy: &NetworkPolicy) -> bool {
        if !policy.allow_downloads {
            return false;
        }
        let parsed = match Url::parse(url_str) {
            Ok(u) => u,
            Err(_) => return false,
        };
        if !policy.allow_http && parsed.scheme() == "http" {
            return false;
        }
        if policy.allowed_domains.is_empty() {
            return true;
        }
        let host = parsed.host_str().unwrap_or("");
        policy
            .allowed_domains
            .iter()
            .any(|d| host == d || host.ends_with(&format!(".{}", d)))
    }

    pub(crate) fn allowed_domains(policy: &NetworkPolicy) -> Vec<&str> {
        policy.allowed_domains.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restrictive_policy() -> NetworkPolicy {
        NetworkPolicy {
            allow_http: false,
            allow_downloads: false,
            timeout_secs: 10,
            allowed_domains: vec!["github.com".to_string(), "api.example.com".to_string()],
        }
    }

    fn permissive_policy() -> NetworkPolicy {
        NetworkPolicy {
            allow_http: true,
            allow_downloads: true,
            timeout_secs: 60,
            allowed_domains: Vec::new(),
        }
    }

    #[test]
    fn test_network_gate_allows_https() {
        let policy = restrictive_policy();
        assert!(NetworkGate::check("https://github.com/elma", &policy).is_ok());
    }

    #[test]
    fn test_network_gate_blocks_http() {
        let policy = restrictive_policy();
        assert!(NetworkGate::check("http://github.com/elma", &policy).is_err());
    }

    #[test]
    fn test_network_gate_http_allowed_when_permissive() {
        let policy = permissive_policy();
        assert!(NetworkGate::check("http://example.com", &policy).is_ok());
    }

    #[test]
    fn test_network_gate_blocks_unlisted_domain() {
        let policy = restrictive_policy();
        assert!(NetworkGate::check("https://evil.com/malware", &policy).is_err());
    }

    #[test]
    fn test_network_gate_invalid_url() {
        let policy = restrictive_policy();
        assert!(NetworkGate::check("not a url", &policy).is_err());
    }

    #[test]
    fn test_sanitize_url_removes_credentials() {
        let cleaned = NetworkGate::sanitize_url("https://user:pass@api.example.com/data");
        assert_eq!(cleaned, Some("https://api.example.com/data".to_string()));
    }

    #[test]
    fn test_sanitize_url_no_credentials() {
        let url = "https://api.example.com/data";
        let cleaned = NetworkGate::sanitize_url(url);
        assert_eq!(cleaned, Some(url.to_string()));
    }

    #[test]
    fn test_sanitize_url_invalid() {
        assert_eq!(NetworkGate::sanitize_url(""), None);
    }

    #[test]
    fn test_download_manager_can_download_blocked() {
        let policy = restrictive_policy();
        assert!(!DownloadManager::can_download(
            "https://github.com/foo",
            &policy
        ));
    }

    #[test]
    fn test_download_manager_can_download_allowed() {
        let policy = permissive_policy();
        assert!(DownloadManager::can_download(
            "https://github.com/foo",
            &policy
        ));
    }

    #[test]
    fn test_download_manager_invalid_url() {
        let policy = permissive_policy();
        assert!(!DownloadManager::can_download("", &policy));
    }

    #[test]
    fn test_download_manager_http_blocked_when_disallowed() {
        let policy = NetworkPolicy {
            allow_http: false,
            allow_downloads: true,
            ..Default::default()
        };
        assert!(!DownloadManager::can_download(
            "http://example.com/file",
            &policy
        ));
    }

    #[test]
    fn test_allowed_domains() {
        let policy = NetworkPolicy {
            allowed_domains: vec!["a.com".into(), "b.org".into()],
            ..Default::default()
        };
        let domains = DownloadManager::allowed_domains(&policy);
        assert_eq!(domains, vec!["a.com", "b.org"]);
    }

    #[test]
    fn test_allowed_domains_empty() {
        let policy = NetworkPolicy::default();
        let domains = DownloadManager::allowed_domains(&policy);
        assert!(domains.is_empty());
    }

    #[test]
    fn test_network_policy_default() {
        let p = NetworkPolicy::default();
        assert!(!p.allow_http);
        assert!(!p.allow_downloads);
        assert_eq!(p.timeout_secs, 30);
        assert!(p.allowed_domains.is_empty());
    }

    #[test]
    fn test_domain_matching_subdomain() {
        let policy = NetworkPolicy {
            allow_http: true,
            allow_downloads: true,
            allowed_domains: vec!["example.com".into()],
            ..Default::default()
        };
        assert!(NetworkGate::check("https://sub.example.com/path", &policy).is_ok());
        assert!(NetworkGate::check("https://other.com/path", &policy).is_err());
    }
}
