//! @efficiency-role: service-orchestrator
//!
//! JSON Temperature Tuning - Find optimal temperature for reliable JSON output
//!
//! Tests model at temperatures 0.0 to 1.0 (step 0.1) across three difficulty levels.
//! Selects the temperature that produces the most reliable, repairable JSON.

use crate::*;

/// Temperature test range: 0.0 to 1.0 in 0.1 increments
const TEMPERATURES: [f32; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

/// Difficulty levels for JSON testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum JsonDifficulty {
    Low,
    Medium,
    Hard,
}

impl JsonDifficulty {
    fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "low" => JsonDifficulty::Low,
            "medium" => JsonDifficulty::Medium,
            "hard" => JsonDifficulty::Hard,
            _ => JsonDifficulty::Medium,
        }
    }

    fn weight(&self) -> f32 {
        match self {
            JsonDifficulty::Low => 1.0,
            JsonDifficulty::Medium => 2.0,
            JsonDifficulty::Hard => 3.0,
        }
    }
}

/// Result of testing a single temperature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TemperatureResult {
    pub(crate) temperature: f32,
    pub(crate) total_tests: usize,
    pub(crate) valid_json_count: usize,
    pub(crate) repairable_count: usize,
    pub(crate) failed_count: usize,
    pub(crate) avg_parse_attempts: f32,
    pub(crate) weighted_score: f32,
}

/// Complete JSON tuning result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct JsonTuningResult {
    pub(crate) optimal_temperature: f32,
    pub(crate) recommended_temperature: f32,
    pub(crate) results_by_temp: Vec<TemperatureResult>,
    pub(crate) passing_temperatures: usize,
    pub(crate) total_temperatures: usize,
}

/// JSON tuning scenario from manifest
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct JsonTuningScenario {
    pub(crate) suite: String,
    pub(crate) file: String,
    pub(crate) difficulty: String,
    #[serde(default)]
    pub(crate) expected_json_type: String,
    #[serde(default)]
    pub(crate) min_fields: Option<usize>,
    #[serde(default)]
    pub(crate) min_depth: Option<usize>,
    #[serde(default)]
    pub(crate) min_items: Option<usize>,
    #[serde(default)]
    pub(crate) notes: String,
}

/// JSON tuning manifest
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct JsonTuningManifest {
    pub(crate) version: u32,
    pub(crate) description: String,
    pub(crate) scenarios: Vec<JsonTuningScenario>,
}

/// Load JSON tuning manifest
pub(crate) fn load_json_tuning_manifest() -> Result<JsonTuningManifest> {
    let manifest_path = repo_root()?.join("scenarios/json_tune/manifest.toml");
    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    toml::from_str(&content).with_context(|| format!("parse {}", manifest_path.display()))
}

/// Test JSON output at a specific temperature
pub(crate) async fn test_json_at_temperature(
    client: &reqwest::Client,
    chat_url: &Url,
    model: &str,
    temperature: f32,
    scenarios: &[JsonTuningScenario],
) -> Result<TemperatureResult> {
    let mut total = 0;
    let mut valid = 0;
    let mut repairable = 0;
    let mut failed = 0;
    let mut total_attempts = 0;

    for scenario in scenarios {
        let scenario_path = repo_root()?.join("scenarios/json_tune").join(&scenario.file);
        let content = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        
        // Extract user message from scenario
        let user_message = content
            .lines()
            .find(|l| l.starts_with("user:"))
            .map(|l| l.trim_start_matches("user:").trim())
            .unwrap_or(&content);

        let difficulty = JsonDifficulty::from_str(&scenario.difficulty);
        total += 1;
        total_attempts += 1;

        // Build request with explicit JSON instruction
        let req = ChatCompletionRequest {
            model: model.to_string(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: "Return ONLY valid JSON. No prose, no explanations, no markdown fences. Just raw JSON.".to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: user_message.to_string(),
                },
            ],
            temperature: temperature as f64,
            top_p: 0.95,
            stream: false,
            max_tokens: 2048,
            n_probs: None,
            repeat_penalty: Some(1.1),
            reasoning_format: Some("none".to_string()),
        grammar: None,
        };

        // Try to get and parse JSON
        let response = chat_once(client, chat_url, &req).await;
        match response {
            Ok(resp) => {
                let text = extract_response_text(&resp);
                
                // Try parsing with our repair pipeline
                let parse_result: Result<serde_json::Value> = parse_json_loose(&text);
                match parse_result {
                    Ok(_json) => {
                        valid += 1;
                        repairable += 1; // If it parsed, it was repairable
                    }
                    Err(_) => {
                        // Try jsonrepair directly
                        if let Ok(repaired) = jsonrepair_rs::jsonrepair(&text) {
                            if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
                                repairable += 1;
                            } else {
                                failed += 1;
                            }
                        } else {
                            failed += 1;
                        }
                    }
                }
            }
            Err(_) => {
                failed += 1;
            }
        }
    }

    // Calculate weighted score
    let base_accuracy = if total > 0 {
        (valid as f32) / (total as f32)
    } else {
        0.0
    };

    // Prefer temperatures with fewer repair attempts needed
    let repair_bonus = if valid > 0 {
        (repairable as f32) / (valid as f32) * 0.1
    } else {
        0.0
    };

    let weighted_score = base_accuracy + repair_bonus;
    let avg_parse_attempts = if total > 0 {
        (total_attempts as f32) / (total as f32)
    } else {
        0.0
    };

    Ok(TemperatureResult {
        temperature,
        total_tests: total,
        valid_json_count: valid,
        repairable_count: repairable,
        failed_count: failed,
        avg_parse_attempts,
        weighted_score,
    })
}

