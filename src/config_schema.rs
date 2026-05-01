//! @efficiency-role: infra-config
//!
//! Config Schema Validation and Migration (Task 341)
//!
//! Provides TOML config validation with path-aware errors and versioned migrations.

use crate::*;
use serde_path_to_error::deserialize;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct ConfigValidationError {
    pub field_path: String,
    pub message: String,
    pub remediation: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigValidationResult {
    pub valid: bool,
    pub errors: Vec<ConfigValidationError>,
}

pub fn validate_toml_config<T: for<'de> Deserialize<'de>>(
    content: &str,
    file_path: &Path,
) -> Result<T, ConfigValidationError> {
    let de: toml::Value = toml::from_str(content).map_err(|e| ConfigValidationError {
        field_path: "file".to_string(),
        message: format!("Invalid TOML syntax: {}", e),
        remediation: Some("Check TOML syntax at the indicated location".to_string()),
    })?;

    let json_value = toml_value_to_json(&de);

    let result: Result<T, serde_path_to_error::Error<serde_json::Error>> = deserialize(&json_value);
    match result {
        Ok(value) => Ok(value),
        Err(e) => {
            let path = e.path();
            let field_path = format!("{:#}", path);
            let message = e.to_string();
            let remediation = suggest_remediation(&field_path, &message);
            Err(ConfigValidationError {
                field_path,
                message,
                remediation,
            })
        }
    }
}

fn toml_value_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i as u64)),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(toml_value_to_json).collect())
        }
        toml::Value::Table(tbl) => serde_json::Value::Object(
            tbl.iter()
                .map(|(k, v)| (k.clone(), toml_value_to_json(v)))
                .collect(),
        ),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
    }
}

fn suggest_remediation(field_path: &str, message: &str) -> Option<String> {
    if message.contains("missing") || message.contains("is missing") {
        return Some(format!(
            "Add the required field '{}' to your config",
            field_path
        ));
    }
    if message.contains("variant") && message.contains("not found") {
        return Some(format!(
            "Check the value for field '{}': use a valid enum variant",
            field_path
        ));
    }
    if message.contains("out of range") || message.contains("invalid") {
        return Some(format!(
            "Adjust the value for field '{}' to be within valid bounds",
            field_path
        ));
    }
    None
}

pub fn format_validation_errors(errors: &[ConfigValidationError]) -> String {
    let mut report = String::new();
    report.push_str("Config validation errors:\n");
    for (i, err) in errors.iter().enumerate() {
        report.push_str(&format!(
            "  {}. Field '{}': {}\n",
            i + 1,
            err.field_path,
            err.message
        ));
        if let Some(remediation) = &err.remediation {
            report.push_str(&format!("     Remediation: {}\n", remediation));
        }
    }
    report
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigVersion {
    pub version: u32,
}

pub fn migrate_config<T: for<'de> Deserialize<'de> + Serialize + 'static>(
    content: &str,
    target_version: u32,
    file_path: &Path,
) -> Result<String, ConfigValidationError> {
    let de: toml::Value = toml::from_str(content).map_err(|e| ConfigValidationError {
        field_path: "file".to_string(),
        message: format!("Invalid TOML syntax: {}", e),
        remediation: Some("Fix TOML syntax errors before migration".to_string()),
    })?;

    let json_value = toml_value_to_json(&de);

    let current_version: u32 = match serde_json::from_value::<ConfigVersion>(json_value) {
        Ok(v) => v.version,
        Err(_) => 1,
    };

    if current_version >= target_version {
        return Ok(content.to_string());
    }

    let migrated = apply_migrations(&de, current_version, target_version).map_err(|e| {
        ConfigValidationError {
            field_path: "version".to_string(),
            message: e,
            remediation: Some("Manual config update may be required".to_string()),
        }
    })?;

    Ok(
        toml::to_string_pretty(&migrated).map_err(|e| ConfigValidationError {
            field_path: "file".to_string(),
            message: format!("Failed to serialize migrated config: {}", e),
            remediation: None,
        })?,
    )
}

fn apply_migrations(
    value: &toml::Value,
    from_version: u32,
    to_version: u32,
) -> Result<toml::Value, String> {
    let mut current = value.clone();
    let mut v = from_version;

    while v < to_version {
        current = apply_migration_step(&current, v)?;
        v += 1;
    }

    Ok(current)
}

fn apply_migration_step(value: &toml::Value, version: u32) -> Result<toml::Value, String> {
    match version {
        1 => migrate_v1_to_v2(value),
        _ => Err(format!("Unknown migration from version {}", version)),
    }
}

fn migrate_v1_to_v2(value: &toml::Value) -> Result<toml::Value, String> {
    let mut tbl = match value {
        toml::Value::Table(t) => t.clone(),
        _ => return Err("Expected table value".to_string()),
    };
    tbl.insert("version".to_string(), toml::Value::Integer(2));
    Ok(toml::Value::Table(tbl))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    struct TestProfile {
        version: u32,
        model: String,
        temperature: f64,
    }

    #[test]
    fn test_validate_valid_toml() {
        let content = r#"
version = 1
model = "test-model"
temperature = 0.5
"#;
        let result = validate_toml_config::<TestProfile>(content, Path::new("test.toml"));
        assert!(result.is_ok());
        let profile = result.unwrap();
        assert_eq!(profile.version, 1);
        assert_eq!(profile.model, "test-model");
    }

    #[test]
    fn test_validate_missing_field() {
        let content = r#"
version = 1
model = "test-model"
"#;
        let result = validate_toml_config::<TestProfile>(content, Path::new("test.toml"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("missing") || err.field_path.contains("temperature"));
    }

    #[test]
    fn test_validate_invalid_type() {
        let content = r#"
version = "not-a-number"
model = "test-model"
temperature = 0.5
"#;
        let result = validate_toml_config::<TestProfile>(content, Path::new("test.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_format_validation_errors() {
        let errors = vec![ConfigValidationError {
            field_path: "temperature".to_string(),
            message: "out of range".to_string(),
            remediation: Some("Use 0.0-2.0".to_string()),
        }];
        let report = format_validation_errors(&errors);
        assert!(report.contains("temperature"));
        assert!(report.contains("out of range"));
        assert!(report.contains("Remediation"));
    }

    #[test]
    fn test_migrate_v1_to_v2() {
        let content = r#"
version = 1
model = "test"
temperature = 0.5
"#;
        let result = migrate_config::<TestProfile>(content, 2, Path::new("test.toml"));
        assert!(result.is_ok(), "Migration failed: {:?}", result);
        let migrated = result.unwrap();
        assert!(
            migrated.contains("version = 2") || migrated.contains("version=2"),
            "Migrated content: {}",
            migrated
        );
    }
}
