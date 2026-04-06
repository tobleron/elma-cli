//! @efficiency-role: service-orchestrator
//!
//! JSON Temperature Tuning - Find optimal temperature for reliable JSON output
//!
//! Tests model at temperatures 0.0 to 1.0 (step 0.1) across three difficulty levels.
//! Selects the temperature that produces the most reliable, repairable JSON.

use std::cmp::Ordering;

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
            "low" => Self::Low,
            "hard" => Self::Hard,
            _ => Self::Medium,
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

/// Test a single scenario at a given temperature
async fn test_scenario(
    client: &reqwest::Client,
    chat_url: &Url,
    model: &str,
    temperature: f32,
    scenario: &JsonTuningScenario,
) -> Result<(bool, bool, bool)> {
    let scenario_path = repo_root()?
        .join("scenarios/json_tune")
        .join(&scenario.file);
    let content = std::fs::read_to_string(&scenario_path)
        .with_context(|| format!("read {}", scenario_path.display()))?;

    let user_message = content
        .lines()
        .find(|l| l.starts_with("user:"))
        .map(|l| l.trim_start_matches("user:").trim())
        .unwrap_or(&content);

    let _difficulty = JsonDifficulty::from_str(&scenario.difficulty);

    let req = build_chat_request(model, user_message, temperature);
    let response = chat_once(client, chat_url, &req).await;

    match response {
        Err(_) => Ok((false, false, true)),
        Ok(resp) => {
            let text = extract_response_text(&resp);
            Ok(classify_json_response(&text))
        }
    }
}

/// Test JSON output at a specific temperature
pub(crate) async fn test_json_at_temperature(
    client: &reqwest::Client,
    chat_url: &Url,
    model: &str,
    temperature: f32,
    scenarios: &[JsonTuningScenario],
) -> Result<TemperatureResult> {
    let mut total = 0usize;
    let mut valid = 0usize;
    let mut repairable = 0usize;
    let mut failed = 0usize;
    let mut total_attempts = 0usize;

    for scenario in scenarios {
        total += 1;
        total_attempts += 1;

        let (v, r, f) = test_scenario(client, chat_url, model, temperature, scenario).await?;
        if v {
            valid += 1;
            repairable += r as usize;
        }
        failed += f as usize;
    }

    let (weighted_score, avg_parse_attempts) =
        compute_scores(valid, repairable, total, total_attempts);

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

// ---------------------------------------------------------------------------
// Private helpers for test_json_at_temperature
// ---------------------------------------------------------------------------

fn build_chat_request(model: &str, user_message: &str, temperature: f32) -> ChatCompletionRequest {
    ChatCompletionRequest {
        model: model.to_string(),
        messages: vec![
            ChatMessage::simple("system", "Return ONLY valid JSON. No prose, no explanations, no markdown fences. Just raw JSON."),
            ChatMessage::simple("user", user_message),
        ],
        temperature: temperature as f64, top_p: 0.95, stream: false, max_tokens: 2048,
        n_probs: None, repeat_penalty: Some(1.1), reasoning_format: Some("none".to_string()),
        grammar: None,
        tools: None,
    }
}

/// Returns (valid, is_repairable, is_failed) for a single response text.
fn classify_json_response(text: &str) -> (bool, bool, bool) {
    let pr = crate::json_parser::parse_intel_output(text, &[("1", "VALID"), ("2", "INVALID")]);
    if pr.label.is_some() {
        let repairable = matches!(
            pr.parse_method,
            crate::json_parser::ParseMethod::JsonDirect
                | crate::json_parser::ParseMethod::JsonMarkdown
                | crate::json_parser::ParseMethod::JsonExtracted
        );
        return if pr.parse_method == crate::json_parser::ParseMethod::Failed {
            try_last_resort_repair(text)
        } else {
            (true, repairable, false)
        };
    }
    try_last_resort_repair(text)
}

fn try_last_resort_repair(text: &str) -> (bool, bool, bool) {
    match jsonrepair_rs::jsonrepair(text) {
        Ok(r) => {
            if serde_json::from_str::<serde_json::Value>(&r).is_ok() {
                (true, true, false)
            } else {
                (false, false, true)
            }
        }
        Err(_) => (false, false, true),
    }
}

fn compute_scores(
    valid: usize,
    repairable: usize,
    total: usize,
    total_attempts: usize,
) -> (f32, f32) {
    let base = if total > 0 {
        valid as f32 / total as f32
    } else {
        0.0
    };
    let bonus = if valid > 0 {
        repairable as f32 / valid as f32 * 0.1
    } else {
        0.0
    };
    let avg = if total > 0 {
        total_attempts as f32 / total as f32
    } else {
        0.0
    };
    (base + bonus, avg)
}

/// Process a single temperature, using cache if available
async fn process_temperature(
    client: &reqwest::Client,
    chat_url: &Url,
    model_id: &str,
    scenarios: &[JsonTuningScenario],
    cached_result: &Option<JsonTuningResult>,
    temp: f32,
    emit_progress: bool,
    args: &Args,
) -> Result<TemperatureResult> {
    if let Some(cached_temp_result) = try_load_cached_temp(cached_result.as_ref(), temp) {
        if emit_progress {
            emit_temp_result_progress(args, &cached_temp_result, true);
        }
        return Ok(cached_temp_result);
    }

    let result = test_json_at_temperature(client, chat_url, model_id, temp, scenarios).await?;

    if emit_progress {
        emit_temp_result_progress(args, &result, false);
    }
    Ok(result)
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
            &format!(
                "JSON temperature tuning for {} ({} scenarios)",
                model_id,
                scenarios.len()
            ),
        );
    }

    let tune_dir = model_cfg_dir.join("tune").join("json");
    let cached_result = load_cached_json_tuning_result(&tune_dir)?;

    if let Some(ref cached) = cached_result {
        if cached.results_by_temp.len() == TEMPERATURES.len() {
            if emit_progress {
                emit_full_cached_progress(args, cached);
            }
            return Ok(cached.clone());
        }
    }

    let mut results_by_temp = Vec::new();

    for &temp in &TEMPERATURES {
        let result = process_temperature(
            client,
            chat_url,
            model_id,
            scenarios,
            &cached_result,
            temp,
            emit_progress,
            args,
        )
        .await?;
        results_by_temp.push(result);
    }

    let optimal = find_optimal_temperature(&results_by_temp);
    let passing_temps = results_by_temp
        .iter()
        .filter(|r| r.failed_count == 0)
        .count();
    let total_temps = TEMPERATURES.len();
    let recommended = find_recommended_temperature(&results_by_temp, &optimal);

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