/// Run complete JSON temperature tuning
pub(crate) async fn run_json_temperature_tuning(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    model_cfg_dir: &Path,
    model_id: &str,
    emit_progress: bool,
) -> Result<JsonTuningResult> {
    let manifest = load_json_tuning_manifest()?;
    let scenarios = &manifest.scenarios;

    if emit_progress {
        calibration_progress(
            args,
            &format!("JSON temperature tuning for {} ({} scenarios)", model_id, scenarios.len()),
        );
    }

    // Check for existing JSON tuning results (caching)
    let tune_dir = model_cfg_dir.join("tune").join("json");
    let cached_result = load_cached_json_tuning_result(&tune_dir)?;
    
    // If we have a complete cached result, use it entirely without any API calls
    if let Some(cached) = &cached_result {
        if cached.results_by_temp.len() == TEMPERATURES.len() {
            if emit_progress {
                for cached_temp_result in &cached.results_by_temp {
                    let status = if cached_temp_result.failed_count == 0 { "✓" } else { "✗" };
                    calibration_progress(
                        args,
                        &format!(
                            "  temp={:.1}: {}/{} valid {} (cached)",
                            cached_temp_result.temperature,
                            cached_temp_result.valid_json_count,
                            cached_temp_result.total_tests,
                            status,
                        ),
                    );
                }
                calibration_progress(
                    args,
                    &format!(
                        "  ✓ JSON tuning complete: {} of {} temperatures reliable, recommended temp={:.2} (all cached)",
                        cached.passing_temperatures, cached.total_temperatures, cached.recommended_temperature
                    ),
                );
            }
            return Ok(cached.clone());
        }
    }

    let mut results_by_temp = Vec::new();

    // Test each temperature (skip if cached)
    for &temp in &TEMPERATURES {
        // Check if this temperature was already tested
        if let Some(cached) = &cached_result {
            if let Some(cached_temp_result) = cached.results_by_temp.iter().find(|r| (r.temperature - temp).abs() < 0.01) {
                if emit_progress {
                    let status = if cached_temp_result.failed_count == 0 { "✓" } else { "✗" };
                    calibration_progress(
                        args,
                        &format!(
                            "  temp={:.1}: {}/{} valid {} (cached)",
                            cached_temp_result.temperature,
                            cached_temp_result.valid_json_count,
                            cached_temp_result.total_tests,
                            status,
                        ),
                    );
                }
                results_by_temp.push(cached_temp_result.clone());
                continue; // Skip re-testing
            }
        }
        
        let result = test_json_at_temperature(client, chat_url, model_id, temp, scenarios).await?;

        if emit_progress {
            let status = if result.failed_count == 0 { "✓" } else { "✗" };
            let note = if result.failed_count > 0 {
                format!(" ({} failed)", result.failed_count)
            } else {
                String::new()
            };
            calibration_progress(
                args,
                &format!(
                    "  temp={:.1}: {}/{} valid {}{}",
                    result.temperature,
                    result.valid_json_count,
                    result.total_tests,
                    status,
                    note,
                ),
            );
        }

        results_by_temp.push(result.clone());
    }

    // Find optimal temperature (highest weighted score)
    let optimal = results_by_temp
        .iter()
        .max_by(|a, b| a.weighted_score.partial_cmp(&b.weighted_score).unwrap())
        .cloned()
        .unwrap_or(TemperatureResult {
            temperature: 0.2,
            total_tests: 0,
            valid_json_count: 0,
            repairable_count: 0,
            failed_count: 0,
            avg_parse_attempts: 0.0,
            weighted_score: 0.0,
        });
    
    // Count how many temps passed
    let passing_temps = results_by_temp.iter().filter(|r| r.failed_count == 0).count();
    let total_temps = TEMPERATURES.len();

    // Recommended temperature: prefer lower temps if scores are close (more deterministic)
    let recommended = if optimal.weighted_score >= 0.9 {
        // Good scores, prefer lower temperature for determinism
        results_by_temp
            .iter()
            .filter(|r| r.weighted_score >= optimal.weighted_score - 0.05)
            .min_by(|a, b| a.temperature.partial_cmp(&b.temperature).unwrap())
            .map(|r| r.temperature)
            .unwrap_or(optimal.temperature)
    } else {
        optimal.temperature
    };
    
    if emit_progress {
        calibration_progress(
            args,
            &format!(
                "  ✓ JSON tuning complete: {} of {} temperatures reliable, recommended temp={:.2}",
                passing_temps, total_temps, recommended
            ),
        );
    }

    Ok(JsonTuningResult {
        optimal_temperature: optimal.temperature,
        recommended_temperature: recommended,
        results_by_temp,
        passing_temperatures: passing_temps,
        total_temperatures: total_temps,
    })
}

