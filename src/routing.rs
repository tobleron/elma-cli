//! @efficiency-role: service-orchestrator
//!
//! Routing Module (De-bloated)
//!
//! Re-exports from specialized sub-modules:
//! - routing_parse: JSON and markdown parsing
//! - routing_calc: Routing calculations and distributions
//! - routing_infer: Router inference functions

pub use crate::routing_calc::*;
pub use crate::routing_infer::*;
pub use crate::routing_parse::*;

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

    #[test]
    fn strip_markdown_wrappers_removes_code_fences() {
        let input = r#"Here is a valid JSON object:

```json
{"objective": "test", "steps": []}
```"#;
        let result = strip_markdown_wrappers(input);
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
        assert!(!result.contains("```"));
    }

    #[test]
    fn strip_markdown_wrappers_handles_no_fences() {
        let input = r#"{"objective": "test", "steps": []}"#;
        let result = strip_markdown_wrappers(input);
        assert_eq!(result, input);
    }

    #[test]
    fn strip_markdown_wrappers_handles_prose_before_fence() {
        let input = r#"Here is the JSON you requested:

```
{"key": "value"}
```

Hope this helps!"#;
        let result = strip_markdown_wrappers(input);
        assert!(result.starts_with('{'));
        assert!(result.ends_with('}'));
    }

    #[test]
    fn extract_json_from_markdown_wrapped() {
        let input = r#"Here is a valid JSON object that matches the target schema:

```json
{
  "objective": "understand current project",
  "steps": [
    {"id": "s1", "type": "shell", "cmd": "cat Cargo.toml"}
  ]
}
```"#;
        let json = extract_first_json_object(input);
        assert!(json.is_some());
        let json_str = json.unwrap();
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert!(parsed.get("objective").is_some());
        assert!(parsed.get("steps").is_some());
    }

    #[test]
    fn extract_json_from_pure_json() {
        let input = r#"{"objective": "test", "steps": []}"#;
        let json = extract_first_json_object(input);
        assert!(json.is_some());
        assert_eq!(json.unwrap(), input);
    }

    #[test]
    fn extract_json_with_prose_after() {
        let input = r#"Here is a valid JSON object that matches the target schema:

```
{
  "objective": "understand current project",
  "steps": [
    {"id": "s1", "type": "shell", "cmd": "cat Cargo.toml"}
  ]
}
```

This JSON object has the following properties:
- "objective": This is the main objective.
- "steps": This is an array of steps."#;
        let json = extract_first_json_object(input);
        assert!(json.is_some());
        let json_str = json.unwrap();
        assert!(json_str.starts_with('{'));
        assert!(json_str.ends_with('}'));
        let parsed: serde_json::Value = serde_json::from_str(json_str).unwrap();
        assert!(parsed.get("objective").is_some());
    }
}