// ---------------------------------------------------------------------------
// Private helpers for run_json_temperature_tuning
// ---------------------------------------------------------------------------

fn try_load_cached_temp(cached: Option<&JsonTuningResult>, temp: f32) -> Option<TemperatureResult> {
    cached?
        .results_by_temp
        .iter()
        .find(|r| (r.temperature - temp).abs() < 0.01)
        .cloned()
}

fn emit_full_cached_progress(args: &Args, cached: &JsonTuningResult) {
    for r in &cached.results_by_temp {
        emit_temp_result_progress(args, r, true);
    }
    calibration_progress(args, &format!(
        "  ✓ JSON tuning complete: {} of {} temperatures reliable, recommended temp={:.2} (all cached)",
        cached.passing_temperatures, cached.total_temperatures, cached.recommended_temperature));
}

fn emit_temp_result_progress(args: &Args, result: &TemperatureResult, cached: bool) {
    let status = if result.failed_count == 0 {
        "✓"
    } else {
        "✗"
    };
    let suffix = if cached { " (cached)" } else { "" };
    let note = if result.failed_count > 0 && !cached {
        format!(" ({} failed)", result.failed_count)
    } else {
        String::new()
    };
    calibration_progress(
        args,
        &format!(
            "  temp={:.1}: {}/{} valid {}{}{}",
            result.temperature, result.valid_json_count, result.total_tests, status, note, suffix
        ),
    );
}

fn default_temp_result() -> TemperatureResult {
    TemperatureResult {
        temperature: 0.2,
        total_tests: 0,
        valid_json_count: 0,
        repairable_count: 0,
        failed_count: 0,
        avg_parse_attempts: 0.0,
        weighted_score: 0.0,
    }
}

fn find_optimal_temperature(results: &[TemperatureResult]) -> TemperatureResult {
    results
        .iter()
        .max_by(|a, b| {
            a.weighted_score
                .partial_cmp(&b.weighted_score)
                .unwrap_or(Ordering::Equal)
        })
        .cloned()
        .unwrap_or_else(default_temp_result)
}

fn find_recommended_temperature(results: &[TemperatureResult], optimal: &TemperatureResult) -> f32 {
    if optimal.weighted_score < 0.9 {
        return optimal.temperature;
    }
    results
        .iter()
        .filter(|r| r.weighted_score >= optimal.weighted_score - 0.05)
        .min_by(|a, b| {
            a.temperature
                .partial_cmp(&b.temperature)
                .unwrap_or(Ordering::Equal)
        })
        .map(|r| r.temperature)
        .unwrap_or(optimal.temperature)
}

/// Load cached JSON tuning result (most recent)
pub(crate) fn load_cached_json_tuning_result(tune_dir: &Path) -> Result<Option<JsonTuningResult>> {
    if !tune_dir.exists() {
        return Ok(None);
    }
    match find_latest_tuning_file(tune_dir)? {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("read {}", path.display()))?;
            Ok(parse_cached_result(&content))
        }
        None => Ok(None),
    }
}

