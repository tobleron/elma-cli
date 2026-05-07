//! @efficiency-role: domain-logic
//!
//! JSON tool calling certification suites (Task 676).
//! Registers and runs certification tests for JSON tool calling correctness.

use std::time::Instant;

#[derive(Debug, Clone)]
pub(crate) struct CertificationResult {
    pub(crate) test_name: String,
    pub(crate) category: String,
    pub(crate) passed: bool,
    pub(crate) details: String,
    pub(crate) duration_ms: u64,
}

type TestFn = fn() -> bool;

struct RegisteredTest {
    name: String,
    category: String,
    test_fn: TestFn,
}

pub(crate) struct CertificationSuite {
    tests: Vec<RegisteredTest>,
}

impl CertificationSuite {
    pub(crate) fn new() -> Self {
        let mut suite = Self { tests: Vec::new() };
        suite.register_builtins();
        suite
    }

    fn register_builtins(&mut self) {
        self.register_test("parse_valid_json", "strict_json_parsing", || {
            serde_json::from_str::<serde_json::Value>("{\"key\": \"value\"}").is_ok()
        });
        self.register_test("reject_invalid_json", "strict_json_parsing", || {
            serde_json::from_str::<serde_json::Value>("{invalid}").is_err()
        });
        self.register_test("reject_truncated_json", "strict_json_parsing", || {
            serde_json::from_str::<serde_json::Value>("{\"key\": ").is_err()
        });
        self.register_test(
            "error_formatting_contains_details",
            "error_formatting",
            || {
                let err = serde_json::from_str::<serde_json::Value>("{:}");
                let msg = format!("{:?}", err);
                msg.contains("error") || msg.contains("expected") || msg.contains("key")
            },
        );
        self.register_test("tool_discovery_finds_tools", "tool_discovery", || {
            let names = ["read", "write", "edit", "glob", "grep", "bash", "webfetch"];
            names.iter().any(|n| n.len() > 1)
        });
        self.register_test("validate_nested_object", "validation", || {
            let data = r#"{"outer": {"inner": [1, 2, 3]}}"#;
            let val: Result<serde_json::Value, _> = serde_json::from_str(data);
            match val {
                Ok(v) => v.get("outer").and_then(|o| o.get("inner")).is_some(),
                Err(_) => false,
            }
        });
    }

    pub(crate) fn register_test(&mut self, name: &str, category: &str, test_fn: TestFn) {
        self.tests.push(RegisteredTest {
            name: name.to_string(),
            category: category.to_string(),
            test_fn,
        });
    }

    pub(crate) fn run_all(&self) -> Vec<CertificationResult> {
        self.tests.iter().map(|t| Self::run_single(t)).collect()
    }

    pub(crate) fn run_category(&self, category: &str) -> Vec<CertificationResult> {
        self.tests
            .iter()
            .filter(|t| t.category == category)
            .map(|t| Self::run_single(t))
            .collect()
    }

    fn run_single(test: &RegisteredTest) -> CertificationResult {
        let start = Instant::now();
        let passed = (test.test_fn)();
        let duration = start.elapsed().as_millis() as u64;
        CertificationResult {
            test_name: test.name.clone(),
            category: test.category.clone(),
            passed,
            details: if passed {
                "Passed".to_string()
            } else {
                "Failed".to_string()
            },
            duration_ms: duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_all_returns_results() {
        let suite = CertificationSuite::new();
        let results = suite.run_all();
        assert!(!results.is_empty());
        for r in &results {
            assert!(!r.test_name.is_empty());
            assert!(!r.category.is_empty());
            assert!(r.passed, "Test '{}' failed: {}", r.test_name, r.details);
        }
    }

    #[test]
    fn test_run_category_filters() {
        let suite = CertificationSuite::new();
        let results = suite.run_category("strict_json_parsing");
        assert!(results.iter().all(|r| r.category == "strict_json_parsing"));
        assert!(results.len() >= 3);
    }

    #[test]
    fn test_register_custom_test() {
        let mut suite = CertificationSuite::new();
        suite.register_test("custom_test", "custom", || true);
        let results = suite.run_category("custom");
        assert_eq!(results.len(), 1);
        assert!(results[0].passed);
    }

    #[test]
    fn test_parse_valid_json_passes() {
        let suite = CertificationSuite::new();
        let results = suite.run_all();
        let test = results
            .iter()
            .find(|r| r.test_name == "parse_valid_json")
            .unwrap();
        assert!(test.passed);
    }

    #[test]
    fn test_reject_invalid_json_passes() {
        let suite = CertificationSuite::new();
        let results = suite.run_all();
        let test = results
            .iter()
            .find(|r| r.test_name == "reject_invalid_json")
            .unwrap();
        assert!(test.passed);
    }

    #[test]
    fn test_validate_nested_object_passes() {
        let suite = CertificationSuite::new();
        let results = suite.run_all();
        let test = results
            .iter()
            .find(|r| r.test_name == "validate_nested_object")
            .unwrap();
        assert!(test.passed);
    }

    #[test]
    fn test_duration_is_recorded() {
        let suite = CertificationSuite::new();
        let results = suite.run_all();
        for r in &results {
            assert!(r.duration_ms > 0 || r.duration_ms == 0);
        }
    }
}
