//! @efficiency-role: util-pure
//!
//! Routing - Calculations and Distribution Handling

use crate::*;

pub(crate) fn workflow_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[("1", "CHAT"), ("2", "WORKFLOW")]
}

pub(crate) fn mode_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("1", "INSPECT"),
        ("2", "EXECUTE"),
        ("3", "PLAN"),
        ("4", "MASTERPLAN"),
        ("5", "DECIDE"),
    ]
}

pub(crate) fn speech_act_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("1", "CAPABILITY_CHECK"),
        ("2", "INFO_REQUEST"),
        ("3", "ACTION_REQUEST"),
    ]
}

pub(crate) fn route_label_from_router_output(
    raw: &str,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<&'static str> {
    let token = raw
        .trim()
        .trim_matches(|c: char| c == '"' || c == '\'')
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim();
    for (code, label) in pairs {
        if token == *code || token.eq_ignore_ascii_case(label) {
            return Some(label);
        }
    }
    None
}

pub(crate) fn logsumexp(values: &[f64]) -> f64 {
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max_v.is_finite() {
        return f64::NEG_INFINITY;
    }
    let sum = values.iter().map(|v| (v - max_v).exp()).sum::<f64>();
    max_v + sum.ln()
}

pub(crate) fn parse_router_distribution(
    logprobs: &serde_json::Value,
    pairs: &'static [(&'static str, &'static str)],
) -> Option<Vec<(String, f64)>> {
    let top_logprobs = logprobs
        .get("content")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .and_then(|v| v.get("top_logprobs"))
        .and_then(|v| v.as_array())?;

    let mut route_logprobs: HashMap<String, Vec<f64>> = HashMap::new();
    for item in top_logprobs {
        let token = item
            .get("token")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let Some(logprob) = item.get("logprob").and_then(|v| v.as_f64()) else {
            continue;
        };
        if let Some(label) = route_label_from_router_output(token, pairs) {
            route_logprobs
                .entry(label.to_string())
                .or_default()
                .push(logprob);
        }
    }
    if route_logprobs.is_empty() {
        return None;
    }

    let mut entries: Vec<(String, f64)> = pairs
        .iter()
        .map(|(_, label)| {
            let lp = route_logprobs
                .get(*label)
                .map(|values| logsumexp(values))
                .unwrap_or(f64::NEG_INFINITY);
            ((*label).to_string(), lp)
        })
        .collect();

    let max_lp = entries
        .iter()
        .map(|(_, lp)| *lp)
        .filter(|lp| lp.is_finite())
        .fold(f64::NEG_INFINITY, f64::max);
    if !max_lp.is_finite() {
        return None;
    }
    let denom = entries
        .iter()
        .map(|(_, lp)| {
            if lp.is_finite() {
                (lp - max_lp).exp()
            } else {
                0.0
            }
        })
        .sum::<f64>();
    if denom <= 0.0 {
        return None;
    }
    for (_, lp) in &mut entries {
        let p = if lp.is_finite() {
            (*lp - max_lp).exp() / denom
        } else {
            0.0
        };
        *lp = p;
    }
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    Some(entries)
}

pub(crate) fn route_margin(distribution: &[(String, f64)]) -> f64 {
    let top = distribution.first().map(|(_, p)| *p).unwrap_or(0.0);
    let second = distribution.get(1).map(|(_, p)| *p).unwrap_or(0.0);
    top - second
}

pub(crate) fn route_entropy(distribution: &[(String, f64)]) -> f64 {
    distribution
        .iter()
        .map(|(_, p)| if *p > 0.0 { -p * p.ln() } else { 0.0 })
        .sum()
}

pub(crate) fn inject_classification_noise(
    distribution: &[(String, f64)],
    entropy: f64,
) -> Vec<(String, f64)> {
    const ENTROPY_THRESHOLD: f64 = 0.1;
    const NOISE_SCALE: f64 = 0.05;

    if entropy >= ENTROPY_THRESHOLD {
        return distribution.to_vec();
    }

    let mut noisy: Vec<(String, f64)> = distribution
        .iter()
        .map(|(label, p)| {
            let noise = (std::process::id() as f64 * 0.001).sin() * NOISE_SCALE * p;
            (label.clone(), *p + noise)
        })
        .collect();

    let sum: f64 = noisy.iter().map(|(_, p)| *p).sum();
    if sum > 0.0 {
        for (_, p) in &mut noisy {
            *p /= sum;
            *p = (*p).max(0.001);
        }
        let sum2: f64 = noisy.iter().map(|(_, p)| *p).sum();
        if sum2 > 0.0 {
            for (_, p) in &mut noisy {
                *p /= sum2;
            }
        }
    }

    noisy.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    noisy
}

pub(crate) fn format_route_distribution(distribution: &[(String, f64)]) -> String {
    distribution
        .iter()
        .map(|(route, p)| format!("{route}:{p:.2}"))
        .collect::<Vec<_>>()
        .join(" ")
}

pub(crate) fn probability_of(distribution: &[(String, f64)], label: &str) -> f64 {
    distribution
        .iter()
        .find(|(name, _)| name == label)
        .map(|(_, p)| *p)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_entropy() {
        let certain = vec![("A".to_string(), 1.0), ("B".to_string(), 0.0)];
        assert!(route_entropy(&certain) < 0.01);

        let uncertain = vec![("A".to_string(), 0.5), ("B".to_string(), 0.5)];
        assert!(route_entropy(&uncertain) > 0.6);
    }

    #[test]
    fn test_inject_classification_noise_skips_high_entropy() {
        let high_entropy = vec![("A".to_string(), 0.5), ("B".to_string(), 0.5)];
        let noisy = inject_classification_noise(&high_entropy, 0.7);
        assert!((noisy[0].1 - 0.5).abs() < 0.01);
        assert!((noisy[1].1 - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_inject_classification_noise_adds_to_low_entropy() {
        let low_entropy = vec![("A".to_string(), 0.99), ("B".to_string(), 0.01)];
        let noisy = inject_classification_noise(&low_entropy, 0.05);
        let sum: f64 = noisy.iter().map(|(_, p)| *p).sum();
        assert!((sum - 1.0).abs() < 0.01);
        assert!(noisy[0].1 > noisy[1].1);
    }

    #[test]
    fn test_inject_classification_noise_preserves_minimum_probability() {
        let low_entropy = vec![("A".to_string(), 0.999), ("B".to_string(), 0.001)];
        for _ in 0..5 {
            let noisy = inject_classification_noise(&low_entropy, 0.01);
            for (_, p) in &noisy {
                assert!(*p >= 0.0009);
            }
            let sum: f64 = noisy.iter().map(|(_, p)| *p).sum();
            assert!((sum - 1.0).abs() < 0.01);
        }
    }
}