/// Load cached JSON tuning result (most recent)
pub(crate) fn load_cached_json_tuning_result(tune_dir: &Path) -> Result<Option<JsonTuningResult>> {
    if !tune_dir.exists() {
        return Ok(None);
    }
    
    // Find most recent tuning result
    let mut latest_path = None;
    let mut latest_timestamp = 0i64;
    
    for entry in std::fs::read_dir(tune_dir)
        .with_context(|| format!("read_dir {}", tune_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("toml") {
            // Extract timestamp from filename: json_tuning_<timestamp>.toml
            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                if let Some(ts_str) = name.strip_prefix("json_tuning_") {
                    if let Ok(ts) = ts_str.parse::<i64>() {
                        if ts > latest_timestamp {
                            latest_timestamp = ts;
                            latest_path = Some(path);
                        }
                    }
                }
            }
        }
    }
    
    if let Some(path) = latest_path {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("read {}", path.display()))?;
        
        // Parse TOML manually to extract results
        // Note: We need a simpler format for caching
        return Ok(parse_cached_result(&content));
    }
    
    Ok(None)
}

/// Parse cached result from TOML content
fn parse_cached_result(content: &str) -> Option<JsonTuningResult> {
    // Simple parser for cached results
    let mut results_by_temp: Vec<TemperatureResult> = Vec::new();
    let mut in_temps = false;
    let mut passing = 0;
    let mut total = 11; // default
    
    // First pass: collect all data
    for line in content.lines() {
        let line = line.trim();
        
        if line.starts_with("passing_temperatures") {
            if let Some(val) = line.split('=').nth(1) {
                passing = val.trim().parse().unwrap_or(0);
            }
        } else if line.starts_with("total_temperatures") {
            if let Some(val) = line.split('=').nth(1) {
                total = val.trim().parse().unwrap_or(11);
            }
        } else if line == "[[temperatures]]" {
            in_temps = true;
        } else if line.starts_with("[[") && !line.starts_with("[[temperatures]]") {
            in_temps = false;
        } else if in_temps && line.trim().starts_with('{') && line.trim().ends_with('}') {
            // Parse temperature result
            if let Some(result) = parse_temp_result_line(line) {
                results_by_temp.push(result);
            }
        }
    }
    
    if results_by_temp.is_empty() {
        return None;
    }
    
    // Find optimal and recommended
    let optimal = results_by_temp
        .iter()
        .max_by(|a, b| a.weighted_score.partial_cmp(&b.weighted_score).unwrap())
        .cloned()?;
    
    let recommended = results_by_temp
        .iter()
        .filter(|r| r.weighted_score >= optimal.weighted_score - 0.05)
        .min_by(|a, b| a.temperature.partial_cmp(&b.temperature).unwrap())
        .map(|r| r.temperature)
        .unwrap_or(optimal.temperature);
    
    Some(JsonTuningResult {
        optimal_temperature: optimal.temperature,
        recommended_temperature: recommended,
        results_by_temp,
        passing_temperatures: passing,
        total_temperatures: total,
    })
}