fn find_latest_tuning_file(tune_dir: &Path) -> Result<Option<std::path::PathBuf>> {
    let (mut latest_path, mut latest_ts) = (None, 0i64);
    for entry in
        std::fs::read_dir(tune_dir).with_context(|| format!("read_dir {}", tune_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(ts_str) = name.strip_prefix("json_tuning_") else {
            continue;
        };
        let Ok(ts) = ts_str.parse::<i64>() else {
            continue;
        };
        if ts > latest_ts {
            latest_ts = ts;
            latest_path = Some(path);
        }
    }
    Ok(latest_path)
}

/// Parse cached result from TOML content
fn parse_cached_result(content: &str) -> Option<JsonTuningResult> {
    let mut results_by_temp: Vec<TemperatureResult> = Vec::new();
    let (mut in_temps, mut passing, mut total) = (false, 0, 11);
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("passing_temperatures") {
            passing = line
                .split('=')
                .nth(1)
                .map(|v| v.trim().parse().unwrap_or(0))
                .unwrap_or(0);
        } else if line.starts_with("total_temperatures") {
            total = line
                .split('=')
                .nth(1)
                .map(|v| v.trim().parse().unwrap_or(11))
                .unwrap_or(total);
        } else if line == "[[temperatures]]" {
            in_temps = true;
        } else if line.starts_with("[[") {
            in_temps = false;
        } else if in_temps && line.starts_with('{') && line.ends_with('}') {
            if let Some(r) = parse_temp_result_line(line) {
                results_by_temp.push(r);
            }
        }
    }
    if results_by_temp.is_empty() {
        return None;
    }
    let optimal = results_by_temp
        .iter()
        .max_by(|a, b| {
            a.weighted_score
                .partial_cmp(&b.weighted_score)
                .unwrap_or(Ordering::Equal)
        })
        .cloned()?;
    let recommended = find_recommended_temperature(&results_by_temp, &optimal);
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
    let (mut temperature, mut valid, mut total, mut repairable, mut failed, mut score) =
        (0.0f32, 0, 0, 0, 0, 0.0f32);
    for part in line.trim_matches(|c| c == '{' || c == '}').split(',') {
        let part = part.trim();
        if let Some((key, value)) = part.split_once('=') {
            let (k, v) = (key.trim(), value.trim());
            match k {
                "temperature" => temperature = v.parse().ok()?,
                "valid" => valid = v.parse().ok()?,
                "total" => total = v.parse().ok()?,
                "repairable" => repairable = v.parse().ok()?,
                "failed" => failed = v.parse().ok()?,
                "score" => score = v.parse().ok()?,
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

/// Build TOML content for a JSON tuning report
fn build_tuning_report_content(result: &JsonTuningResult, timestamp: u64) -> String {
    let mut c = String::new();
    c.push_str("# JSON Temperature Tuning Report\n");
    c.push_str(&format!("timestamp = {}\n", timestamp));
    c.push_str(&format!(
        "optimal_temperature = {:.2}\n",
        result.optimal_temperature
    ));
    c.push_str(&format!(
        "recommended_temperature = {:.2}\n",
        result.recommended_temperature
    ));
    c.push_str(&format!(
        "passing_temperatures = {}\n",
        result.passing_temperatures
    ));
    c.push_str(&format!(
        "total_temperatures = {}\n\n",
        result.total_temperatures
    ));
    c.push_str("# Results by Temperature\n[[temperatures]]\n");
    for r in &result.results_by_temp {
        c.push_str(&format!(
            "  {{ temperature = {:.1}, valid = {}, total = {}, repairable = {}, failed = {}, score = {:.3} }}\n",
            r.temperature, r.valid_json_count, r.total_tests, r.repairable_count, r.failed_count, r.weighted_score));
    }
    c
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
    std::fs::write(&report_path, build_tuning_report_content(result, timestamp))
        .with_context(|| format!("write {}", report_path.display()))?;
    eprintln!("[JSON_TUNING] Report saved to {}", report_path.display());
    Ok(())
}

/// Apply optimal temperature to orchestrator profiles
pub(crate) fn apply_json_tuning_temperature(model_cfg_dir: &Path, temperature: f32) -> Result<()> {
    let temp_f64 = temperature as f64;
    for name in &[
        "orchestrator.toml",
        "workflow_planner.toml",
        "json_outputter.toml",
        "router.toml",
        "speech_act.toml",
        "mode_router.toml",
    ] {
        let path = model_cfg_dir.join(name);
        if path.exists() {
            let mut cfg = load_agent_config(&path)?;
            cfg.temperature = temp_f64;
            save_agent_config(&path, &cfg)?;
        }
    }
    Ok(())
}