/// Parse a single temperature result line
fn parse_temp_result_line(line: &str) -> Option<TemperatureResult> {
    // Format: { temperature = 0.0, valid = 6, total = 6, repairable = 6, failed = 0, score = 1.100 }
    let mut temperature = 0.0f32;
    let mut valid = 0;
    let mut total = 0;
    let mut repairable = 0;
    let mut failed = 0;
    let mut score = 0.0f32;
    
    for part in line.trim_start_matches('{').trim_end_matches('}').split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "temperature" => temperature = value.parse().ok()?,
                "valid" => valid = value.parse().ok()?,
                "total" => total = value.parse().ok()?,
                "repairable" => repairable = value.parse().ok()?,
                "failed" => failed = value.parse().ok()?,
                "score" => score = value.parse().ok()?,
                _ => {}
            }
        }
    }
    
    if total == 0 {
        return None;
    }
    
    Some(TemperatureResult {
        temperature,
        total_tests: total,
        valid_json_count: valid,
        repairable_count: repairable,
        failed_count: failed,
        avg_parse_attempts: 1.0,
        weighted_score: score,
    })
}

/// Save JSON tuning result to file
pub(crate) fn save_json_tuning_report(
    model_cfg_dir: &Path,
    result: &JsonTuningResult,
) -> Result<()> {
    let report_dir = model_cfg_dir.join("tune").join("json");
    std::fs::create_dir_all(&report_dir)
        .with_context(|| format!("mkdir {}", report_dir.display()))?;

    let timestamp = now_unix_s()?;
    let report_path = report_dir.join(format!("json_tuning_{}.toml", timestamp));

    let mut content = String::new();
    content.push_str(&format!("# JSON Temperature Tuning Report\n"));
    content.push_str(&format!("timestamp = {}\n", timestamp));
    content.push_str(&format!("optimal_temperature = {:.2}\n", result.optimal_temperature));
    content.push_str(&format!("recommended_temperature = {:.2}\n", result.recommended_temperature));
    content.push_str(&format!("passing_temperatures = {}\n", result.passing_temperatures));
    content.push_str(&format!("total_temperatures = {}\n\n", result.total_temperatures));

    content.push_str("# Results by Temperature\n");
    content.push_str("[[temperatures]]\n");
    for r in &result.results_by_temp {
        content.push_str(&format!(
            "  {{ temperature = {:.1}, valid = {}, total = {}, repairable = {}, failed = {}, score = {:.3} }}\n",
            r.temperature, r.valid_json_count, r.total_tests, r.repairable_count, r.failed_count, r.weighted_score
        ));
    }

    std::fs::write(&report_path, &content)
        .with_context(|| format!("write {}", report_path.display()))?;

    eprintln!("[JSON_TUNING] Report saved to {}", report_path.display());
    Ok(())
}

/// Apply optimal temperature to orchestrator profiles
pub(crate) fn apply_json_tuning_temperature(
    model_cfg_dir: &Path,
    temperature: f32,
) -> Result<()> {
    let temp_f64 = temperature as f64;
    
    // Update orchestrator.toml
    let orchestrator_path = model_cfg_dir.join("orchestrator.toml");
    if orchestrator_path.exists() {
        let mut cfg = load_agent_config(&orchestrator_path)?;
        cfg.temperature = temp_f64;
        save_agent_config(&orchestrator_path, &cfg)?;
    }

    // Update workflow_planner.toml
    let planner_path = model_cfg_dir.join("workflow_planner.toml");
    if planner_path.exists() {
        let mut cfg = load_agent_config(&planner_path)?;
        cfg.temperature = temp_f64;
        save_agent_config(&planner_path, &cfg)?;
    }

    // Update json_outputter.toml
    let outputter_path = model_cfg_dir.join("json_outputter.toml");
    if outputter_path.exists() {
        let mut cfg = load_agent_config(&outputter_path)?;
        cfg.temperature = temp_f64;
        save_agent_config(&outputter_path, &cfg)?;
    }

    // CRITICAL: Update router profiles for deterministic classification
    // Routers output single digits and need low temperature
    for router_name in &["router.toml", "speech_act.toml", "mode_router.toml"] {
        let router_path = model_cfg_dir.join(router_name);
        if router_path.exists() {
            let mut cfg = load_agent_config(&router_path)?;
            // Use the optimal temperature (should be 0.0-0.2 for determinism)
            cfg.temperature = temp_f64;
            save_agent_config(&router_path, &cfg)?;
        }
    }

    Ok(())
}
