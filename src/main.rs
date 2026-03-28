use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Url;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::IsTerminal;
use std::io::{self, Write};
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(
    name = "elma-cli",
    version,
    about = "Minimal chat CLI for llama.cpp /v1/chat/completions"
)]
struct Args {
    /// Base URL of the server (example: http://192.168.1.186:8080)
    #[arg(long, env = "LLAMA_BASE_URL")]
    base_url: Option<String>,

    /// Optional model override. If omitted, we fetch the first model id from GET /v1/models.
    #[arg(long, env = "LLAMA_MODEL")]
    model: Option<String>,

    /// Root config directory (model-specific folders will be created under it).
    #[arg(long, default_value = "config")]
    config_root: String,

    /// Root sessions directory.
    #[arg(long, default_value = "sessions")]
    sessions_root: String,

    /// Print model thinking (reasoning_content) if present.
    #[arg(long, default_value_t = true)]
    show_thinking: bool,

    /// Disable ANSI colors.
    #[arg(long, default_value_t = false)]
    no_color: bool,

    /// Run tuning for all models exposed by the endpoint, then exit.
    #[arg(long, default_value_t = false)]
    tune: bool,

    /// Run calibration only for the selected model(s), then exit.
    #[arg(long, default_value_t = false)]
    calibrate: bool,

    /// When tuning or calibrating, target all models exposed by the endpoint.
    #[arg(long, default_value_t = false)]
    all_models: bool,

    /// Restore the immutable baseline profile set for the selected model, then exit.
    #[arg(long, default_value_t = false)]
    restore_base: bool,

    /// Restore the last active profile set for the selected model, then exit.
    #[arg(long, default_value_t = false)]
    restore_last: bool,

    /// Show raw machine trace lines in the terminal.
    #[arg(long, default_value_t = false)]
    debug_trace: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct Profile {
    version: u32,
    name: String,
    base_url: String,
    model: String,
    temperature: f64,
    top_p: f64,
    repeat_penalty: f64,
    reasoning_format: String,
    max_tokens: u32,
    timeout_s: u64,
    system_prompt: String,
}

fn repo_root() -> Result<PathBuf> {
    // Best-effort: assume current working directory is the repo root for now.
    std::env::current_dir().context("Failed to get current directory")
}

fn config_root_path(config_root: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join(config_root))
}

fn sessions_root_path(sessions_root: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join(sessions_root))
}

fn discover_saved_base_url(config_root: &Path, model_hint: Option<&str>) -> Option<String> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(model_id) = model_hint {
        let hinted = config_root.join(sanitize_model_folder_name(model_id));
        if hinted.is_dir() {
            candidates.push(hinted);
        }
    }

    if let Ok(rd) = std::fs::read_dir(config_root) {
        let mut dirs: Vec<PathBuf> = rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        for dir in dirs {
            if !candidates.contains(&dir) {
                candidates.push(dir);
            }
        }
    }

    for dir in candidates {
        let elma_cfg_path = dir.join("_elma.config");
        if elma_cfg_path.exists() {
            if let Ok(cfg) = load_agent_config(&elma_cfg_path) {
                let url = cfg.base_url.trim();
                if !url.is_empty() {
                    return Some(url.to_string());
                }
            }
        }

        let router_cal_path = dir.join("router_calibration.toml");
        if router_cal_path.exists() {
            if let Ok(cal) = load_router_calibration(&router_cal_path) {
                let url = cal.base_url.trim();
                if !url.is_empty() {
                    return Some(url.to_string());
                }
            }
        }
    }

    None
}

fn resolve_base_url(
    config_root: &Path,
    explicit: Option<&str>,
    model_hint: Option<&str>,
) -> (String, &'static str) {
    if let Some(url) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return (url.to_string(), "cli_or_env");
    }
    if let Some(url) = discover_saved_base_url(config_root, model_hint) {
        return (url, "saved_config");
    }
    ("http://localhost:8080".to_string(), "fallback_default")
}

fn load_agent_config(path: &PathBuf) -> Result<Profile> {
    let bytes = std::fs::read(&path)
        .with_context(|| format!("Failed to read config file at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("config file is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_agent_config(path: &PathBuf, p: &Profile) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(p).context("Failed to serialize config toml")?;
    std::fs::write(&path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn save_router_calibration(path: &PathBuf, c: &RouterCalibration) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(c).context("Failed to serialize router calibration toml")?;
    std::fs::write(&path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn load_router_calibration(path: &PathBuf) -> Result<RouterCalibration> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read router calibration at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("router calibration is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_active_manifest(path: &PathBuf, m: &ActiveManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(m).context("Failed to serialize active manifest toml")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn load_active_manifest(path: &PathBuf) -> Result<ActiveManifest> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read active manifest at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("active manifest is not valid UTF-8")?;
    toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_tune_run_manifest(path: &PathBuf, m: &TuneRunManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s = toml::to_string_pretty(m).context("Failed to serialize tune run manifest toml")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RouterCalibration {
    version: u32,
    model: String,
    base_url: String,
    n_probs: u32,
    supports_logprobs: bool,
    routes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ActiveManifest {
    version: u32,
    model: String,
    active_source: String,
    active_run_id: Option<String>,
    activated_unix_s: u64,
    final_score: f64,
    certified: bool,
    restore_last_dir: String,
    restore_base_dir: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct TuneRunManifest {
    version: u32,
    run_id: String,
    model: String,
    mode: String,
    started_unix_s: u64,
    activated: bool,
    final_score: f64,
    certified: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct CalibrationManifest {
    version: u32,
    scenarios: Vec<CalibrationScenario>,
}

#[derive(Debug, Clone, Deserialize)]
struct CalibrationScenario {
    #[serde(default)]
    suite: String,
    file: String,
    speech_act: String,
    workflow: String,
    #[serde(default)]
    mode: Option<String>,
    route: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    expected_formula: Option<String>,
    #[serde(default)]
    expected_scope_terms: Vec<String>,
    #[serde(default)]
    forbidden_scope_terms: Vec<String>,
    #[serde(default)]
    expected_answer_keywords: Vec<String>,
    #[serde(default)]
    avoid_answer_keywords: Vec<String>,
    #[serde(default)]
    expected_categories: Vec<String>,
    #[serde(default)]
    minimum_step_count: Option<usize>,
    #[serde(default)]
    maximum_step_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalibrationMetric {
    total: usize,
    correct: usize,
    accuracy: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalibrationConfusion {
    expected: String,
    predicted: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ScenarioCalibrationResult {
    suite: String,
    file: String,
    notes: String,
    speech_act_expected: String,
    speech_act_predicted: String,
    speech_act_probability: f64,
    speech_act_ok: bool,
    workflow_expected: String,
    workflow_predicted: String,
    workflow_probability: f64,
    workflow_ok: bool,
    mode_expected: Option<String>,
    mode_predicted: Option<String>,
    mode_probability: Option<f64>,
    mode_ok: Option<bool>,
    route_expected: String,
    route_predicted: String,
    route_probability: f64,
    route_ok: bool,
    program_signature: String,
    program_parse_ok: bool,
    program_parse_error: String,
    program_shape_ok: bool,
    program_shape_reason: String,
    program_policy_ok: bool,
    program_policy_reason: String,
    program_consistency_ok: bool,
    executed_in_tune: bool,
    execution_ok: Option<bool>,
    critic_ok: Option<bool>,
    critic_reason: Option<String>,
    response_ok: Option<bool>,
    response_reason: Option<String>,
    response_plain_text: Option<bool>,
    scope_ok: Option<bool>,
    scope_reason: Option<String>,
    compaction_ok: Option<bool>,
    compaction_reason: Option<String>,
    classification_ok: Option<bool>,
    classification_reason: Option<String>,
    claim_check_ok: Option<bool>,
    claim_check_reason: Option<String>,
    presentation_ok: Option<bool>,
    presentation_reason: Option<String>,
    tool_economy_score: Option<f64>,
    all_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalibrationSummary {
    total_cases: usize,
    speech_act: CalibrationMetric,
    workflow: CalibrationMetric,
    mode: CalibrationMetric,
    route: CalibrationMetric,
    program_parse: CalibrationMetric,
    program_shape: CalibrationMetric,
    program_policy: CalibrationMetric,
    program_consistency: CalibrationMetric,
    execution: CalibrationMetric,
    critic: CalibrationMetric,
    response: CalibrationMetric,
    scope: CalibrationMetric,
    compaction: CalibrationMetric,
    classification: CalibrationMetric,
    claim_check: CalibrationMetric,
    presentation: CalibrationMetric,
    all_ok: CalibrationMetric,
    certified: bool,
    certification_rule: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CalibrationReport {
    version: u32,
    model: String,
    base_url: String,
    supports_logprobs: bool,
    n_probs: u32,
    summary: CalibrationSummary,
    speech_act_confusions: Vec<CalibrationConfusion>,
    workflow_confusions: Vec<CalibrationConfusion>,
    mode_confusions: Vec<CalibrationConfusion>,
    route_confusions: Vec<CalibrationConfusion>,
    scenarios: Vec<ScenarioCalibrationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EfficiencyMetric {
    total: usize,
    score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EfficiencyScenarioResult {
    suite: String,
    file: String,
    task_success: bool,
    grounding_ok: Option<bool>,
    scope_ok: Option<bool>,
    compaction_ok: Option<bool>,
    classification_ok: Option<bool>,
    claim_check_ok: Option<bool>,
    presentation_ok: Option<bool>,
    tool_economy_score: f64,
    actual_steps: usize,
    expected_min_steps: Option<usize>,
    expected_max_steps: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EfficiencySummary {
    total_cases: usize,
    task_success_rate: EfficiencyMetric,
    grounding_rate: EfficiencyMetric,
    scope_precision: EfficiencyMetric,
    compaction_rate: EfficiencyMetric,
    classification_rate: EfficiencyMetric,
    claim_check_rate: EfficiencyMetric,
    presentation_rate: EfficiencyMetric,
    tool_economy: EfficiencyMetric,
    overall_efficiency: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EfficiencyReport {
    version: u32,
    model: String,
    base_url: String,
    summary: EfficiencySummary,
    scenarios: Vec<EfficiencyScenarioResult>,
}

#[derive(Debug, Clone)]
struct ProgramEvaluation {
    parsed: bool,
    parse_error: String,
    shape_ok: bool,
    shape_reason: String,
    policy_ok: bool,
    policy_reason: String,
    executable_in_tune: bool,
    signature: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CalibrationJudgeVerdict {
    status: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    answered_request: bool,
    #[serde(default)]
    faithful_to_evidence: bool,
    #[serde(default)]
    plain_text: bool,
}

#[derive(Debug, Clone)]
struct CandidateScore {
    name: String,
    dir: PathBuf,
    report: CalibrationReport,
    score: f64,
    hard_rejected: bool,
}

#[derive(Debug, Clone)]
struct SearchCandidate {
    name: String,
    dir: PathBuf,
    score: f64,
    hard_rejected: bool,
}

#[derive(Debug, Clone)]
struct RouteDecision {
    route: String,
    source: String,
    distribution: Vec<(String, f64)>,
    margin: f64,
    entropy: f64,
    speech_act: ProbabilityDecision,
    workflow: ProbabilityDecision,
    mode: ProbabilityDecision,
}

#[derive(Debug, Clone)]
struct ProbabilityDecision {
    choice: String,
    source: String,
    distribution: Vec<(String, f64)>,
    margin: f64,
    entropy: f64,
}

#[derive(Debug, Clone, Deserialize)]
struct Program {
    objective: String,
    steps: Vec<Step>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct StepCommon {
    #[serde(default)]
    purpose: String,
    #[serde(default)]
    depends_on: Vec<String>,
    #[serde(default)]
    success_condition: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum Step {
    #[serde(rename = "shell")]
    Shell {
        id: String,
        cmd: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "plan")]
    Plan {
        id: String,
        goal: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "masterplan")]
    MasterPlan {
        id: String,
        goal: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "decide")]
    Decide {
        id: String,
        prompt: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "summarize")]
    Summarize {
        id: String,
        #[serde(default)]
        text: String,
        #[serde(default)]
        instructions: String,
        #[serde(flatten)]
        common: StepCommon,
    },
    #[serde(rename = "reply")]
    Reply {
        id: String,
        instructions: String,
        #[serde(flatten)]
        common: StepCommon,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct CriticVerdict {
    status: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    program: Option<Program>,
}

#[derive(Debug, Clone)]
struct StepResult {
    id: String,
    kind: String,
    purpose: String,
    depends_on: Vec<String>,
    success_condition: String,
    ok: bool,
    summary: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct ComplexityAssessment {
    #[serde(default)]
    complexity: String,
    #[serde(default)]
    needs_evidence: bool,
    #[serde(default)]
    needs_tools: bool,
    #[serde(default)]
    needs_decision: bool,
    #[serde(default)]
    needs_plan: bool,
    #[serde(default)]
    risk: String,
    #[serde(default)]
    suggested_pattern: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct FormulaSelection {
    #[serde(default)]
    primary: String,
    #[serde(default)]
    alternatives: Vec<String>,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    memory_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct CommandRepair {
    #[serde(default)]
    cmd: String,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct ScopePlan {
    #[serde(default)]
    objective: String,
    #[serde(default)]
    focus_paths: Vec<String>,
    #[serde(default)]
    include_globs: Vec<String>,
    #[serde(default)]
    exclude_globs: Vec<String>,
    #[serde(default)]
    query_terms: Vec<String>,
    #[serde(default)]
    expected_artifacts: Vec<String>,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct EvidenceCompact {
    #[serde(default)]
    summary: String,
    #[serde(default)]
    key_facts: Vec<String>,
    #[serde(default)]
    noise: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct ArtifactClassification {
    #[serde(default)]
    safe: Vec<String>,
    #[serde(default)]
    maybe: Vec<String>,
    #[serde(default)]
    keep: Vec<String>,
    #[serde(default)]
    ignore: Vec<String>,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct ClaimCheckVerdict {
    #[serde(default)]
    status: String,
    #[serde(default)]
    reason: String,
    #[serde(default)]
    unsupported_claims: Vec<String>,
    #[serde(default)]
    missing_points: Vec<String>,
    #[serde(default)]
    rewrite_instructions: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FormulaMemoryRecord {
    id: String,
    created_unix_s: u64,
    user_message: String,
    route: String,
    complexity: String,
    formula: String,
    objective: String,
    title: String,
    program_signature: String,
}

#[derive(Debug, Deserialize)]
struct ModelsList {
    data: Option<Vec<ModelItem>>,
    models: Option<Vec<ModelItem>>, // some servers return both
}

#[derive(Debug, Deserialize)]
struct ModelItem {
    id: Option<String>,
    name: Option<String>,
    model: Option<String>,
}

async fn fetch_first_model_id(client: &reqwest::Client, base_url: &Url) -> Result<String> {
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        anyhow::bail!("GET /v1/models returned HTTP {status}: {text}");
    }
    let parsed: ModelsList = serde_json::from_str(&text).context("Invalid JSON from /v1/models")?;
    let list = parsed
        .data
        .or(parsed.models)
        .unwrap_or_default()
        .into_iter();
    for item in list {
        if let Some(id) = item.id.or(item.name).or(item.model) {
            if !id.trim().is_empty() {
                return Ok(id);
            }
        }
    }
    anyhow::bail!("No model ids found in /v1/models response")
}

async fn fetch_all_model_ids(client: &reqwest::Client, base_url: &Url) -> Result<Vec<String>> {
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        anyhow::bail!("GET /v1/models returned HTTP {status}: {text}");
    }
    let parsed: ModelsList = serde_json::from_str(&text).context("Invalid JSON from /v1/models")?;
    let mut out = Vec::new();
    let list = parsed.data.or(parsed.models).unwrap_or_default();
    for item in list {
        if let Some(id) = item.id.or(item.name).or(item.model) {
            let id = id.trim().to_string();
            if !id.is_empty() && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    if out.is_empty() {
        anyhow::bail!("No model ids found in /v1/models response");
    }
    Ok(out)
}

async fn fetch_ctx_max(client: &reqwest::Client, base_url: &Url) -> Result<Option<u64>> {
    // Best-effort, ordered by "most likely runtime truth":
    // 1) /slots[0].n_ctx (runtime ctx size)
    // 2) /props.default_generation_settings.n_ctx (runtime default)
    // 3) /v1/models meta.n_ctx_train (training ctx, can be larger than runtime)

    // 1) /slots
    if let Ok(url) = base_url.join("/slots") {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let n = v
                            .get(0)
                            .and_then(|s| s.get("n_ctx"))
                            .and_then(|x| x.as_u64());
                        if n.is_some() {
                            return Ok(n);
                        }
                    }
                }
            }
        }
    }

    // 2) /props
    if let Ok(url) = base_url.join("/props") {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let n = v
                            .get("default_generation_settings")
                            .and_then(|d| d.get("n_ctx"))
                            .and_then(|x| x.as_u64());
                        if n.is_some() {
                            return Ok(n);
                        }
                    }
                }
            }
        }
    }

    // 3) /v1/models
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        return Ok(None);
    }
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    Ok(v.get("data")
        .and_then(|d| d.get(0))
        .and_then(|m| m.get("meta"))
        .and_then(|meta| meta.get("n_ctx_train"))
        .and_then(|x| x.as_u64()))
}

fn sanitize_model_folder_name(s: &str) -> String {
    // Keep it filesystem-safe and stable.
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('_');
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn default_elma_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "_elma".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.6,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "auto".to_string(),
        max_tokens: 4096,
        timeout_s: 120,
        // Only Elma is self-aware by name.
        system_prompt: "You are Elma.\n\nYou are a helpful, faithful assistant.\nUse the provided WORKSPACE CONTEXT facts.\n\nOutput formatting:\n- Do not use Markdown unless the user explicitly asks for Markdown.\n- Prefer plain text suitable for a terminal.\n\nKeep responses concise."
            .to_string(),
    }
}

fn default_intention_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "intention".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You are an expert intent classifier.\n\nGiven the user's message, respond with exactly ONE WORD that best describes the user's intent.\n\nSTRICT RULES:\n- Output must be exactly one word.\n- Output must match: ^[A-Za-z]+$\n- No punctuation.\n- No explanation.\n- No quotes.\n"
            .to_string(),
    }
}

fn default_gate_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "gate".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 8,
        timeout_s: 120,
        system_prompt: "Classify the user's message into exactly one token.\n\nReturn exactly one of:\nCHAT\nACTION\n\nGuidance:\n- ACTION if the user wants any terminal/workspace action (commands, file operations, build/test, search, etc).\n- CHAT otherwise.\n\nNo other text."
            .to_string(),
    }
}

fn default_gate_why_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "gate_why".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 64,
        timeout_s: 120,
        system_prompt: "Explain in exactly ONE short sentence why you classified the user message as CHAT (not ACTION). Do not include any extra lines."
            .to_string(),
    }
}

fn default_tooler_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "tooler".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You are an expert shell user.\n\nGiven a user's request, output exactly one line of JSON.\nSchema:\n{\"type\":\"shell\",\"cmd\":\"<one-liner>\"}\n\nRules:\n- cmd must be a single shell one-liner.\n- Do not include markdown.\n- Do not include explanations.\n- Prefer robust, common commands (e.g. use \"ls -l\" or \"ls -la\", never incomplete flags like \"ls -\").\n- If the request is not actionable in a shell, still output a safe no-op command (e.g. \"true\")."
            .to_string(),
    }
}

fn default_orchestrator_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "orchestrator".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.2,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "You are Elma's reasoning orchestrator.\n\nReturn ONLY one valid JSON object. No prose. No code fences. No backticks.\n\nSTRICT JSON RULES:\n- The first character must be '{'.\n- The last character must be '}'.\n- No text before or after the JSON object.\n\nYour JSON is a Program with steps executed in order.\n\nSchema:\n{\n  \"objective\": \"string\",\n  \"steps\": [\n    {\"id\":\"s1\",\"type\":\"shell\",\"cmd\":\"<one liner>\"},\n    {\"id\":\"p1\",\"type\":\"plan\",\"goal\":\"...\"},\n    {\"id\":\"m1\",\"type\":\"masterplan\",\"goal\":\"...\"},\n    {\"id\":\"d1\",\"type\":\"decide\",\"prompt\":\"...\"},\n    {\"id\":\"sum1\",\"type\":\"summarize\",\"text\":\"...\",\"instructions\":\"...\"},\n    {\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"...\"}\n  ]\n}\n\nROUTER PRIOR RULES:\n- You will receive a probabilistic route prior over CHAT, SHELL, PLAN, MASTERPLAN, and DECIDE.\n- Treat the route prior as evidence, not a hard rule.\n- If the route prior is uncertain or the user request is genuinely ambiguous, you may output a Program with a single reply step that asks one concise clarifying question.\n\nEVIDENCE-FIRST RULES:\n- If the request is about the current project, codebase, files, functions, symbols, or config, you must inspect workspace evidence before replying.\n- If the request names a file, inspect that file first.\n- If the request names a function or symbol, use rg in source files and exclude target/.\n- Prefer rg over grep.\n- A shell step is for real workspace inspection or execution only. Never use shell steps to print prose, plan lines, or explanations.\n- If the user asks for one concrete step-by-step plan, use a plan step.\n- If the user asks for a higher-level overall plan across phases, use a masterplan step.\n- Do not emit plan text through shell commands.\n- Do not invent file paths, symbols, signatures, or repo facts.\n- Do not include network, remote, or destructive commands.\n- If no tool use is needed, output a Program with a single reply step.\n- reply step must instruct the final assistant response in plain terminal text with no Markdown unless the user asked for it.\n\nExamples:\nUser: What is my current project about?\nOutput:\n{\"objective\":\"understand current project from workspace evidence\",\"steps\":[{\"id\":\"s1\",\"type\":\"shell\",\"cmd\":\"cat Cargo.toml\"},{\"id\":\"s2\",\"type\":\"shell\",\"cmd\":\"rg -n --glob '!target/**' '^(fn|struct|enum|mod|pub fn|pub struct)' src config tests || true\"},{\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"Using the shell outputs as evidence, explain what the current project is about in plain text. Mention uncertainty if the evidence is incomplete.\"}]}\n\nUser: find where fetch_ctx_max is defined and show me the function signature\nOutput:\n{\"objective\":\"locate symbol definition and report its signature from source\",\"steps\":[{\"id\":\"s1\",\"type\":\"shell\",\"cmd\":\"rg -n --glob '!target/**' '^((async )?fn) fetch_ctx_max' src || true\"},{\"id\":\"s2\",\"type\":\"shell\",\"cmd\":\"sed -n '1,260p' src/main.rs\"},{\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"Using only the shell outputs, tell the user where fetch_ctx_max is defined and show the exact function signature in plain text without Markdown.\"}]}\n"
            .to_string(),
    }
}

fn default_critic_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "critic".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You are Elma's execution critic.\n\nReturn ONLY one valid JSON object. No prose. No code fences.\n\nSchema:\n{\n  \"status\": \"ok\" | \"retry\",\n  \"reason\": \"one short sentence\",\n  \"program\": <Program>\n}\n\nRules:\n- Omit program or set it to null when status is ok.\n- If the request is about project/code/files/functions/symbols and there is no workspace evidence in the step results, choose retry.\n- If the user asked for a step-by-step plan and there is no plan step result, choose retry and provide a corrected Program that uses type \"plan\".\n- If the user asked for an overall or master plan and there is no masterplan step result, choose retry and provide a corrected Program that uses type \"masterplan\".\n- If a shell step only prints prose or plan text instead of inspecting or executing something real in the workspace, choose retry.\n- If the result is incomplete, unsupported by workspace evidence, or likely hallucinated, choose retry and provide a corrected Program.\n- Do not invent file paths or outputs.\n"
            .to_string(),
    }
}

fn default_router_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "router".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1,
        timeout_s: 120,
        system_prompt: "You are Elma's workflow gate estimator.\n\nReturn exactly one digit and nothing else.\n\nMapping:\n1 = CHAT\n2 = WORKFLOW\n\nInterpretation:\n- 1 CHAT: answer directly without an internal workflow.\n- 2 WORKFLOW: use internal reasoning steps, workspace evidence, or another intel unit before the final answer.\n\nImportant distinctions:\n- Greetings or general knowledge questions are usually 1.\n- Questions about the current project, files, code, commands, or tasks that need planning or decisions are usually 2.\n\nRules:\n- Output must be exactly one digit from 1 to 2.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents whether Elma should enter workflow mode.\n".to_string(),
    }
}

fn default_mode_router_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "mode_router".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1,
        timeout_s: 120,
        system_prompt: "You are Elma's workflow mode estimator.\n\nReturn exactly one digit and nothing else.\n\nMapping:\n1 = INSPECT\n2 = EXECUTE\n3 = PLAN\n4 = MASTERPLAN\n5 = DECIDE\n\nInterpretation:\n- 1 INSPECT: inspect workspace evidence, files, code, or configuration.\n- 2 EXECUTE: run commands or carry out direct terminal actions.\n- 3 PLAN: create one concrete step-by-step plan.\n- 4 MASTERPLAN: create a higher-level overall plan across phases.\n- 5 DECIDE: return a concise decision or label.\n\nImportant distinctions:\n- \"What is my current project about?\", \"read Cargo.toml and summarize it\", and \"find where fetch_ctx_max is defined\" are usually 1.\n- \"list files\", \"run tests\", and \"build the project\" are usually 2.\n- \"Create a step-by-step plan\" is 3, not 4.\n- Only choose 4 when the user truly wants an overall master plan.\n\nRules:\n- Output must be exactly one digit from 1 to 5.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents the workflow mode.\n".to_string(),
    }
}

fn default_speech_act_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "speech_act".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1,
        timeout_s: 120,
        system_prompt: "You are Elma's speech-act estimator.\n\nReturn exactly one digit and nothing else.\n\nMapping:\n1 = CAPABILITY_CHECK\n2 = INFO_REQUEST\n3 = ACTION_REQUEST\n\nInterpretation:\n- 1 CAPABILITY_CHECK: the user is asking whether Elma can do something, not asking Elma to do it now.\n- 2 INFO_REQUEST: the user wants information or an answer; a workflow may still be needed to inspect evidence.\n- 3 ACTION_REQUEST: the user wants Elma to actually do something now, including indirect polite requests.\n\nImportant distinctions:\n- \"Are you able to list files here?\" is usually 1.\n- \"What is my current project about?\" is usually 2.\n- \"Can you list files?\" and \"Could you run the tests?\" are usually 3 in normal English, because they are indirect requests.\n\nRules:\n- Output must be exactly one digit from 1 to 3.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents the user's speech act.\n".to_string(),
    }
}

fn default_action_type_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "action_type".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 16,
        timeout_s: 120,
        system_prompt: "Classify the user's request into exactly ONE WORD route.\n\nAllowed routes:\nCHAT\nSHELL\nPLAN\nMASTERPLAN\nDECIDE\n\nGuidance:\n- CHAT: greetings, smalltalk, questions that do not require terminal/workspace changes.\n- SHELL: any request to run a terminal command (list files, search, build, test, run scripts, inspect files).\n- PLAN: user asks for a step-by-step plan.\n- MASTERPLAN: user asks for an overall master plan for a multi-step objective.\n- DECIDE: user asks for a single-word decision/label.\n\nRules:\n- Output must be exactly one word from the allowed routes.\n- No punctuation.\n- No explanation.\n"
            .to_string(),
    }
}

fn default_planner_master_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "planner_master".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.6,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "auto".to_string(),
        max_tokens: 4096,
        timeout_s: 120,
        system_prompt: "You create and maintain a master execution plan.\n\nOutput Markdown only.\nUse checkboxes like:\n- [ ] step\nKeep it concise and actionable.\nDo not include any analysis."
            .to_string(),
    }
}

fn default_planner_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "planner".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.6,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "auto".to_string(),
        max_tokens: 4096,
        timeout_s: 120,
        system_prompt: "You create a detailed plan for the user's request.\n\nOutput Markdown only.\nUse a title, then a checklist of numbered actions, each as a checkbox.\nExample:\n# Plan\n- [ ] 1. Do X\n- [ ] 2. Do Y\nDo not include analysis."
            .to_string(),
    }
}

fn default_decider_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "decider".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 16,
        timeout_s: 120,
        system_prompt: "Return one word only. No punctuation. No explanation.".to_string(),
    }
}

fn default_summarizer_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "summarizer".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.3,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You summarize file contents for a terminal user.\n\nRules:\n- Output plain text only (no Markdown) unless the user explicitly asks for Markdown.\n- Be concise.\n- If the content appears truncated, say so in one short sentence.\n"
            .to_string(),
    }
}

fn default_formatter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "formatter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "Rewrite the assistant answer into plain terminal text.\n\nRules:\n- No Markdown.\n- No code fences.\n- No backticks.\n- Preserve technical accuracy.\n- If there is a function signature, show it as plain text on its own line.\n"
            .to_string(),
    }
}

fn default_calibration_judge_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "calibration_judge".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You evaluate whether Elma's final answer satisfied a calibration scenario.\n\nReturn ONLY one valid JSON object. No prose. No code fences.\n\nSchema:\n{\n  \"status\": \"pass\" | \"fail\",\n  \"reason\": \"one short sentence\",\n  \"answered_request\": true | false,\n  \"faithful_to_evidence\": true | false,\n  \"plain_text\": true | false\n}\n\nRules:\n- Pass only when the answer clearly addresses the user's final request.\n- faithful_to_evidence must be true only if the answer stays within the provided evidence or clearly marks uncertainty.\n- plain_text must be false if the answer uses Markdown and the user did not ask for Markdown.\n- Be strict.\n"
            .to_string(),
    }
}

fn default_complexity_assessor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "complexity_assessor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You assess task complexity for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"complexity\": \"DIRECT\" | \"INVESTIGATE\" | \"MULTISTEP\" | \"OPEN_ENDED\",\n  \"needs_evidence\": true | false,\n  \"needs_tools\": true | false,\n  \"needs_decision\": true | false,\n  \"needs_plan\": true | false,\n  \"risk\": \"LOW\" | \"MEDIUM\" | \"HIGH\",\n  \"suggested_pattern\": \"reply\" | \"inspect_reply\" | \"inspect_summarize_reply\" | \"inspect_decide_reply\" | \"execute_reply\" | \"plan_reply\" | \"masterplan_reply\"\n}\n\nRules:\n- Cleanup, safety review, comparison, and 'what is safe to remove' tasks are usually MULTISTEP with suggested_pattern inspect_decide_reply.\n- Questions about the current project, code, files, or configuration usually need evidence.\n- Be strict.\n"
            .to_string(),
    }
}

fn default_formula_selector_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "formula_selector".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You select reasoning formulas for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"primary\": \"one formula name\",\n  \"alternatives\": [\"...\", \"...\"],\n  \"reason\": \"one short sentence\",\n  \"memory_id\": \"optional formula memory id or empty string\"\n}\n\nPreferred built-in formulas:\n- capability_reply\n- reply_only\n- inspect_reply\n- inspect_summarize_reply\n- inspect_decide_reply\n- execute_reply\n- plan_reply\n- masterplan_reply\n- cleanup_safety_review\n- code_search_and_quote\n- config_compare\n\nRules:\n- Use the provided scope and memory candidates.\n- If a memory candidate is a strong fit, return its id in memory_id.\n- Cleanup safety questions should usually prefer cleanup_safety_review or inspect_decide_reply.\n- Code/file understanding should usually prefer code_search_and_quote, inspect_reply, or inspect_summarize_reply.\n- Direct terminal execution requests should usually prefer execute_reply.\n- Keep alternatives short and relevant.\n"
            .to_string(),
    }
}

fn default_command_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "command_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You repair one failed shell command for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\"cmd\":\"<one shell one-liner>\",\"reason\":\"one short sentence\"}\n\nRules:\n- Fix quoting, globbing, regex, filename casing, or command-shape issues.\n- Keep the same intent.\n- Prefer rg over grep.\n- Do not introduce network, remote, destructive, or privileged commands.\n- If the command cannot be safely repaired, return the original command.\n"
            .to_string(),
    }
}

fn default_scope_builder_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "scope_builder".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 384,
        timeout_s: 120,
        system_prompt: "You define the evidence scope for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"objective\": \"short string\",\n  \"focus_paths\": [\"...\"],\n  \"include_globs\": [\"...\"],\n  \"exclude_globs\": [\"...\"],\n  \"query_terms\": [\"...\"],\n  \"expected_artifacts\": [\"...\"],\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Prefer narrow scopes.\n- Exclude noisy or irrelevant areas when possible.\n- For cleanup review, focus on the repo root plus obvious generated or cluttered areas such as target, sessions, .DS_Store, temporary files, and current config artifacts. Exclude config/*/baseline, config/*/fallback, config/*/tune, and unrelated scratch directories unless the user explicitly asks about them.\n- For code lookup, focus on source and test files and relevant config files.\n- Do not include network or remote scope.\n\nExamples:\n- User asks: \"Which files in this project are safe to clean up?\"\n  Good scope: focus_paths [\".\", \"target\", \"sessions\", \"config\"], include_globs [\".gitignore\", \"Cargo.toml\"], exclude_globs [\"config/*/baseline/**\", \"config/*/fallback/**\", \"config/*/tune/**\"], query_terms [\"safe to delete\", \"generated\", \"temporary\", \"keep\"].\n- User asks: \"Find where fetch_ctx_max is defined.\"\n  Good scope: focus_paths [\"src\", \"tests\"], include_globs [\"**/*.rs\"], exclude_globs [\"target/**\"], query_terms [\"fetch_ctx_max\"].\n"
            .to_string(),
    }
}

fn default_evidence_compactor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "evidence_compactor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You compact raw workspace evidence for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"summary\": \"plain text summary\",\n  \"key_facts\": [\"...\"],\n  \"noise\": [\"...\"]\n}\n\nRules:\n- Preserve only facts that help solve the user's task.\n- Prefer exact paths, signatures, versions, and short facts.\n- Omit repetitive listings and irrelevant build artifacts.\n- Output plain text fragments only.\n"
            .to_string(),
    }
}

fn default_artifact_classifier_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "artifact_classifier".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You classify workspace artifacts for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"safe\": [\"...\"],\n  \"maybe\": [\"...\"],\n  \"keep\": [\"...\"],\n  \"ignore\": [\"...\"],\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- 'safe' means safe to delete or clean up now.\n- 'maybe' means regenerable or context-dependent; mention caution.\n- 'keep' means should normally stay.\n- 'ignore' means irrelevant to the current question.\n- Be conservative.\n"
            .to_string(),
    }
}

fn default_result_presenter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "result_presenter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.2,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You present Elma's final answer to the terminal user.\n\nRules:\n- Output plain text only unless the user explicitly asked for Markdown.\n- Be concise, professional, and direct.\n- Use the provided evidence and reply instructions.\n- If evidence is partial or failed, say so plainly.\n- Do not repeat long raw tool output.\n"
            .to_string(),
    }
}

fn default_claim_checker_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "claim_checker".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You verify that Elma's answer is supported by evidence.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"ok\" | \"revise\",\n  \"reason\": \"one short sentence\",\n  \"unsupported_claims\": [\"...\"],\n  \"missing_points\": [\"...\"],\n  \"rewrite_instructions\": \"short revision guidance\"\n}\n\nRules:\n- Choose revise if the answer contains unsupported claims, misses the main request, or overstates certainty.\n- Choose ok only when the answer is faithful to the provided evidence or clearly states uncertainty.\n- Keep rewrite_instructions short and actionable.\n"
            .to_string(),
    }
}

fn default_intention_tune_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "intention_tune".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 64,
        timeout_s: 120,
        system_prompt: "You label the user's scenario intent.\n\nGiven a scenario dialog, output EXACTLY 3 words, each on its own line.\n\nSTRICT RULES:\n- Output must be exactly 3 lines.\n- Each line must be exactly one word.\n- Each word must match: ^[A-Za-z]+$\n- No punctuation.\n- No explanation.\n"
            .to_string(),
    }
}

fn managed_profile_specs(base_url: &str, model: &str) -> Vec<(&'static str, Profile)> {
    vec![
        ("_elma.config", default_elma_config(base_url, model)),
        ("intention.toml", default_intention_config(base_url, model)),
        ("gate.toml", default_gate_config(base_url, model)),
        ("gate_why.toml", default_gate_why_config(base_url, model)),
        ("tooler.toml", default_tooler_config(base_url, model)),
        (
            "action_type.toml",
            default_action_type_config(base_url, model),
        ),
        (
            "planner_master.toml",
            default_planner_master_config(base_url, model),
        ),
        ("planner.toml", default_planner_config(base_url, model)),
        ("decider.toml", default_decider_config(base_url, model)),
        (
            "summarizer.toml",
            default_summarizer_config(base_url, model),
        ),
        ("formatter.toml", default_formatter_config(base_url, model)),
        (
            "calibration_judge.toml",
            default_calibration_judge_config(base_url, model),
        ),
        (
            "complexity_assessor.toml",
            default_complexity_assessor_config(base_url, model),
        ),
        (
            "formula_selector.toml",
            default_formula_selector_config(base_url, model),
        ),
        (
            "command_repair.toml",
            default_command_repair_config(base_url, model),
        ),
        (
            "scope_builder.toml",
            default_scope_builder_config(base_url, model),
        ),
        (
            "evidence_compactor.toml",
            default_evidence_compactor_config(base_url, model),
        ),
        (
            "artifact_classifier.toml",
            default_artifact_classifier_config(base_url, model),
        ),
        (
            "result_presenter.toml",
            default_result_presenter_config(base_url, model),
        ),
        (
            "claim_checker.toml",
            default_claim_checker_config(base_url, model),
        ),
        (
            "intention_tune.toml",
            default_intention_tune_config(base_url, model),
        ),
        ("router.toml", default_router_config(base_url, model)),
        (
            "mode_router.toml",
            default_mode_router_config(base_url, model),
        ),
        (
            "speech_act.toml",
            default_speech_act_config(base_url, model),
        ),
        (
            "orchestrator.toml",
            default_orchestrator_config(base_url, model),
        ),
        ("critic.toml", default_critic_config(base_url, model)),
    ]
}

fn managed_profile_file_names() -> Vec<&'static str> {
    managed_profile_specs("http://localhost:8080", "model")
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn new_tune_run_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    Ok(format!("run_{:010}_{}", now.as_secs(), now.subsec_nanos()))
}

fn now_unix_s() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?
        .as_secs())
}

fn model_baseline_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("baseline")
}

fn model_fallback_last_active_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("fallback").join("last_active")
}

fn model_tune_runs_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("tune").join("runs")
}

fn model_active_manifest_path(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("tune").join("active_manifest.toml")
}

fn model_formula_memory_dir(model_cfg_dir: &Path) -> PathBuf {
    model_cfg_dir.join("formula_memory")
}

fn write_profile_specs_to_dir(dir: &Path, specs: &[(&str, Profile)]) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("mkdir {}", dir.display()))?;
    for (filename, profile) in specs {
        save_agent_config(&dir.join(filename), profile)?;
    }
    Ok(())
}

fn ensure_baseline_profile_set(
    model_cfg_dir: &Path,
    base_url: &str,
    model: &str,
) -> Result<PathBuf> {
    let dir = model_baseline_dir(model_cfg_dir);
    if !dir.exists() {
        let specs = managed_profile_specs(base_url, model);
        write_profile_specs_to_dir(&dir, &specs)?;
    }
    Ok(dir)
}

fn copy_profile_set(src_dir: &Path, dst_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dst_dir).with_context(|| format!("mkdir {}", dst_dir.display()))?;
    for filename in managed_profile_file_names() {
        let src = src_dir.join(filename);
        if !src.exists() {
            continue;
        }
        let dst = dst_dir.join(filename);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

fn snapshot_active_profile_set(model_cfg_dir: &Path, snapshot_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(snapshot_dir)
        .with_context(|| format!("mkdir {}", snapshot_dir.display()))?;
    for filename in managed_profile_file_names() {
        let src = model_cfg_dir.join(filename);
        if !src.exists() {
            continue;
        }
        let dst = snapshot_dir.join(filename);
        std::fs::copy(&src, &dst)
            .with_context(|| format!("copy {} -> {}", src.display(), dst.display()))?;
    }
    Ok(())
}

fn sync_profile_dir_base_url_and_model(dir: &Path, base_url: &str, model: &str) -> Result<()> {
    for filename in managed_profile_file_names() {
        let path = dir.join(filename);
        if !path.exists() {
            continue;
        }
        let mut profile = load_agent_config(&path)?;
        profile.base_url = base_url.to_string();
        profile.model = model.to_string();
        save_agent_config(&path, &profile)?;
    }
    Ok(())
}

fn activate_profile_set(
    model_cfg_dir: &Path,
    src_dir: &Path,
    base_url: &str,
    model: &str,
    active_source: &str,
    active_run_id: Option<String>,
    final_score: f64,
    certified: bool,
) -> Result<()> {
    let fallback_dir = model_fallback_last_active_dir(model_cfg_dir);
    snapshot_active_profile_set(model_cfg_dir, &fallback_dir)?;
    copy_profile_set(src_dir, model_cfg_dir)?;
    sync_profile_dir_base_url_and_model(model_cfg_dir, base_url, model)?;
    let manifest = ActiveManifest {
        version: 1,
        model: model.to_string(),
        active_source: active_source.to_string(),
        active_run_id,
        activated_unix_s: now_unix_s()?,
        final_score,
        certified,
        restore_last_dir: fallback_dir.display().to_string(),
        restore_base_dir: model_baseline_dir(model_cfg_dir).display().to_string(),
    };
    save_active_manifest(&model_active_manifest_path(model_cfg_dir), &manifest)?;
    Ok(())
}

fn load_recent_formula_memories(
    model_cfg_dir: &Path,
    limit: usize,
) -> Result<Vec<FormulaMemoryRecord>> {
    let dir = model_formula_memory_dir(model_cfg_dir);
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    paths.sort();
    paths.reverse();
    let mut out = Vec::new();
    for path in paths.into_iter().take(limit) {
        let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
        let s = String::from_utf8(bytes).context("formula memory is not valid UTF-8")?;
        if let Ok(record) = serde_json::from_str::<FormulaMemoryRecord>(&s) {
            out.push(record);
        }
    }
    Ok(out)
}

fn save_formula_memory(model_cfg_dir: &Path, record: &FormulaMemoryRecord) -> Result<PathBuf> {
    let dir = model_formula_memory_dir(model_cfg_dir);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    let path = dir.join(format!("{}.json", record.id));
    let body = serde_json::to_string_pretty(record).context("serialize formula memory")?;
    std::fs::write(&path, body).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn ensure_model_config_folder(
    config_root: &PathBuf,
    base_url: &str,
    model_id: &str,
) -> Result<PathBuf> {
    let folder = sanitize_model_folder_name(model_id);
    let dir = config_root.join(folder);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;

    let elma_path = dir.join("_elma.config");
    if !elma_path.exists() {
        save_agent_config(&elma_path, &default_elma_config(base_url, model_id))?;
    }
    let intention_path = dir.join("intention.toml");
    if !intention_path.exists() {
        save_agent_config(
            &intention_path,
            &default_intention_config(base_url, model_id),
        )?;
    }
    let gate_path = dir.join("gate.toml");
    if !gate_path.exists() {
        save_agent_config(&gate_path, &default_gate_config(base_url, model_id))?;
    }
    let gate_why_path = dir.join("gate_why.toml");
    if !gate_why_path.exists() {
        save_agent_config(&gate_why_path, &default_gate_why_config(base_url, model_id))?;
    }
    let tooler_path = dir.join("tooler.toml");
    if !tooler_path.exists() {
        save_agent_config(&tooler_path, &default_tooler_config(base_url, model_id))?;
    }
    let planner_master_path = dir.join("planner_master.toml");
    if !planner_master_path.exists() {
        save_agent_config(
            &planner_master_path,
            &default_planner_master_config(base_url, model_id),
        )?;
    }
    let planner_path = dir.join("planner.toml");
    if !planner_path.exists() {
        save_agent_config(&planner_path, &default_planner_config(base_url, model_id))?;
    }
    let decider_path = dir.join("decider.toml");
    if !decider_path.exists() {
        save_agent_config(&decider_path, &default_decider_config(base_url, model_id))?;
    }
    let tune_path = dir.join("intention_tune.toml");
    if !tune_path.exists() {
        save_agent_config(
            &tune_path,
            &default_intention_tune_config(base_url, model_id),
        )?;
    }
    let action_type_path = dir.join("action_type.toml");
    if !action_type_path.exists() {
        save_agent_config(
            &action_type_path,
            &default_action_type_config(base_url, model_id),
        )?;
    }
    let router_path = dir.join("router.toml");
    if !router_path.exists() {
        save_agent_config(&router_path, &default_router_config(base_url, model_id))?;
    }
    let mode_router_path = dir.join("mode_router.toml");
    if !mode_router_path.exists() {
        save_agent_config(
            &mode_router_path,
            &default_mode_router_config(base_url, model_id),
        )?;
    }
    let speech_act_path = dir.join("speech_act.toml");
    if !speech_act_path.exists() {
        save_agent_config(
            &speech_act_path,
            &default_speech_act_config(base_url, model_id),
        )?;
    }
    let summarizer_path = dir.join("summarizer.toml");
    if !summarizer_path.exists() {
        save_agent_config(
            &summarizer_path,
            &default_summarizer_config(base_url, model_id),
        )?;
    }
    let formatter_path = dir.join("formatter.toml");
    if !formatter_path.exists() {
        save_agent_config(
            &formatter_path,
            &default_formatter_config(base_url, model_id),
        )?;
    }
    let calibration_judge_path = dir.join("calibration_judge.toml");
    if !calibration_judge_path.exists() {
        save_agent_config(
            &calibration_judge_path,
            &default_calibration_judge_config(base_url, model_id),
        )?;
    }
    let complexity_assessor_path = dir.join("complexity_assessor.toml");
    if !complexity_assessor_path.exists() {
        save_agent_config(
            &complexity_assessor_path,
            &default_complexity_assessor_config(base_url, model_id),
        )?;
    }
    let formula_selector_path = dir.join("formula_selector.toml");
    if !formula_selector_path.exists() {
        save_agent_config(
            &formula_selector_path,
            &default_formula_selector_config(base_url, model_id),
        )?;
    }
    let command_repair_path = dir.join("command_repair.toml");
    if !command_repair_path.exists() {
        save_agent_config(
            &command_repair_path,
            &default_command_repair_config(base_url, model_id),
        )?;
    }
    let scope_builder_path = dir.join("scope_builder.toml");
    if !scope_builder_path.exists() {
        save_agent_config(
            &scope_builder_path,
            &default_scope_builder_config(base_url, model_id),
        )?;
    }
    let evidence_compactor_path = dir.join("evidence_compactor.toml");
    if !evidence_compactor_path.exists() {
        save_agent_config(
            &evidence_compactor_path,
            &default_evidence_compactor_config(base_url, model_id),
        )?;
    }
    let artifact_classifier_path = dir.join("artifact_classifier.toml");
    if !artifact_classifier_path.exists() {
        save_agent_config(
            &artifact_classifier_path,
            &default_artifact_classifier_config(base_url, model_id),
        )?;
    }
    let result_presenter_path = dir.join("result_presenter.toml");
    if !result_presenter_path.exists() {
        save_agent_config(
            &result_presenter_path,
            &default_result_presenter_config(base_url, model_id),
        )?;
    }
    let claim_checker_path = dir.join("claim_checker.toml");
    if !claim_checker_path.exists() {
        save_agent_config(
            &claim_checker_path,
            &default_claim_checker_config(base_url, model_id),
        )?;
    }
    let router_cal_path = dir.join("router_calibration.toml");
    if !router_cal_path.exists() {
        // Placeholder; real values written by --tune.
        save_router_calibration(
            &router_cal_path,
            &RouterCalibration {
                version: 1,
                model: model_id.to_string(),
                base_url: base_url.to_string(),
                n_probs: 64,
                supports_logprobs: false,
                routes: vec![
                    "CHAT".to_string(),
                    "WORKFLOW".to_string(),
                    "INSPECT".to_string(),
                    "EXECUTE".to_string(),
                    "PLAN".to_string(),
                    "MASTERPLAN".to_string(),
                    "DECIDE".to_string(),
                    "CAPABILITY_CHECK".to_string(),
                    "INFO_REQUEST".to_string(),
                    "ACTION_REQUEST".to_string(),
                ],
            },
        )?;
    }
    let orch_path = dir.join("orchestrator.toml");
    if !orch_path.exists() {
        save_agent_config(&orch_path, &default_orchestrator_config(base_url, model_id))?;
    }
    let critic_path = dir.join("critic.toml");
    if !critic_path.exists() {
        save_agent_config(&critic_path, &default_critic_config(base_url, model_id))?;
    }
    let _ = ensure_baseline_profile_set(&dir, base_url, model_id)?;

    Ok(dir)
}

fn maybe_upgrade_system_prompt(profile: &mut Profile, expected_name: &str, patch: &str) -> bool {
    if profile.name != expected_name {
        return false;
    }
    if profile.system_prompt.contains(patch) {
        return false;
    }
    // Non-destructive upgrade: append a small block that corrects known failures
    // without overwriting user customizations.
    profile.system_prompt.push_str("\n\n");
    profile.system_prompt.push_str(patch);
    true
}

fn replace_system_prompt_if_missing(
    profile: &mut Profile,
    expected_name: &str,
    must_contain: &str,
    replacement: String,
) -> bool {
    if profile.name != expected_name {
        return false;
    }
    if profile.system_prompt.contains(must_contain) {
        return false;
    }
    profile.system_prompt = replacement;
    true
}

fn cmd_out(cmd: &str, cwd: &Path) -> String {
    let out = std::process::Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(cwd)
        .output();
    match out {
        Ok(o) => {
            let mut s = String::new();
            s.push_str(&String::from_utf8_lossy(&o.stdout));
            s.push_str(&String::from_utf8_lossy(&o.stderr));
            s.trim().to_string()
        }
        Err(_) => String::new(),
    }
}

fn gather_workspace_context(repo_root: &Path) -> String {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let term = std::env::var("TERM").unwrap_or_default();
    let user = std::env::var("USER").unwrap_or_default();
    let os_uname = cmd_out("uname -a", repo_root);
    let sw_vers = cmd_out(
        "command -v sw_vers >/dev/null 2>&1 && sw_vers || true",
        repo_root,
    );
    let whoami = cmd_out("whoami", repo_root);
    let pwd = cmd_out("pwd", repo_root);
    let tty = cmd_out("tty || true", repo_root);

    let mut s = String::new();
    s.push_str(&format!(
        "cwd: {}\n",
        if !pwd.is_empty() {
            pwd
        } else {
            repo_root.display().to_string()
        }
    ));
    if !user.is_empty() {
        s.push_str(&format!("user: {user}\n"));
    } else if !whoami.is_empty() {
        s.push_str(&format!("user: {whoami}\n"));
    }
    if !shell.is_empty() {
        s.push_str(&format!("shell: {shell}\n"));
    }
    if !term.is_empty() {
        s.push_str(&format!("term: {term}\n"));
    }
    if !tty.is_empty() {
        s.push_str(&format!("tty: {tty}\n"));
    }
    if !sw_vers.is_empty() {
        s.push_str(&format!("os: {}\n", sw_vers.replace('\n', " | ")));
    } else if !os_uname.is_empty() {
        s.push_str(&format!("os: {os_uname}\n"));
    }
    s.trim().to_string()
}

fn gather_workspace_brief(repo_root: &Path) -> String {
    let mut parts: Vec<String> = Vec::new();

    if let Ok(rd) = std::fs::read_dir(repo_root) {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| n != "target" && n != "sessions" && !n.starts_with(".git"))
            .collect();
        names.sort();
        parts.push(format!(
            "top_level: {}",
            names.into_iter().take(24).collect::<Vec<_>>().join(", ")
        ));
    }

    let cargo = repo_root.join("Cargo.toml");
    if let Ok(text) = std::fs::read_to_string(&cargo) {
        let excerpt = text.lines().take(24).collect::<Vec<_>>().join("\n");
        parts.push(format!("Cargo.toml:\n{excerpt}"));
    }

    let src_dir = repo_root.join("src");
    if let Ok(rd) = std::fs::read_dir(&src_dir) {
        let mut names: Vec<String> = rd
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        parts.push(format!("src_files: {}", names.join(", ")));
    }

    parts.join("\n\n")
}

fn extract_first_json_object(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &b) in bytes.iter().enumerate() {
        if start.is_none() {
            if b == b'{' {
                start = Some(i);
                depth = 1;
            }
            continue;
        }

        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let s = start?;
                    return text.get(s..=i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_json_loose<T: DeserializeOwned>(text: &str) -> Result<T> {
    if let Ok(v) = serde_json::from_str::<T>(text.trim()) {
        return Ok(v);
    }
    if let Some(obj) = extract_first_json_object(text) {
        return serde_json::from_str::<T>(obj.trim())
            .context("Failed to parse extracted JSON object");
    }
    anyhow::bail!("No JSON object found")
}

fn workflow_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[("1", "CHAT"), ("2", "WORKFLOW")]
}

fn mode_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("1", "INSPECT"),
        ("2", "EXECUTE"),
        ("3", "PLAN"),
        ("4", "MASTERPLAN"),
        ("5", "DECIDE"),
    ]
}

fn speech_act_code_pairs() -> &'static [(&'static str, &'static str)] {
    &[
        ("1", "CAPABILITY_CHECK"),
        ("2", "INFO_REQUEST"),
        ("3", "ACTION_REQUEST"),
    ]
}

fn route_label_from_router_output(
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

fn logsumexp(values: &[f64]) -> f64 {
    let max_v = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max_v.is_finite() {
        return f64::NEG_INFINITY;
    }
    let sum = values.iter().map(|v| (v - max_v).exp()).sum::<f64>();
    max_v + sum.ln()
}

fn parse_router_distribution(
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

fn route_margin(distribution: &[(String, f64)]) -> f64 {
    let top = distribution.first().map(|(_, p)| *p).unwrap_or(0.0);
    let second = distribution.get(1).map(|(_, p)| *p).unwrap_or(0.0);
    top - second
}

fn route_entropy(distribution: &[(String, f64)]) -> f64 {
    distribution
        .iter()
        .map(|(_, p)| if *p > 0.0 { -p * p.ln() } else { 0.0 })
        .sum()
}

fn format_route_distribution(distribution: &[(String, f64)]) -> String {
    distribution
        .iter()
        .map(|(route, p)| format!("{route}:{p:.2}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn probability_of(distribution: &[(String, f64)], label: &str) -> f64 {
    distribution
        .iter()
        .find(|(name, _)| name == label)
        .map(|(_, p)| *p)
        .unwrap_or(0.0)
}

async fn infer_digit_router(
    client: &reqwest::Client,
    chat_url: &Url,
    router_cfg: &Profile,
    router_cal: &RouterCalibration,
    prompt: String,
    pairs: &'static [(&'static str, &'static str)],
) -> Result<ProbabilityDecision> {
    let req = ChatCompletionRequest {
        model: router_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: router_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt,
            },
        ],
        temperature: router_cfg.temperature,
        top_p: router_cfg.top_p,
        stream: false,
        max_tokens: router_cfg.max_tokens,
        n_probs: Some(router_cal.n_probs.max(16)),
        repeat_penalty: Some(router_cfg.repeat_penalty),
        reasoning_format: Some(router_cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let raw = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    let fallback_choice = pairs
        .first()
        .map(|(_, label)| (*label).to_string())
        .unwrap_or_else(|| "CHAT".to_string());
    let chosen = route_label_from_router_output(&raw, pairs)
        .unwrap_or(fallback_choice.as_str())
        .to_string();

    let logprob_distribution = resp
        .choices
        .get(0)
        .and_then(|c| c.logprobs.as_ref())
        .and_then(|v| parse_router_distribution(v, pairs));
    let used_logprobs = logprob_distribution.is_some();
    let mut distribution = logprob_distribution.unwrap_or_else(|| {
        pairs
            .iter()
            .map(|(_, label)| {
                (
                    (*label).to_string(),
                    if *label == chosen { 1.0 } else { 0.0 },
                )
            })
            .collect()
    });
    distribution.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let source = if used_logprobs {
        "logprobs"
    } else {
        "token_only"
    };

    let route = distribution
        .first()
        .map(|(label, _)| label.clone())
        .unwrap_or(chosen);

    Ok(ProbabilityDecision {
        choice: route,
        source: source.to_string(),
        margin: route_margin(&distribution),
        entropy: route_entropy(&distribution),
        distribution,
    })
}

async fn infer_route_prior(
    client: &reqwest::Client,
    chat_url: &Url,
    speech_act_cfg: &Profile,
    workflow_router_cfg: &Profile,
    mode_router_cfg: &Profile,
    router_cal: &RouterCalibration,
    user_message: &str,
    workspace_facts: &str,
    workspace_brief: &str,
    recent_messages: &[ChatMessage],
) -> Result<RouteDecision> {
    let conversation = recent_messages
        .iter()
        .skip(1)
        .rev()
        .take(12)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content.replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n");

    let workflow_prompt = format!(
        "User message:\n{user_message}\n\nWorkspace facts:\n{}\n\nWorkspace brief:\n{}\n\nConversation so far (most recent last):\n{}",
        workspace_facts.trim(),
        workspace_brief.trim(),
        conversation
    );
    let workflow = infer_digit_router(
        client,
        chat_url,
        workflow_router_cfg,
        router_cal,
        workflow_prompt,
        workflow_code_pairs(),
    )
    .await?;

    let mode_prompt = format!(
        "User message:\n{user_message}\n\nWorkflow prior:\n- choice: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nWorkspace facts:\n{}\n\nWorkspace brief:\n{}\n\nConversation so far (most recent last):\n{}",
        workflow.choice,
        format_route_distribution(&workflow.distribution),
        workflow.margin,
        workflow.entropy,
        workspace_facts.trim(),
        workspace_brief.trim(),
        conversation
    );
    let mode = infer_digit_router(
        client,
        chat_url,
        mode_router_cfg,
        router_cal,
        mode_prompt,
        mode_code_pairs(),
    )
    .await?;

    let speech_prompt = format!(
        "User message:\n{user_message}\n\nConversation so far (most recent last):\n{}",
        conversation
    );
    let speech_act = infer_digit_router(
        client,
        chat_url,
        speech_act_cfg,
        router_cal,
        speech_prompt,
        speech_act_code_pairs(),
    )
    .await?;

    let chat_p = probability_of(&workflow.distribution, "CHAT");
    let workflow_p = probability_of(&workflow.distribution, "WORKFLOW");
    let shell_p = workflow_p
        * (probability_of(&mode.distribution, "INSPECT")
            + probability_of(&mode.distribution, "EXECUTE"));
    let plan_p = workflow_p * probability_of(&mode.distribution, "PLAN");
    let masterplan_p = workflow_p * probability_of(&mode.distribution, "MASTERPLAN");
    let decide_p = workflow_p * probability_of(&mode.distribution, "DECIDE");
    let mut distribution = vec![
        ("CHAT".to_string(), chat_p),
        ("SHELL".to_string(), shell_p),
        ("PLAN".to_string(), plan_p),
        ("MASTERPLAN".to_string(), masterplan_p),
        ("DECIDE".to_string(), decide_p),
    ];
    let capability_p = probability_of(&speech_act.distribution, "CAPABILITY_CHECK");
    for (label, p) in &mut distribution {
        if label == "CHAT" {
            *p = capability_p + (1.0 - capability_p) * *p;
        } else {
            *p *= 1.0 - capability_p;
        }
    }
    distribution.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let route = distribution
        .first()
        .map(|(label, _)| label.clone())
        .unwrap_or_else(|| "CHAT".to_string());

    Ok(RouteDecision {
        route,
        source: format!(
            "speech:{} workflow:{} mode:{}",
            speech_act.source, workflow.source, mode.source
        ),
        margin: route_margin(&distribution),
        entropy: route_entropy(&distribution),
        distribution,
        speech_act,
        workflow,
        mode,
    })
}

fn looks_like_path_token(s: &str) -> bool {
    let t = s.trim_matches(|c: char| c == '"' || c == '\'' || c == '`');
    if t.is_empty() {
        return false;
    }
    // Common project filenames and simple relative/absolute paths.
    if t.contains('/') || t.contains('\\') {
        return true;
    }
    let lower = t.to_ascii_lowercase();
    lower.ends_with(".toml")
        || lower.ends_with(".md")
        || lower.ends_with(".rs")
        || lower.ends_with(".txt")
        || lower.ends_with(".json")
        || lower.ends_with(".lock")
        || lower == "makefile"
        || lower == "dockerfile"
}

fn extract_first_path_from_user_text(line: &str) -> Option<String> {
    for tok in line.split_whitespace() {
        if looks_like_path_token(tok) {
            return Some(
                tok.trim_matches(|c: char| c == '"' || c == '\'' || c == '`')
                    .to_string(),
            );
        }
    }
    None
}

fn plain_terminal_text(s: &str) -> String {
    // Minimal "de-markdown" for terminal readability:
    // - remove code fences
    // - strip backticks
    // - convert leading "* " bullets to "- "
    // - drop heading markers
    let mut out = String::new();
    let mut in_fence = false;
    for raw in s.lines() {
        let line = raw.trim_end();
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        let mut l = line.to_string();
        if l.trim_start().starts_with('#') {
            l = l.trim_start_matches('#').trim_start().to_string();
        }
        if let Some(rest) = l.strip_prefix("* ") {
            l = format!("- {rest}");
        }
        l = l.replace('`', "");
        // Remove simple emphasis markers.
        l = l.replace("**", "");
        l = l.replace('*', "");
        out.push_str(l.trim_end());
        out.push('\n');
    }
    squash_blank_lines(out.trim()).trim().to_string()
}

fn shell_quote(s: &str) -> String {
    // POSIX-ish single-quote escaping: ' -> '\''.
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn normalize_shell_cmd(cmd: &str) -> String {
    let c = cmd.trim();
    // Common flaky model output: "ls -" (dangling flag).
    if c == "ls -" || c.ends_with(" ls -") || c.ends_with("\nls -") {
        return "ls -l".to_string();
    }
    if c.starts_with("ls -") && c.len() <= "ls -".len() + 2 && c.ends_with('-') {
        return "ls -l".to_string();
    }
    // Another common: "cat cargo.toml" wrong casing on macOS.
    if c.starts_with("cat cargo.toml") {
        return c.replacen("cat cargo.toml", "cat Cargo.toml", 1);
    }
    c.to_string()
}

fn summarize_shell_output(output: &str) -> String {
    const MAX_CHARS: usize = 12_000;
    let trimmed = output.trim();
    if trimmed.len() <= MAX_CHARS {
        return trimmed.to_string();
    }
    let mut s = trimmed[..MAX_CHARS].to_string();
    s.push_str("\n[truncated]");
    s
}

fn looks_like_markdown(text: &str) -> bool {
    let t = text.trim();
    t.contains("```")
        || t.contains('`')
        || t.lines().any(|l| l.trim_start().starts_with("#"))
        || t.lines().any(|l| l.trim_start().starts_with("* "))
}

fn user_requested_markdown(text: &str) -> bool {
    let t = text.to_lowercase();
    t.contains("markdown")
}

fn program_safety_check(cmd: &str) -> bool {
    is_command_sane(cmd) && is_command_allowed(cmd)
}

fn step_kind(s: &Step) -> &'static str {
    match s {
        Step::Shell { .. } => "shell",
        Step::Plan { .. } => "plan",
        Step::MasterPlan { .. } => "masterplan",
        Step::Decide { .. } => "decide",
        Step::Summarize { .. } => "summarize",
        Step::Reply { .. } => "reply",
    }
}

fn step_id(s: &Step) -> &str {
    match s {
        Step::Shell { id, .. } => id,
        Step::Plan { id, .. } => id,
        Step::MasterPlan { id, .. } => id,
        Step::Decide { id, .. } => id,
        Step::Summarize { id, .. } => id,
        Step::Reply { id, .. } => id,
    }
}

fn step_common(s: &Step) -> &StepCommon {
    match s {
        Step::Shell { common, .. } => common,
        Step::Plan { common, .. } => common,
        Step::MasterPlan { common, .. } => common,
        Step::Decide { common, .. } => common,
        Step::Summarize { common, .. } => common,
        Step::Reply { common, .. } => common,
    }
}

fn step_purpose(s: &Step) -> String {
    let common = step_common(s);
    if !common.purpose.trim().is_empty() {
        return common.purpose.trim().to_string();
    }
    match s {
        Step::Shell { .. } => "shell".to_string(),
        Step::Plan { .. } => "plan".to_string(),
        Step::MasterPlan { .. } => "masterplan".to_string(),
        Step::Decide { .. } => "decide".to_string(),
        Step::Summarize { .. } => "summarize".to_string(),
        Step::Reply { .. } => "answer".to_string(),
    }
}

fn step_success_condition(s: &Step) -> String {
    step_common(s).success_condition.trim().to_string()
}

fn step_depends_on(s: &Step) -> Vec<String> {
    step_common(s).depends_on.clone()
}

#[derive(Debug, Clone)]
struct SessionPaths {
    root: PathBuf,
    shell_dir: PathBuf,
    plans_dir: PathBuf,
    decisions_dir: PathBuf,
    tune_dir: PathBuf,
}

fn new_session_id() -> Result<String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("System time before UNIX_EPOCH")?;
    // Stable, filesystem-safe, unique-enough.
    Ok(format!("s_{:010}_{}", now.as_secs(), now.subsec_nanos()))
}

fn ensure_session_layout(sessions_root: &PathBuf) -> Result<SessionPaths> {
    std::fs::create_dir_all(sessions_root)
        .with_context(|| format!("mkdir {}", sessions_root.display()))?;

    let sid = new_session_id()?;
    let root = sessions_root.join(&sid);
    let shell_dir = root.join("shell");
    let plans_dir = root.join("plans");
    let decisions_dir = root.join("decisions");
    let tune_dir = root.join("tune");

    std::fs::create_dir_all(&shell_dir)
        .with_context(|| format!("mkdir {}", shell_dir.display()))?;
    std::fs::create_dir_all(&plans_dir)
        .with_context(|| format!("mkdir {}", plans_dir.display()))?;
    std::fs::create_dir_all(&decisions_dir)
        .with_context(|| format!("mkdir {}", decisions_dir.display()))?;
    std::fs::create_dir_all(&tune_dir).with_context(|| format!("mkdir {}", tune_dir.display()))?;

    let master = plans_dir.join("_master.md");
    if !master.exists() {
        std::fs::write(
            &master,
            "# Master Plan\n\n- [ ] (Add high-level plan items here)\n",
        )
        .with_context(|| format!("write {}", master.display()))?;
    }

    Ok(SessionPaths {
        root,
        shell_dir,
        plans_dir,
        decisions_dir,
        tune_dir,
    })
}

fn next_shell_seq(shell_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(shell_dir).with_context(|| format!("read_dir {}", shell_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        // Accept "001.sh" or "act_001.sh"
        let digits = name
            .chars()
            .filter(|c| c.is_ascii_digit())
            .collect::<String>();
        if digits.len() >= 3 {
            if let Ok(n) = digits[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

fn write_shell_action(shell_dir: &PathBuf, cmd_line: &str) -> Result<PathBuf> {
    let n = next_shell_seq(shell_dir)?;
    let path = shell_dir.join(format!("{n:03}.sh"));
    std::fs::write(&path, format!("{cmd_line}\n"))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn write_shell_output(shell_dir: &PathBuf, seq_path: &PathBuf, output: &str) -> Result<PathBuf> {
    let stem = seq_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "000".to_string());
    let path = shell_dir.join(format!("{stem}.out"));
    std::fs::write(&path, output).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn is_command_allowed(cmd: &str) -> bool {
    // For now: workspace-only, no network/remote, no destructive operations.
    // This is intentionally strict to keep "no internet" and avoid dangerous commands.
    let lower = cmd.to_lowercase();
    let tokens: Vec<String> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>'))
        .filter(|s| !s.is_empty())
        .map(|s| s.rsplit('/').next().unwrap_or(s).to_string())
        .collect();

    let banned_cmds = [
        "curl", "wget", "ssh", "scp", "rsync", "nc", "netcat", "ping", "sudo", "shutdown", "reboot",
    ];

    if tokens.iter().any(|t| banned_cmds.contains(&t.as_str())) {
        return false;
    }

    for pair in tokens.windows(2) {
        if pair[0] == "rm" && (pair[1] == "-rf" || pair[1] == "-fr") {
            return false;
        }
    }

    true
}

fn command_is_readonly(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    if lower.contains(" >")
        || lower.contains(">>")
        || lower.contains(">|")
        || lower.contains("tee ")
        || lower.contains("sed -i")
        || lower.contains("perl -pi")
    {
        return false;
    }

    let tokens: Vec<&str> = lower
        .split(|c: char| c.is_whitespace() || matches!(c, ';' | '|' | '&' | '(' | ')' | '<' | '>'))
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return false;
    }

    let first = tokens[0].rsplit('/').next().unwrap_or(tokens[0]);
    match first {
        "ls" | "pwd" | "cat" | "head" | "tail" | "rg" | "grep" | "find" | "awk" | "cut"
        | "sort" | "uniq" | "wc" | "basename" | "dirname" | "stat" | "tree" | "fd" | "jq"
        | "uname" | "whoami" | "tty" => return true,
        "sed" => return !tokens.iter().any(|t| *t == "-i"),
        "git" => {
            let sub = tokens.get(1).copied().unwrap_or("");
            return matches!(
                sub,
                "status" | "diff" | "log" | "show" | "branch" | "rev-parse"
            );
        }
        _ => {}
    }

    false
}

fn program_signature(program: &Program) -> String {
    program
        .steps
        .iter()
        .map(|step| match step {
            Step::Shell { cmd, .. } => format!("shell:{}", normalize_shell_cmd(cmd)),
            Step::Plan { .. } => "plan".to_string(),
            Step::MasterPlan { .. } => "masterplan".to_string(),
            Step::Decide { .. } => "decide".to_string(),
            Step::Summarize { .. } => "summarize".to_string(),
            Step::Reply { .. } => "reply".to_string(),
        })
        .collect::<Vec<_>>()
        .join(" | ")
}

fn evaluate_program_for_scenario(
    program: &Program,
    scenario: &CalibrationScenario,
) -> ProgramEvaluation {
    let mut ids: HashMap<String, usize> = HashMap::new();
    let mut has_reply = false;
    let mut has_shell = false;
    let mut has_plan = false;
    let mut has_masterplan = false;
    let mut has_decide = false;
    let mut shape_errors = Vec::new();
    let mut policy_errors = Vec::new();
    let mut executable_in_tune = true;

    for step in &program.steps {
        let sid = step_id(step).to_string();
        *ids.entry(sid.clone()).or_insert(0usize) += 1;
        if step_purpose(step).trim().is_empty() {
            shape_errors.push(format!("step {sid} missing purpose"));
        }
        if step_success_condition(step).trim().is_empty() {
            shape_errors.push(format!("step {sid} missing success_condition"));
        }

        match step {
            Step::Shell { cmd, .. } => {
                has_shell = true;
                let normalized = normalize_shell_cmd(cmd);
                if !program_safety_check(&normalized) {
                    policy_errors.push(format!("shell step {sid} blocked by policy"));
                }
                if scenario.mode.as_deref() == Some("INSPECT") && !command_is_readonly(&normalized)
                {
                    policy_errors.push(format!("inspect shell step {sid} is not read-only"));
                }
                if !command_is_readonly(&normalized) {
                    executable_in_tune = false;
                }
            }
            Step::Plan { .. } => has_plan = true,
            Step::MasterPlan { .. } => has_masterplan = true,
            Step::Decide { .. } => has_decide = true,
            Step::Summarize { .. } => {}
            Step::Reply { .. } => has_reply = true,
        }
    }

    for (id, count) in ids {
        if count > 1 {
            shape_errors.push(format!("duplicate step id {id}"));
        }
    }

    if program.steps.is_empty() {
        shape_errors.push("program has no steps".to_string());
    }
    if !has_reply {
        shape_errors.push("program has no reply step".to_string());
    }

    match scenario.route.as_str() {
        "CHAT" => {
            if has_shell || has_plan || has_masterplan || has_decide {
                shape_errors.push("chat route should not execute workflow steps".to_string());
            }
        }
        "SHELL" => {
            if !has_shell {
                shape_errors.push("shell route missing shell step".to_string());
            }
        }
        "PLAN" => {
            if !has_plan {
                shape_errors.push("plan route missing plan step".to_string());
            }
        }
        "MASTERPLAN" => {
            if !has_masterplan {
                shape_errors.push("masterplan route missing masterplan step".to_string());
            }
        }
        "DECIDE" => {
            if !has_decide {
                shape_errors.push("decide route missing decide step".to_string());
            }
        }
        _ => {}
    }

    if scenario.speech_act == "CAPABILITY_CHECK"
        && (has_shell || has_plan || has_masterplan || has_decide)
    {
        shape_errors.push("capability check should not execute or plan".to_string());
    }

    ProgramEvaluation {
        parsed: true,
        parse_error: String::new(),
        shape_ok: shape_errors.is_empty(),
        shape_reason: if shape_errors.is_empty() {
            "program structure matches scenario expectations".to_string()
        } else {
            shape_errors.join("; ")
        },
        policy_ok: policy_errors.is_empty(),
        policy_reason: if policy_errors.is_empty() {
            "program policy is acceptable".to_string()
        } else {
            policy_errors.join("; ")
        },
        executable_in_tune: executable_in_tune && policy_errors.is_empty(),
        signature: program_signature(program),
    }
}

fn capability_guard_threshold(route_decision: &RouteDecision) -> bool {
    route_decision
        .speech_act
        .choice
        .eq_ignore_ascii_case("CAPABILITY_CHECK")
        && probability_of(&route_decision.speech_act.distribution, "CAPABILITY_CHECK") >= 0.65
}

fn apply_capability_guard(program: &mut Program, route_decision: &RouteDecision) -> bool {
    if !capability_guard_threshold(route_decision) {
        return false;
    }
    let has_non_reply = program
        .steps
        .iter()
        .any(|s| !matches!(s, Step::Reply { .. }));
    if !has_non_reply {
        return false;
    }

    let existing_reply = program.steps.iter().find_map(|s| match s {
        Step::Reply { instructions, .. } => Some(instructions.clone()),
        _ => None,
    });
    let instructions = existing_reply.unwrap_or_else(|| {
        "Answer the user's capability question in plain text. Do not execute commands. If helpful, say what Elma can do in this workspace and that you can do it if the user asks.".to_string()
    });
    program.steps = vec![Step::Reply {
        id: "r_cap".to_string(),
        instructions,
        common: StepCommon {
            purpose: "answer capability question without executing".to_string(),
            depends_on: Vec::new(),
            success_condition:
                "the user receives a plain-text capability answer with no command execution"
                    .to_string(),
        },
    }];
    true
}

fn is_command_sane(cmd: &str) -> bool {
    // Very small sanity checks to avoid common model glitches.
    let t = cmd.trim();
    if t.is_empty() {
        return false;
    }
    if t == "ls -" || t.ends_with(" ls -") || t.contains(" ls - ") {
        return false;
    }
    true
}

fn should_classify_artifacts(
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> bool {
    formula.primary.eq_ignore_ascii_case("inspect_decide_reply")
        || complexity
            .suggested_pattern
            .eq_ignore_ascii_case("inspect_decide_reply")
}

fn preview_text(text: &str, max_lines: usize) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_shell_one_liner(cmd: &str, workdir: &PathBuf) -> Result<(i32, String)> {
    let out = Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(workdir)
        .output()
        .with_context(|| format!("Failed to run shell: {cmd}"))?;
    let code = out.status.code().unwrap_or(1);
    let mut s = String::new();
    if !out.stdout.is_empty() {
        s.push_str(&String::from_utf8_lossy(&out.stdout));
    }
    if !out.stderr.is_empty() {
        if !s.is_empty() && !s.ends_with('\n') {
            s.push('\n');
        }
        s.push_str(&String::from_utf8_lossy(&out.stderr));
    }
    Ok((code, s))
}

fn next_plan_seq(plans_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(plans_dir).with_context(|| format!("read_dir {}", plans_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        // plan_001.md
        if let Some(rest) = name.strip_prefix("plan_") {
            if rest.len() >= 7 && rest.as_bytes()[3] == b'.' {
                let digits = &rest[..3];
                if let Ok(n) = digits.parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

fn write_plan_file(plans_dir: &PathBuf, content: &str) -> Result<PathBuf> {
    let n = next_plan_seq(plans_dir)?;
    let path = plans_dir.join(format!("plan_{n:03}.md"));
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn append_master_link(plans_dir: &PathBuf, plan_path: &PathBuf, title: &str) -> Result<()> {
    let master = plans_dir.join("_master.md");
    let rel = plan_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "plan_???".to_string());
    let line = format!("- [ ] {title} ({rel})\n");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&master)
        .with_context(|| format!("open {}", master.display()))?
        .write_all(line.as_bytes())
        .with_context(|| format!("append {}", master.display()))?;
    Ok(())
}

fn next_decision_seq(decisions_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in std::fs::read_dir(decisions_dir)
        .with_context(|| format!("read_dir {}", decisions_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        // 001.txt
        if name.len() >= 7 && name.ends_with(".txt") {
            if let Ok(n) = name[..3].parse::<u32>() {
                max_n = max_n.max(n);
            }
        }
    }
    Ok(max_n + 1)
}

fn write_decision(decisions_dir: &PathBuf, word: &str) -> Result<PathBuf> {
    let n = next_decision_seq(decisions_dir)?;
    let path = decisions_dir.join(format!("{n:03}.txt"));
    std::fs::write(&path, format!("{}\n", word.trim()))
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn next_gate_why_seq(tune_dir: &PathBuf) -> Result<u32> {
    let mut max_n = 0u32;
    for ent in
        std::fs::read_dir(tune_dir).with_context(|| format!("read_dir {}", tune_dir.display()))?
    {
        let ent = ent?;
        let name = ent.file_name().to_string_lossy().to_string();
        if let Some(rest) = name.strip_prefix("gate_why_") {
            if rest.len() >= 7 && rest.ends_with(".txt") {
                if let Ok(n) = rest[..3].parse::<u32>() {
                    max_n = max_n.max(n);
                }
            }
        }
    }
    Ok(max_n + 1)
}

fn write_gate_why(tune_dir: &PathBuf, text: &str) -> Result<PathBuf> {
    let n = next_gate_why_seq(tune_dir)?;
    let path = tune_dir.join(format!("gate_why_{n:03}.txt"));
    std::fs::write(&path, text.trim().to_string() + "\n")
        .with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

async fn execute_program(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    session: &SessionPaths,
    workdir: &PathBuf,
    program: &Program,
    planner_cfg: &Profile,
    planner_master_cfg: &Profile,
    decider_cfg: &Profile,
    summarizer_cfg: &Profile,
    command_repair_cfg: Option<&Profile>,
    evidence_compactor_cfg: Option<&Profile>,
    artifact_classifier_cfg: Option<&Profile>,
    scope: &ScopePlan,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
    objective: &str,
    emit_shell_output: bool,
    readonly_only: bool,
) -> Result<(Vec<StepResult>, Option<String>)> {
    let mut step_results: Vec<StepResult> = Vec::new();
    let mut final_reply: Option<String> = None;
    let mut artifacts: HashMap<String, String> = HashMap::new();

    for step in program.steps.clone() {
        let sid = step_id(&step).to_string();
        let kind = step_kind(&step).to_string();
        let purpose = step_purpose(&step);
        let depends_on = step_depends_on(&step);
        let success_condition = step_success_condition(&step);
        trace(
            args,
            &format!(
                "step id={sid} type={kind} purpose={} depends_on={}",
                purpose,
                if depends_on.is_empty() {
                    "-".to_string()
                } else {
                    depends_on.join(",")
                }
            ),
        );
        if !matches!(step, Step::Reply { .. }) {
            operator_trace(args, &purpose);
        }

        match step {
            Step::Shell { id: _, cmd, .. } => {
                let cmd = normalize_shell_cmd(&cmd);
                if !program_safety_check(&cmd) {
                    trace(
                        args,
                        &format!("step_blocked id={sid} cmd={}", cmd.replace('\n', " ")),
                    );
                    step_results.push(StepResult {
                        id: sid,
                        kind,
                        purpose,
                        depends_on,
                        success_condition,
                        ok: false,
                        summary: "blocked_by_policy".to_string(),
                    });
                    continue;
                }
                if readonly_only && !command_is_readonly(&cmd) {
                    trace(
                        args,
                        &format!(
                            "step_skipped_readonly_only id={sid} cmd={}",
                            cmd.replace('\n', " ")
                        ),
                    );
                    step_results.push(StepResult {
                        id: sid,
                        kind,
                        purpose,
                        depends_on,
                        success_condition,
                        ok: false,
                        summary: "skipped_by_calibration_policy".to_string(),
                    });
                    continue;
                }
                let path = write_shell_action(&session.shell_dir, &cmd)?;
                trace(args, &format!("shell_saved={}", path.display()));
                let (mut code, mut output) = run_shell_one_liner(&cmd, workdir)?;
                let mut output_path_base = path.clone();
                let mut repaired_cmd: Option<String> = None;
                if code != 0 {
                    if let Some(repair_cfg) = command_repair_cfg {
                        if let Ok(repair) = repair_command_once(
                            client, chat_url, repair_cfg, objective, &purpose, &cmd, &output,
                        )
                        .await
                        {
                            let repaired = normalize_shell_cmd(repair.cmd.trim());
                            if !repaired.is_empty()
                                && repaired != cmd
                                && program_safety_check(&repaired)
                                && (!readonly_only || command_is_readonly(&repaired))
                            {
                                trace(
                                    args,
                                    &format!(
                                        "command_repair id={sid} reason={} cmd={}",
                                        repair.reason.trim(),
                                        repaired.replace('\n', " ")
                                    ),
                                );
                                operator_trace(args, "repairing a failed shell command");
                                let repair_path =
                                    write_shell_action(&session.shell_dir, &repaired)?;
                                trace(args, &format!("shell_saved={}", repair_path.display()));
                                output_path_base = repair_path;
                                let (repair_code, repair_output) =
                                    run_shell_one_liner(&repaired, workdir)?;
                                code = repair_code;
                                output = repair_output;
                                repaired_cmd = Some(repaired);
                            }
                        }
                    }
                }
                let out_path = write_shell_output(&session.shell_dir, &output_path_base, &output)?;
                trace(args, &format!("shell_output_saved={}", out_path.display()));
                trace(args, &format!("exec_exit_code={code}"));
                if emit_shell_output || code != 0 {
                    println!("elma> exit_code={code}\n{output}");
                }
                artifacts.insert(format!("{sid}:raw"), output.clone());
                let mut compact_summary = summarize_shell_output(&output);
                if let Some(compactor_cfg) = evidence_compactor_cfg {
                    if let Ok(compact) = compact_evidence_once(
                        client,
                        chat_url,
                        compactor_cfg,
                        objective,
                        &purpose,
                        scope,
                        repaired_cmd.as_deref().unwrap_or(&cmd),
                        &output,
                    )
                    .await
                    {
                        let compact_text = summarize_evidence_compact(&compact);
                        if !compact_text.trim().is_empty() {
                            compact_summary = compact_text.clone();
                            artifacts.insert(sid.clone(), compact_text);
                        }
                    }
                }
                if !artifacts.contains_key(&sid) {
                    artifacts.insert(sid.clone(), output.clone());
                }
                if let Some(classifier_cfg) = artifact_classifier_cfg {
                    if should_classify_artifacts(complexity, formula) {
                        if let Ok(classification) = classify_artifacts_once(
                            client,
                            chat_url,
                            classifier_cfg,
                            objective,
                            scope,
                            artifacts.get(&sid).map(String::as_str).unwrap_or(&output),
                        )
                        .await
                        {
                            let classification_text =
                                summarize_artifact_classification(&classification);
                            if !classification_text.trim().is_empty() {
                                artifacts.insert(
                                    format!("{sid}:classification"),
                                    classification_text.clone(),
                                );
                                compact_summary =
                                    format!("{compact_summary}\n{classification_text}");
                            }
                        }
                    }
                }
                step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: code == 0,
                    summary: if let Some(repaired) = repaired_cmd {
                        format!("repaired_cmd: {}\n{}", repaired, compact_summary)
                    } else {
                        compact_summary
                    },
                });
            }
            Step::Summarize {
                id: _,
                mut text,
                instructions,
                ..
            } => {
                if text.trim().is_empty() && !depends_on.is_empty() {
                    text = depends_on
                        .iter()
                        .filter_map(|dep| artifacts.get(dep))
                        .map(|s| s.trim())
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join("\n\n");
                }
                let sum_req = ChatCompletionRequest {
                    model: summarizer_cfg.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: summarizer_cfg.system_prompt.clone(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: format!(
                                "Instructions:\n{}\n\nText:\n{}",
                                instructions.trim(),
                                text
                            ),
                        },
                    ],
                    temperature: summarizer_cfg.temperature,
                    top_p: summarizer_cfg.top_p,
                    stream: false,
                    max_tokens: summarizer_cfg.max_tokens,
                    n_probs: None,
                    repeat_penalty: Some(summarizer_cfg.repeat_penalty),
                    reasoning_format: Some(summarizer_cfg.reasoning_format.clone()),
                };
                let sum_resp = chat_once(client, chat_url, &sum_req).await?;
                let sum_text = sum_resp
                    .choices
                    .get(0)
                    .and_then(|c| {
                        c.message
                            .content
                            .clone()
                            .or(c.message.reasoning_content.clone())
                    })
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                artifacts.insert(sid.clone(), sum_text.clone());
                step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: !sum_text.is_empty(),
                    summary: sum_text,
                });
            }
            Step::Plan { id: _, goal, .. } => {
                let req = ChatCompletionRequest {
                    model: planner_cfg.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: planner_cfg.system_prompt.clone(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: format!(
                                "Goal:\n{goal}\n\nMaster plan (_master.md):\n{}",
                                std::fs::read_to_string(session.plans_dir.join("_master.md"))
                                    .unwrap_or_default()
                            ),
                        },
                    ],
                    temperature: planner_cfg.temperature,
                    top_p: planner_cfg.top_p,
                    stream: false,
                    max_tokens: planner_cfg.max_tokens,
                    n_probs: None,
                    repeat_penalty: Some(planner_cfg.repeat_penalty),
                    reasoning_format: Some(planner_cfg.reasoning_format.clone()),
                };
                let resp = chat_once(client, chat_url, &req).await?;
                let text = resp
                    .choices
                    .get(0)
                    .and_then(|c| {
                        c.message
                            .content
                            .clone()
                            .or(c.message.reasoning_content.clone())
                    })
                    .unwrap_or_default();
                let plan_path =
                    write_plan_file(&session.plans_dir, &(text.trim().to_string() + "\n"))?;
                append_master_link(&session.plans_dir, &plan_path, &goal)?;
                trace(args, &format!("plan_saved={}", plan_path.display()));
                artifacts.insert(sid.clone(), text.trim().to_string());
                step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: true,
                    summary: format!(
                        "saved {}\n{}",
                        plan_path.display(),
                        preview_text(text.trim(), 8)
                    ),
                });
            }
            Step::MasterPlan { id: _, goal, .. } => {
                let req = ChatCompletionRequest {
                    model: planner_master_cfg.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: planner_master_cfg.system_prompt.clone(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: format!("Goal:\n{goal}\n\nUpdate the master plan."),
                        },
                    ],
                    temperature: planner_master_cfg.temperature,
                    top_p: planner_master_cfg.top_p,
                    stream: false,
                    max_tokens: planner_master_cfg.max_tokens,
                    n_probs: None,
                    repeat_penalty: Some(planner_master_cfg.repeat_penalty),
                    reasoning_format: Some(planner_master_cfg.reasoning_format.clone()),
                };
                let resp = chat_once(client, chat_url, &req).await?;
                let text = resp
                    .choices
                    .get(0)
                    .and_then(|c| {
                        c.message
                            .content
                            .clone()
                            .or(c.message.reasoning_content.clone())
                    })
                    .unwrap_or_default();
                let p = session.plans_dir.join("_master.md");
                std::fs::write(
                    &p,
                    squash_blank_lines(text.trim()).trim().to_string() + "\n",
                )
                .with_context(|| format!("write {}", p.display()))?;
                trace(args, &format!("masterplan_saved={}", p.display()));
                artifacts.insert(sid.clone(), text.trim().to_string());
                step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: true,
                    summary: format!("saved {}\n{}", p.display(), preview_text(text.trim(), 8)),
                });
            }
            Step::Decide { id: _, prompt, .. } => {
                let req = ChatCompletionRequest {
                    model: decider_cfg.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: decider_cfg.system_prompt.clone(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: prompt,
                        },
                    ],
                    temperature: decider_cfg.temperature,
                    top_p: decider_cfg.top_p,
                    stream: false,
                    max_tokens: decider_cfg.max_tokens,
                    n_probs: None,
                    repeat_penalty: Some(decider_cfg.repeat_penalty),
                    reasoning_format: Some(decider_cfg.reasoning_format.clone()),
                };
                let resp = chat_once(client, chat_url, &req).await?;
                let word = resp
                    .choices
                    .get(0)
                    .and_then(|c| {
                        c.message
                            .content
                            .clone()
                            .or(c.message.reasoning_content.clone())
                    })
                    .unwrap_or_default();
                let word = word
                    .trim()
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                let path = write_decision(&session.decisions_dir, &word)?;
                trace(args, &format!("decision_saved={}", path.display()));
                artifacts.insert(sid.clone(), word.clone());
                step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: true,
                    summary: word,
                });
            }
            Step::Reply {
                id: _,
                instructions,
                ..
            } => {
                final_reply = Some(instructions.clone());
                artifacts.insert(sid.clone(), instructions);
                step_results.push(StepResult {
                    id: sid,
                    kind,
                    purpose,
                    depends_on,
                    success_condition,
                    ok: true,
                    summary: "reply".to_string(),
                });
            }
        }
    }

    Ok((step_results, final_reply))
}

fn read_expected_line(s: &str) -> Option<String> {
    for line in s.lines() {
        let l = line.trim();
        if let Some(rest) = l.strip_prefix("expected:") {
            let t = rest.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn parse_three_tags(s: &str) -> [String; 3] {
    let mut out: Vec<String> = Vec::new();
    for line in s.lines() {
        let w = line.trim().split_whitespace().next().unwrap_or("").trim();
        if w.is_empty() {
            continue;
        }
        // keep only letters
        let cleaned: String = w.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        if cleaned.is_empty() {
            continue;
        }
        out.push(cleaned);
        if out.len() == 3 {
            break;
        }
    }
    while out.len() < 3 {
        out.push("Unknown".to_string());
    }
    [out[0].clone(), out[1].clone(), out[2].clone()]
}

fn load_intention_mapping(model_cfg_dir: &PathBuf) -> Option<Vec<(String, [String; 3])>> {
    let path = model_cfg_dir.join("intention_mapping.txt");
    let txt = std::fs::read_to_string(path).ok()?;
    let mut out = Vec::new();
    for line in txt.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        let Some((expected, tags)) = l.split_once(':') else {
            continue;
        };
        let expected = expected.trim().to_string();
        let parts: Vec<String> = tags
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 3 {
            out.push((
                expected,
                [parts[0].clone(), parts[1].clone(), parts[2].clone()],
            ));
        }
    }
    Some(out)
}

fn scenario_helper(intent_word: &str, mapping: &[(String, [String; 3])]) -> (Option<String>, f64) {
    let w = intent_word.trim();
    if w.is_empty() {
        return (None, 0.0);
    }
    let wl = w.to_lowercase();
    let mut best: Option<(String, f64)> = None;
    for (expected, tags) in mapping {
        let mut score: f64 = 0.0;
        for t in tags {
            let tl = t.to_lowercase();
            if tl == wl {
                score = score.max(0.9);
            }
            // soft match for variants (Listing vs List)
            if tl.starts_with(&wl) || wl.starts_with(&tl) {
                score = score.max(0.75);
            }
        }
        if score == 0.0 {
            // weak sentence keyword match
            if expected.to_lowercase().contains(&wl) {
                score = 0.6;
            }
        }
        if score > best.as_ref().map(|(_, s)| *s).unwrap_or(0.0) {
            best = Some((expected.clone(), score));
        }
    }
    if let Some((e, s)) = best {
        (Some(e), s)
    } else {
        (None, 0.0)
    }
}

fn list_intention_scenario_paths() -> Result<Vec<PathBuf>> {
    let dir = repo_root()?.join("scenarios").join("intention");
    let mut out: Vec<PathBuf> = std::fs::read_dir(&dir)
        .with_context(|| format!("read_dir {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("scenario_") && s.ends_with(".md"))
                .unwrap_or(false)
        })
        .collect();
    out.sort();
    Ok(out)
}

fn load_calibration_manifest() -> Result<CalibrationManifest> {
    let root = repo_root()?.join("scenarios");
    let mut scenarios = Vec::new();
    for suite in ["intention", "stress"] {
        let path = root.join(suite).join("manifest.toml");
        if !path.exists() {
            continue;
        }
        let bytes = std::fs::read(&path).with_context(|| {
            format!("Failed to read calibration manifest at {}", path.display())
        })?;
        let s = String::from_utf8(bytes).context("calibration manifest is not valid UTF-8")?;
        let mut manifest: CalibrationManifest =
            toml::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))?;
        for scenario in &mut manifest.scenarios {
            if scenario.suite.trim().is_empty() {
                scenario.suite = suite.to_string();
            }
        }
        scenarios.extend(manifest.scenarios);
    }
    Ok(CalibrationManifest {
        version: 1,
        scenarios,
    })
}

fn calibration_scenario_path(root: &Path, scenario: &CalibrationScenario) -> PathBuf {
    let suite = if scenario.suite.trim().is_empty() {
        "intention"
    } else {
        scenario.suite.as_str()
    };
    root.join("scenarios").join(suite).join(&scenario.file)
}

fn parse_scenario_dialog(s: &str) -> (String, Vec<ChatMessage>) {
    let mut messages = Vec::new();
    for line in s.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("user:") {
            messages.push(ChatMessage {
                role: "user".to_string(),
                content: rest.trim().to_string(),
            });
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("elma:") {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: rest.trim().to_string(),
            });
        }
    }
    let user_message = messages
        .iter()
        .rev()
        .find(|m| m.role == "user")
        .map(|m| m.content.clone())
        .unwrap_or_else(|| s.trim().to_string());
    (user_message, messages)
}

fn calibration_metric(correct: usize, total: usize) -> CalibrationMetric {
    CalibrationMetric {
        total,
        correct,
        accuracy: if total == 0 {
            0.0
        } else {
            correct as f64 / total as f64
        },
    }
}

fn build_confusions(pairs: &[(String, String)]) -> Vec<CalibrationConfusion> {
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for (expected, predicted) in pairs {
        *counts
            .entry((expected.clone(), predicted.clone()))
            .or_insert(0usize) += 1;
    }
    let mut out: Vec<CalibrationConfusion> = counts
        .into_iter()
        .map(|((expected, predicted), count)| CalibrationConfusion {
            expected,
            predicted,
            count,
        })
        .collect();
    out.sort_by(|a, b| {
        a.expected
            .cmp(&b.expected)
            .then(a.predicted.cmp(&b.predicted))
    });
    out
}

fn metric_accuracy_or_neutral(correct: usize, total: usize) -> f64 {
    if total == 0 {
        1.0
    } else {
        correct as f64 / total as f64
    }
}

fn is_workflow_calibration_scenario(scenario: &CalibrationScenario) -> bool {
    scenario.workflow.eq_ignore_ascii_case("WORKFLOW")
}

fn is_response_calibration_scenario(scenario: &CalibrationScenario) -> bool {
    if scenario.route.eq_ignore_ascii_case("CHAT") {
        return true;
    }
    matches!(
        scenario.mode.as_deref(),
        Some("INSPECT") | Some("PLAN") | Some("MASTERPLAN") | Some("DECIDE")
    ) || matches!(scenario.route.as_str(), "PLAN" | "MASTERPLAN" | "DECIDE")
}

fn summarize_evidence_compact(compact: &EvidenceCompact) -> String {
    let mut lines = Vec::new();
    if !compact.summary.trim().is_empty() {
        lines.push(compact.summary.trim().to_string());
    }
    for fact in compact.key_facts.iter().take(6) {
        let fact = fact.trim();
        if !fact.is_empty() {
            lines.push(format!("- {fact}"));
        }
    }
    lines.join("\n")
}

fn summarize_artifact_classification(classification: &ArtifactClassification) -> String {
    let fmt = |label: &str, items: &[String]| -> Option<String> {
        let values: Vec<String> = items
            .iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .take(6)
            .collect();
        if values.is_empty() {
            None
        } else {
            Some(format!("{label}: {}", values.join(", ")))
        }
    };
    [
        fmt("safe", &classification.safe),
        fmt("maybe", &classification.maybe),
        fmt("keep", &classification.keep),
        fmt("ignore", &classification.ignore),
        if classification.reason.trim().is_empty() {
            None
        } else {
            Some(format!("reason: {}", classification.reason.trim()))
        },
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

fn scope_contains_expected_terms(scope: &ScopePlan, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        scope.objective,
        scope.focus_paths.join("\n"),
        scope.include_globs.join("\n"),
        scope.query_terms.join("\n")
    )
    .to_lowercase();
    terms
        .iter()
        .all(|term| haystack.contains(&term.to_lowercase()))
}

fn scope_avoids_forbidden_terms(scope: &ScopePlan, terms: &[String]) -> bool {
    if terms.is_empty() {
        return true;
    }
    let haystack = format!(
        "{}\n{}\n{}\n{}",
        scope.objective,
        scope.focus_paths.join("\n"),
        scope.include_globs.join("\n"),
        scope.exclude_globs.join("\n")
    )
    .to_lowercase();
    !terms.iter().any(|term| {
        haystack.contains(&term.to_lowercase())
            && !scope
                .exclude_globs
                .iter()
                .any(|g| g.to_lowercase().contains(&term.to_lowercase()))
    })
}

fn text_contains_keywords(text: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    keywords.iter().all(|kw| lower.contains(&kw.to_lowercase()))
}

fn text_avoids_keywords(text: &str, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let lower = text.to_lowercase();
    !keywords.iter().any(|kw| lower.contains(&kw.to_lowercase()))
}

fn classification_has_categories(
    classification: &ArtifactClassification,
    categories: &[String],
) -> bool {
    if categories.is_empty() {
        return true;
    }
    let mut present = Vec::new();
    if !classification.safe.is_empty() {
        present.push("safe");
    }
    if !classification.maybe.is_empty() {
        present.push("maybe");
    }
    if !classification.keep.is_empty() {
        present.push("keep");
    }
    if !classification.ignore.is_empty() {
        present.push("ignore");
    }
    categories
        .iter()
        .all(|category| present.iter().any(|p| p.eq_ignore_ascii_case(category)))
}

fn tool_economy_score(
    step_count: usize,
    min_steps: Option<usize>,
    max_steps: Option<usize>,
) -> f64 {
    let min_steps = min_steps.unwrap_or(step_count.max(1));
    let max_steps = max_steps.unwrap_or(step_count.max(min_steps));
    if step_count < min_steps {
        (step_count as f64 / min_steps as f64).clamp(0.0, 1.0)
    } else if step_count <= max_steps {
        1.0
    } else {
        (max_steps as f64 / step_count as f64).clamp(0.0, 1.0)
    }
}

fn save_stage_score_note(dir: &Path, stage: &str, note: &str) -> Result<()> {
    let path = dir.join(format!("{stage}_score.txt"));
    std::fs::write(&path, note).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn save_calibration_report(path: &PathBuf, report: &CalibrationReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s =
        serde_json::to_string_pretty(report).context("Failed to serialize calibration report")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn load_calibration_report(path: &PathBuf) -> Result<CalibrationReport> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read calibration report at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("calibration report is not valid UTF-8")?;
    serde_json::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

fn save_efficiency_report(path: &PathBuf, report: &EfficiencyReport) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("mkdir {}", parent.display()))?;
    }
    let s =
        serde_json::to_string_pretty(report).context("Failed to serialize efficiency report")?;
    std::fs::write(path, s).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn load_efficiency_report(path: &PathBuf) -> Result<EfficiencyReport> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("Failed to read efficiency report at {}", path.display()))?;
    let s = String::from_utf8(bytes).context("efficiency report is not valid UTF-8")?;
    serde_json::from_str(&s).with_context(|| format!("Failed to parse {}", path.display()))
}

fn efficiency_metric_from_score(score_sum: f64, total: usize) -> EfficiencyMetric {
    EfficiencyMetric {
        total,
        score: if total == 0 {
            0.0
        } else {
            score_sum / total as f64
        },
    }
}

async fn assess_complexity_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ComplexityAssessment> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route_prior": {
                        "route": route_decision.route,
                        "distribution": route_decision.distribution.iter().map(|(route, p)| serde_json::json!({"route": route, "p": p})).collect::<Vec<_>>(),
                    },
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn build_scope_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    workspace_facts: &str,
    workspace_brief: &str,
    messages: &[ChatMessage],
) -> Result<ScopePlan> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "speech_act": route_decision.speech_act.choice,
                    "complexity": complexity,
                    "workspace_facts": workspace_facts,
                    "workspace_brief": workspace_brief,
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn select_formula_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    memories: &[FormulaMemoryRecord],
    messages: &[ChatMessage],
) -> Result<FormulaSelection> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "speech_act": route_decision.speech_act.choice,
                    "route": route_decision.route,
                    "complexity": complexity,
                    "scope": scope,
                    "memory_candidates": memories.iter().map(|m| {
                        serde_json::json!({
                            "id": m.id,
                            "title": m.title,
                            "route": m.route,
                            "complexity": m.complexity,
                            "formula": m.formula,
                            "objective": m.objective,
                            "example_user_message": m.user_message,
                            "program_signature": m.program_signature,
                        })
                    }).collect::<Vec<_>>(),
                    "conversation": conversation_excerpt(messages, 12),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn compact_evidence_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    scope: &ScopePlan,
    cmd: &str,
    output: &str,
) -> Result<EvidenceCompact> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "purpose": purpose,
                    "scope": scope,
                    "cmd": cmd,
                    "output": output,
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn classify_artifacts_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    scope: &ScopePlan,
    evidence: &str,
) -> Result<ArtifactClassification> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "scope": scope,
                    "evidence": evidence,
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn present_result_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<String> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "route": route_decision.route,
                    "speech_act": route_decision.speech_act.choice,
                    "instructions": reply_instructions,
                    "step_results": step_results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "type": r.kind,
                            "purpose": r.purpose,
                            "ok": r.ok,
                            "summary": r.summary,
                        })
                    }).collect::<Vec<_>>(),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default()
        .trim()
        .to_string();
    Ok(text)
}

async fn claim_check_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    step_results: &[StepResult],
    draft: &str,
) -> Result<ClaimCheckVerdict> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": user_message,
                    "draft": draft,
                    "step_results": step_results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "type": r.kind,
                            "ok": r.ok,
                            "summary": r.summary,
                        })
                    }).collect::<Vec<_>>(),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn repair_command_once(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    objective: &str,
    purpose: &str,
    failed_cmd: &str,
    output: &str,
) -> Result<CommandRepair> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "objective": objective,
                    "purpose": purpose,
                    "failed_cmd": failed_cmd,
                    "stderr_or_output": summarize_shell_output(output),
                })
                .to_string(),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

fn score_calibration_report(report: &CalibrationReport) -> f64 {
    let s = &report.summary;
    (0.10 * s.speech_act.accuracy)
        + (0.10 * s.workflow.accuracy)
        + (0.05 * s.mode.accuracy)
        + (0.10 * s.route.accuracy)
        + (0.05 * s.program_parse.accuracy)
        + (0.10 * s.program_shape.accuracy)
        + (0.10 * s.program_policy.accuracy)
        + (0.07 * s.program_consistency.accuracy)
        + (0.08 * s.execution.accuracy)
        + (0.05 * s.critic.accuracy)
        + (0.08 * s.response.accuracy)
        + (0.04 * s.scope.accuracy)
        + (0.03 * s.compaction.accuracy)
        + (0.02 * s.classification.accuracy)
        + (0.02 * s.claim_check.accuracy)
        + (0.01 * s.presentation.accuracy)
}

fn hard_rejects_calibration_report(report: &CalibrationReport) -> bool {
    report.summary.program_parse.accuracy < 0.95 || report.summary.program_policy.accuracy < 0.95
}

fn score_efficiency_report(report: &EfficiencyReport) -> f64 {
    report.summary.overall_efficiency
}

fn prompt_patch_routing() -> &'static str {
    "ADDITIONAL EXAMPLES:\n- \"Which files in this project are safe to clean up?\" should prefer WORKFLOW, not CHAT.\n- \"Can you help me decide which files to clean up?\" should not jump directly to DECIDE without evidence.\n- Safety, cleanup, inspection, and comparison questions about the workspace usually require workflow mode."
}

fn prompt_patch_mode_router() -> &'static str {
    "ADDITIONAL EXAMPLES:\n- \"Which files in this project are safe to clean up?\" is usually INSPECT first, not DECIDE.\n- If the user asks to compare workspace candidates or identify safe cleanup targets, prefer INSPECT so evidence is gathered before any decision.\n- Use DECIDE only when the task is truly label-like and does not need fresh workspace evidence."
}

fn prompt_patch_orchestrator_cleanup() -> &'static str {
    "CLEANUP AND SAFETY RULES:\n- For cleanup, safety review, or \"what is safe to remove\" requests, default to inspect_decide_reply.\n- Gather workspace evidence first: inspect directory names, build output dirs, generated artifacts, ignore rules, and obvious system clutter.\n- Do not search repo file contents for English phrases like \"safe to delete\", \"generated\", or \"temporary\". Cleanup evidence should come from filesystem structure and known artifact types, not prose matches.\n- Distinguish safe generated artifacts, maybe-safe regenerable files, and files that should normally stay.\n- Never answer cleanup safety questions from general knowledge alone when workspace evidence is available.\n- If a shell command fails with regex, glob, quoting, or parser errors, inspect stderr and retry once with a corrected command instead of proceeding as if the evidence was valid.\n- Good cleanup evidence usually includes commands like ls, find, rg on .gitignore or config, and short targeted inspection of target, sessions, config, and repo-root clutter."
}

fn prompt_patch_critic_cleanup() -> &'static str {
    "CLEANUP VALIDATION:\n- If the user asked what is safe to clean up and there is no inspected workspace evidence, choose retry.\n- If a cleanup answer classifies files without evidence or after a failed shell step, choose retry.\n- If the program used DECIDE without first inspecting relevant workspace files for a cleanup task, choose retry."
}

fn prompt_patch_elma_grounding() -> &'static str {
    "GROUNDING RULES:\n- Base answers on the provided step results.\n- If a shell step failed or evidence is incomplete, say so plainly.\n- Do not silently replace failed evidence with generic advice unless you clearly mark it as general guidance."
}

fn apply_prompt_bundle(dir: &Path, bundle: &str) -> Result<()> {
    match bundle {
        "routing_bundle" => {
            let mut router = load_agent_config(&dir.join("router.toml"))?;
            let mut mode_router = load_agent_config(&dir.join("mode_router.toml"))?;
            let mut speech_act = load_agent_config(&dir.join("speech_act.toml"))?;
            let _ = maybe_upgrade_system_prompt(&mut router, "router", prompt_patch_routing());
            let _ = maybe_upgrade_system_prompt(
                &mut mode_router,
                "mode_router",
                prompt_patch_mode_router(),
            );
            let _ = maybe_upgrade_system_prompt(&mut speech_act, "speech_act", "ADDITIONAL EXAMPLES:\n- \"Can you help me decide which files to clean up?\" is usually ACTION_REQUEST because the user is asking Elma to help now.\n- \"Which files in this project are safe to clean up?\" is usually INFO_REQUEST, but it still may require workflow inspection.");
            save_agent_config(&dir.join("router.toml"), &router)?;
            save_agent_config(&dir.join("mode_router.toml"), &mode_router)?;
            save_agent_config(&dir.join("speech_act.toml"), &speech_act)?;
        }
        "workflow_bundle" => {
            let mut orch = load_agent_config(&dir.join("orchestrator.toml"))?;
            let mut critic = load_agent_config(&dir.join("critic.toml"))?;
            let _ = maybe_upgrade_system_prompt(
                &mut orch,
                "orchestrator",
                prompt_patch_orchestrator_cleanup(),
            );
            let _ =
                maybe_upgrade_system_prompt(&mut critic, "critic", prompt_patch_critic_cleanup());
            save_agent_config(&dir.join("orchestrator.toml"), &orch)?;
            save_agent_config(&dir.join("critic.toml"), &critic)?;
        }
        "response_bundle" => {
            let mut elma = load_agent_config(&dir.join("_elma.config"))?;
            let mut critic = load_agent_config(&dir.join("critic.toml"))?;
            let _ = maybe_upgrade_system_prompt(&mut elma, "_elma", prompt_patch_elma_grounding());
            let _ = maybe_upgrade_system_prompt(&mut critic, "critic", "RESPONSE VALIDATION:\n- If the final answer ignores a failed shell step, choose retry.\n- If the answer gives generic advice where evidence was expected, choose retry unless the uncertainty is explicit.");
            save_agent_config(&dir.join("_elma.config"), &elma)?;
            save_agent_config(&dir.join("critic.toml"), &critic)?;
        }
        "comprehensive_bundle" => {
            apply_prompt_bundle(dir, "routing_bundle")?;
            apply_prompt_bundle(dir, "workflow_bundle")?;
            apply_prompt_bundle(dir, "response_bundle")?;
        }
        "none" => {}
        other => anyhow::bail!("Unknown prompt bundle: {other}"),
    }
    Ok(())
}

fn apply_router_param_variant(dir: &Path, variant: &str) -> Result<()> {
    let settings = match variant {
        "router_strict" => (0.0, 1.0, 1u32),
        "router_soft" => (0.1, 1.0, 2u32),
        other => anyhow::bail!("Unknown router variant: {other}"),
    };
    for name in ["router.toml", "mode_router.toml", "speech_act.toml"] {
        let path = dir.join(name);
        let mut profile = load_agent_config(&path)?;
        profile.temperature = settings.0;
        profile.top_p = settings.1;
        profile.max_tokens = settings.2;
        save_agent_config(&path, &profile)?;
    }
    Ok(())
}

fn apply_orchestrator_param_variant(dir: &Path, variant: &str) -> Result<()> {
    let (temperature, top_p, max_tokens) = match variant {
        "orch_conservative" => (0.0, 0.90, 1024),
        "orch_balanced" => (0.2, 0.95, 2048),
        "orch_creative" => (0.3, 1.0, 2048),
        other => anyhow::bail!("Unknown orchestrator variant: {other}"),
    };
    let path = dir.join("orchestrator.toml");
    let mut profile = load_agent_config(&path)?;
    profile.temperature = temperature;
    profile.top_p = top_p;
    profile.max_tokens = max_tokens;
    save_agent_config(&path, &profile)?;
    Ok(())
}

fn apply_response_param_variant(dir: &Path, variant: &str) -> Result<()> {
    let (elma_temp, elma_top_p, sum_temp, plan_temp, max_tokens) = match variant {
        "response_stable" => (0.3, 0.90, 0.0, 0.4, 2048),
        "response_balanced" => (0.5, 0.95, 0.2, 0.6, 4096),
        "response_creative" => (0.7, 1.0, 0.3, 0.8, 4096),
        other => anyhow::bail!("Unknown response variant: {other}"),
    };
    let mut elma = load_agent_config(&dir.join("_elma.config"))?;
    elma.temperature = elma_temp;
    elma.top_p = elma_top_p;
    elma.max_tokens = max_tokens;
    save_agent_config(&dir.join("_elma.config"), &elma)?;

    let mut summarizer = load_agent_config(&dir.join("summarizer.toml"))?;
    summarizer.temperature = sum_temp;
    save_agent_config(&dir.join("summarizer.toml"), &summarizer)?;

    for name in ["planner.toml", "planner_master.toml"] {
        let path = dir.join(name);
        let mut planner = load_agent_config(&path)?;
        planner.temperature = plan_temp;
        planner.top_p = 0.95;
        planner.max_tokens = max_tokens;
        save_agent_config(&path, &planner)?;
    }
    Ok(())
}

fn conversation_excerpt(messages: &[ChatMessage], max_items: usize) -> String {
    messages
        .iter()
        .skip(1)
        .rev()
        .take(max_items)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content.replace('\n', " ")))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_orchestrator_user_content(
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> String {
    format!(
        "User message:\n{line}\n\nSpeech-act prior:\n- chosen: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nWorkflow prior:\n- chosen: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nMode prior:\n- chosen: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nCombined route prior:\n- chosen route: {}\n- source: {}\n- distribution: {}\n- margin: {:.2}\n- entropy: {:.2}\n\nComplexity prior:\n{}\n\nScope prior:\n{}\n\nFormula prior:\n{}\n\nWorkspace facts:\n{}\n\nWorkspace brief:\n{}\n\nConversation so far (most recent last):\n{}",
        route_decision.speech_act.choice,
        route_decision.speech_act.source,
        format_route_distribution(&route_decision.speech_act.distribution),
        route_decision.speech_act.margin,
        route_decision.speech_act.entropy,
        route_decision.workflow.choice,
        route_decision.workflow.source,
        format_route_distribution(&route_decision.workflow.distribution),
        route_decision.workflow.margin,
        route_decision.workflow.entropy,
        route_decision.mode.choice,
        route_decision.mode.source,
        format_route_distribution(&route_decision.mode.distribution),
        route_decision.mode.margin,
        route_decision.mode.entropy,
        route_decision.route,
        route_decision.source,
        format_route_distribution(&route_decision.distribution),
        route_decision.margin,
        route_decision.entropy,
        serde_json::to_string_pretty(complexity).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string_pretty(scope).unwrap_or_else(|_| "{}".to_string()),
        serde_json::to_string_pretty(formula).unwrap_or_else(|_| "{}".to_string()),
        ws.trim(),
        ws_brief.trim(),
        conversation_excerpt(messages, 12)
    )
}

async fn orchestrate_program_once(
    client: &reqwest::Client,
    chat_url: &Url,
    orchestrator_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    complexity: &ComplexityAssessment,
    scope: &ScopePlan,
    formula: &FormulaSelection,
    ws: &str,
    ws_brief: &str,
    messages: &[ChatMessage],
) -> Result<(Program, String)> {
    let prompt = build_orchestrator_user_content(
        line,
        route_decision,
        complexity,
        scope,
        formula,
        ws,
        ws_brief,
        messages,
    );
    let orch_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: orchestrator_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: prompt.clone(),
            },
        ],
        temperature: orchestrator_cfg.temperature,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
    };
    let orch_resp = chat_once(client, chat_url, &orch_req).await?;
    let orch_text = orch_resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();

    if let Ok(program) = parse_json_loose(&orch_text) {
        return Ok((program, orch_text));
    }

    let repair_req = ChatCompletionRequest {
        model: orchestrator_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: orchestrator_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Your previous answer was invalid. Return ONLY a valid Program JSON object for this request.\n\n{}\n\nPrevious invalid output:\n{}",
                    prompt,
                    orch_text.trim()
                ),
            },
        ],
        temperature: orchestrator_cfg.temperature,
        top_p: orchestrator_cfg.top_p,
        stream: false,
        max_tokens: orchestrator_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(orchestrator_cfg.repeat_penalty),
        reasoning_format: Some(orchestrator_cfg.reasoning_format.clone()),
    };
    let repaired = chat_once(client, chat_url, &repair_req).await?;
    let repaired_text = repaired
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    let program = parse_json_loose(&repaired_text)?;
    Ok((program, repaired_text))
}

async fn run_critic_once(
    client: &reqwest::Client,
    chat_url: &Url,
    critic_cfg: &Profile,
    line: &str,
    route_decision: &RouteDecision,
    program: &Program,
    step_results: &[StepResult],
    attempt: u32,
) -> Result<CriticVerdict> {
    let critic_req = ChatCompletionRequest {
        model: critic_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: critic_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "user_message": line,
                    "objective": program.objective,
                    "speech_act_prior": {
                        "choice": route_decision.speech_act.choice,
                        "source": route_decision.speech_act.source,
                        "distribution": route_decision.speech_act.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.speech_act.margin,
                        "entropy": route_decision.speech_act.entropy,
                    },
                    "workflow_prior": {
                        "choice": route_decision.workflow.choice,
                        "source": route_decision.workflow.source,
                        "distribution": route_decision.workflow.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.workflow.margin,
                        "entropy": route_decision.workflow.entropy,
                    },
                    "mode_prior": {
                        "choice": route_decision.mode.choice,
                        "source": route_decision.mode.source,
                        "distribution": route_decision.mode.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.mode.margin,
                        "entropy": route_decision.mode.entropy,
                    },
                    "route_prior": {
                        "route": route_decision.route,
                        "source": route_decision.source,
                        "distribution": route_decision.distribution.iter().map(|(route, p)| {
                            serde_json::json!({"route": route, "p": p})
                        }).collect::<Vec<_>>(),
                        "margin": route_decision.margin,
                        "entropy": route_decision.entropy,
                    },
                    "attempt": attempt,
                    "program_steps": program.steps.iter().map(|s| {
                        serde_json::json!({
                            "id": step_id(s),
                            "type": step_kind(s),
                            "purpose": step_purpose(s),
                            "depends_on": step_depends_on(s),
                            "success_condition": step_success_condition(s),
                        })
                    }).collect::<Vec<_>>(),
                    "step_results": step_results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "type": r.kind,
                            "purpose": r.purpose,
                            "depends_on": r.depends_on,
                            "success_condition": r.success_condition,
                            "ok": r.ok,
                            "summary": r.summary,
                        })
                    }).collect::<Vec<_>>(),
                })
                .to_string(),
            },
        ],
        temperature: critic_cfg.temperature,
        top_p: critic_cfg.top_p,
        stream: false,
        max_tokens: critic_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(critic_cfg.repeat_penalty),
        reasoning_format: Some(critic_cfg.reasoning_format.clone()),
    };
    let verdict_resp = chat_once(client, chat_url, &critic_req).await?;
    let verdict_text = verdict_resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&verdict_text)
}

async fn generate_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    elma_cfg: &Profile,
    presenter_cfg: &Profile,
    claim_checker_cfg: &Profile,
    formatter_cfg: &Profile,
    system_content: &str,
    line: &str,
    route_decision: &RouteDecision,
    step_results: &[StepResult],
    reply_instructions: &str,
) -> Result<(String, Option<u64>)> {
    let mut usage_total: Option<u64> = None;
    let mut final_text = if route_decision.route.eq_ignore_ascii_case("CHAT") {
        let reply_req = ChatCompletionRequest {
            model: elma_cfg.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: system_content.to_string(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: serde_json::json!({
                        "user_message": line,
                        "instructions": reply_instructions,
                        "step_results": step_results.iter().map(|r| {
                            serde_json::json!({
                                "id": r.id,
                                "type": r.kind,
                                "ok": r.ok,
                                "summary": r.summary,
                            })
                        }).collect::<Vec<_>>(),
                    })
                    .to_string(),
                },
            ],
            temperature: elma_cfg.temperature,
            top_p: elma_cfg.top_p,
            stream: false,
            max_tokens: elma_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(elma_cfg.repeat_penalty),
            reasoning_format: Some(elma_cfg.reasoning_format.clone()),
        };
        let parsed = chat_once(client, chat_url, &reply_req).await?;
        usage_total = parsed.usage.as_ref().and_then(|u| u.total_tokens);
        let msg = &parsed
            .choices
            .get(0)
            .context("No choices[0] in response")?
            .message;
        msg.content.as_deref().unwrap_or("").trim().to_string()
    } else {
        present_result_once(
            client,
            chat_url,
            presenter_cfg,
            line,
            route_decision,
            step_results,
            reply_instructions,
        )
        .await
        .unwrap_or_default()
    };

    if !route_decision.route.eq_ignore_ascii_case("CHAT") && !final_text.trim().is_empty() {
        if let Ok(verdict) = claim_check_once(
            client,
            chat_url,
            claim_checker_cfg,
            line,
            step_results,
            &final_text,
        )
        .await
        {
            if verdict.status.eq_ignore_ascii_case("revise") {
                let revised = present_result_once(
                    client,
                    chat_url,
                    presenter_cfg,
                    line,
                    route_decision,
                    step_results,
                    &format!(
                        "{}\n\nRevision guidance:\n{}",
                        reply_instructions,
                        if verdict.rewrite_instructions.trim().is_empty() {
                            verdict.reason.trim()
                        } else {
                            verdict.rewrite_instructions.trim()
                        }
                    ),
                )
                .await
                .unwrap_or_default();
                if !revised.trim().is_empty() {
                    final_text = revised;
                }
            }
        }
    }
    if !user_requested_markdown(line) && looks_like_markdown(&final_text) {
        let fmt_req = ChatCompletionRequest {
            model: formatter_cfg.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: formatter_cfg.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: final_text.clone(),
                },
            ],
            temperature: formatter_cfg.temperature,
            top_p: formatter_cfg.top_p,
            stream: false,
            max_tokens: formatter_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(formatter_cfg.repeat_penalty),
            reasoning_format: Some(formatter_cfg.reasoning_format.clone()),
        };
        if let Ok(fmt_resp) = chat_once(client, chat_url, &fmt_req).await {
            usage_total = fmt_resp
                .usage
                .as_ref()
                .and_then(|u| u.total_tokens)
                .or(usage_total);
            let formatted = fmt_resp
                .choices
                .get(0)
                .and_then(|c| {
                    c.message
                        .content
                        .clone()
                        .or(c.message.reasoning_content.clone())
                })
                .unwrap_or_default();
            if !formatted.trim().is_empty() {
                final_text = formatted.trim().to_string();
            }
        }
    }
    Ok((final_text, usage_total))
}

async fn judge_final_answer_once(
    client: &reqwest::Client,
    chat_url: &Url,
    judge_cfg: &Profile,
    scenario: &CalibrationScenario,
    user_message: &str,
    step_results: &[StepResult],
    final_text: &str,
) -> Result<CalibrationJudgeVerdict> {
    let req = ChatCompletionRequest {
        model: judge_cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: judge_cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: serde_json::json!({
                    "scenario_notes": scenario.notes,
                    "expected_route": scenario.route,
                    "expected_speech_act": scenario.speech_act,
                    "user_message": user_message,
                    "step_results": step_results.iter().map(|r| {
                        serde_json::json!({
                            "id": r.id,
                            "type": r.kind,
                            "ok": r.ok,
                            "summary": r.summary,
                        })
                    }).collect::<Vec<_>>(),
                    "final_answer": final_text,
                    "markdown_requested": user_requested_markdown(user_message),
                })
                .to_string(),
            },
        ],
        temperature: judge_cfg.temperature,
        top_p: judge_cfg.top_p,
        stream: false,
        max_tokens: judge_cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(judge_cfg.repeat_penalty),
        reasoning_format: Some(judge_cfg.reasoning_format.clone()),
    };
    let resp = chat_once(client, chat_url, &req).await?;
    let text = resp
        .choices
        .get(0)
        .and_then(|c| {
            c.message
                .content
                .clone()
                .or(c.message.reasoning_content.clone())
        })
        .unwrap_or_default();
    parse_json_loose(&text)
}

async fn evaluate_routing_suite(
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    let speech_act_cfg = load_agent_config(&candidate_dir.join("speech_act.toml"))?;
    let router_cfg = load_agent_config(&candidate_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&candidate_dir.join("mode_router.toml"))?;
    let cal = load_router_calibration(&candidate_dir.join("router_calibration.toml")).unwrap_or(
        RouterCalibration {
            version: 1,
            model: model_id.to_string(),
            base_url: String::new(),
            n_probs: 64,
            supports_logprobs: false,
            routes: vec![
                "CHAT".to_string(),
                "WORKFLOW".to_string(),
                "INSPECT".to_string(),
                "EXECUTE".to_string(),
                "PLAN".to_string(),
                "MASTERPLAN".to_string(),
                "DECIDE".to_string(),
                "CAPABILITY_CHECK".to_string(),
                "INFO_REQUEST".to_string(),
                "ACTION_REQUEST".to_string(),
            ],
        },
    );
    let manifest = load_calibration_manifest()?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);

    let mut speech_correct = 0usize;
    let mut workflow_correct = 0usize;
    let mut mode_correct = 0usize;
    let mut mode_total = 0usize;
    let mut route_correct = 0usize;
    let total = manifest.scenarios.len();

    for scenario in manifest.scenarios {
        let scenario_path = repo
            .join("scenarios")
            .join("intention")
            .join(&scenario.file);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages);

        let decision = infer_route_prior(
            client,
            chat_url,
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &cal,
            &user_message,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await?;

        if decision
            .speech_act
            .choice
            .eq_ignore_ascii_case(&scenario.speech_act)
        {
            speech_correct += 1;
        }
        if decision
            .workflow
            .choice
            .eq_ignore_ascii_case(&scenario.workflow)
        {
            workflow_correct += 1;
        }
        if let Some(expected_mode) = scenario.mode.as_ref() {
            mode_total += 1;
            if decision.mode.choice.eq_ignore_ascii_case(expected_mode) {
                mode_correct += 1;
            }
        }
        if decision.route.eq_ignore_ascii_case(&scenario.route) {
            route_correct += 1;
        }
    }

    let speech_acc = metric_accuracy_or_neutral(speech_correct, total);
    let workflow_acc = metric_accuracy_or_neutral(workflow_correct, total);
    let mode_acc = metric_accuracy_or_neutral(mode_correct, mode_total);
    let route_acc = metric_accuracy_or_neutral(route_correct, total);
    let score =
        (speech_acc * 0.25) + (workflow_acc * 0.25) + (mode_acc * 0.10) + (route_acc * 0.40);
    let hard_rejected = speech_acc < 0.65 || workflow_acc < 0.70 || route_acc < 0.70;
    let note = format!(
        "routing_score={score:.4}\nspeech={speech_acc:.3}\nworkflow={workflow_acc:.3}\nmode={mode_acc:.3}\nroute={route_acc:.3}\nhard_rejected={hard_rejected}\n"
    );
    Ok((score, hard_rejected, note))
}

async fn evaluate_workflow_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    let speech_act_cfg = load_agent_config(&candidate_dir.join("speech_act.toml"))?;
    let router_cfg = load_agent_config(&candidate_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&candidate_dir.join("mode_router.toml"))?;
    let complexity_cfg = load_agent_config(&candidate_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&candidate_dir.join("formula_selector.toml"))?;
    let orchestrator_cfg = load_agent_config(&candidate_dir.join("orchestrator.toml"))?;
    let critic_cfg = load_agent_config(&candidate_dir.join("critic.toml"))?;
    let planner_master_cfg = load_agent_config(&candidate_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&candidate_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&candidate_dir.join("decider.toml"))?;
    let summarizer_cfg = load_agent_config(&candidate_dir.join("summarizer.toml"))?;
    let command_repair_cfg = load_agent_config(&candidate_dir.join("command_repair.toml"))?;
    let scope_builder_cfg = load_agent_config(&candidate_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&candidate_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&candidate_dir.join("artifact_classifier.toml"))?;
    let cal = load_router_calibration(&candidate_dir.join("router_calibration.toml")).unwrap_or(
        RouterCalibration {
            version: 1,
            model: model_id.to_string(),
            base_url: String::new(),
            n_probs: 64,
            supports_logprobs: false,
            routes: vec![],
        },
    );
    let manifest = load_calibration_manifest()?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    let tune_sessions_root = sessions_root_path(&args.sessions_root)?.join("_tune_search");

    let scenarios: Vec<CalibrationScenario> = manifest
        .scenarios
        .into_iter()
        .filter(is_workflow_calibration_scenario)
        .collect();

    let mut route_correct = 0usize;
    let mut parse_correct = 0usize;
    let mut shape_correct = 0usize;
    let mut policy_correct = 0usize;
    let mut consistency_correct = 0usize;
    let mut execution_correct = 0usize;
    let mut execution_total = 0usize;
    let mut critic_correct = 0usize;
    let mut critic_total = 0usize;

    for scenario in &scenarios {
        let scenario_path = calibration_scenario_path(&repo, scenario);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages);

        let decision = infer_route_prior(
            client,
            chat_url,
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &cal,
            &user_message,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await?;
        if decision.route.eq_ignore_ascii_case(&scenario.route) {
            route_correct += 1;
        }

        let complexity = assess_complexity_once(
            client,
            chat_url,
            &complexity_cfg,
            &user_message,
            &decision,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        .unwrap_or_default();
        let scope = build_scope_once(
            client,
            chat_url,
            &scope_builder_cfg,
            &user_message,
            &decision,
            &complexity,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        .unwrap_or_default();
        let memories = load_recent_formula_memories(candidate_dir, 8).unwrap_or_default();
        let formula = select_formula_once(
            client,
            chat_url,
            &formula_cfg,
            &user_message,
            &decision,
            &complexity,
            &scope,
            &memories,
            &conversation_messages,
        )
        .await
        .unwrap_or_default();

        let (mut program, _) = match orchestrate_program_once(
            client,
            chat_url,
            &orchestrator_cfg,
            &user_message,
            &decision,
            &complexity,
            &scope,
            &formula,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        {
            Ok(v) => v,
            Err(_) => continue,
        };

        let _ = apply_capability_guard(&mut program, &decision);
        let program_eval = evaluate_program_for_scenario(&program, scenario);
        if program_eval.parsed {
            parse_correct += 1;
        }
        if program_eval.shape_ok {
            shape_correct += 1;
        }
        if program_eval.policy_ok {
            policy_correct += 1;
        }

        if let Ok((mut second_program, _)) = orchestrate_program_once(
            client,
            chat_url,
            &orchestrator_cfg,
            &user_message,
            &decision,
            &complexity,
            &scope,
            &formula,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        {
            let _ = apply_capability_guard(&mut second_program, &decision);
            if program_signature(&program) == program_signature(&second_program) {
                consistency_correct += 1;
            }
        }

        if program_eval.parsed
            && program_eval.shape_ok
            && program_eval.policy_ok
            && program_eval.executable_in_tune
        {
            execution_total += 1;
            let session = ensure_session_layout(&tune_sessions_root)?;
            let (step_results, _) = execute_program(
                args,
                client,
                chat_url,
                &session,
                &repo,
                &program,
                &planner_cfg,
                &planner_master_cfg,
                &decider_cfg,
                &summarizer_cfg,
                Some(&command_repair_cfg),
                Some(&evidence_compactor_cfg),
                Some(&artifact_classifier_cfg),
                &scope,
                &complexity,
                &formula,
                &program.objective,
                false,
                true,
            )
            .await?;
            let step_ok = step_results.iter().all(|r| r.ok);
            if step_ok {
                execution_correct += 1;
            }

            critic_total += 1;
            if let Ok(verdict) = run_critic_once(
                client,
                chat_url,
                &critic_cfg,
                &user_message,
                &decision,
                &program,
                &step_results,
                0,
            )
            .await
            {
                let expected = if step_ok { "ok" } else { "retry" };
                if verdict.status.eq_ignore_ascii_case(expected) {
                    critic_correct += 1;
                }
            }
        }
    }

    let total = scenarios.len();
    let route_acc = metric_accuracy_or_neutral(route_correct, total);
    let parse_acc = metric_accuracy_or_neutral(parse_correct, total);
    let shape_acc = metric_accuracy_or_neutral(shape_correct, total);
    let policy_acc = metric_accuracy_or_neutral(policy_correct, total);
    let consistency_acc = metric_accuracy_or_neutral(consistency_correct, total);
    let execution_acc = metric_accuracy_or_neutral(execution_correct, execution_total);
    let critic_acc = metric_accuracy_or_neutral(critic_correct, critic_total);
    let score = (route_acc * 0.10)
        + (parse_acc * 0.20)
        + (shape_acc * 0.25)
        + (policy_acc * 0.20)
        + (consistency_acc * 0.15)
        + (execution_acc * 0.05)
        + (critic_acc * 0.05);
    let hard_rejected = parse_acc < 0.90 || policy_acc < 0.95 || shape_acc < 0.70;
    let note = format!(
        "workflow_score={score:.4}\nroute={route_acc:.3}\nparse={parse_acc:.3}\nshape={shape_acc:.3}\npolicy={policy_acc:.3}\nconsistency={consistency_acc:.3}\nexecution={execution_acc:.3}\ncritic={critic_acc:.3}\nhard_rejected={hard_rejected}\n"
    );
    Ok((score, hard_rejected, note))
}

async fn evaluate_response_suite(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    candidate_dir: &PathBuf,
    model_id: &str,
) -> Result<(f64, bool, String)> {
    let elma_cfg = load_agent_config(&candidate_dir.join("_elma.config"))?;
    let result_presenter_cfg = load_agent_config(&candidate_dir.join("result_presenter.toml"))?;
    let claim_checker_cfg = load_agent_config(&candidate_dir.join("claim_checker.toml"))?;
    let formatter_cfg = load_agent_config(&candidate_dir.join("formatter.toml"))?;
    let calibration_judge_cfg = load_agent_config(&candidate_dir.join("calibration_judge.toml"))?;
    let speech_act_cfg = load_agent_config(&candidate_dir.join("speech_act.toml"))?;
    let router_cfg = load_agent_config(&candidate_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&candidate_dir.join("mode_router.toml"))?;
    let complexity_cfg = load_agent_config(&candidate_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&candidate_dir.join("formula_selector.toml"))?;
    let orchestrator_cfg = load_agent_config(&candidate_dir.join("orchestrator.toml"))?;
    let planner_master_cfg = load_agent_config(&candidate_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&candidate_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&candidate_dir.join("decider.toml"))?;
    let summarizer_cfg = load_agent_config(&candidate_dir.join("summarizer.toml"))?;
    let command_repair_cfg = load_agent_config(&candidate_dir.join("command_repair.toml"))?;
    let scope_builder_cfg = load_agent_config(&candidate_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&candidate_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&candidate_dir.join("artifact_classifier.toml"))?;
    let cal = load_router_calibration(&candidate_dir.join("router_calibration.toml")).unwrap_or(
        RouterCalibration {
            version: 1,
            model: model_id.to_string(),
            base_url: String::new(),
            n_probs: 64,
            supports_logprobs: false,
            routes: vec![],
        },
    );
    let manifest = load_calibration_manifest()?;
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    let mut system_content = elma_cfg.system_prompt.clone();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let tune_sessions_root = sessions_root_path(&args.sessions_root)?.join("_tune_search");
    let scenarios: Vec<CalibrationScenario> = manifest
        .scenarios
        .into_iter()
        .filter(is_response_calibration_scenario)
        .collect();

    let mut response_correct = 0usize;
    let mut response_total = 0usize;
    let mut route_correct = 0usize;
    let mut route_total = 0usize;
    let mut plain_text_correct = 0usize;
    let mut plain_text_total = 0usize;

    for scenario in &scenarios {
        let scenario_path = calibration_scenario_path(&repo, scenario);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages);

        let decision = infer_route_prior(
            client,
            chat_url,
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &cal,
            &user_message,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await?;
        route_total += 1;
        if decision.route.eq_ignore_ascii_case(&scenario.route) {
            route_correct += 1;
        }

        let complexity = assess_complexity_once(
            client,
            chat_url,
            &complexity_cfg,
            &user_message,
            &decision,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        .unwrap_or_default();
        let scope = build_scope_once(
            client,
            chat_url,
            &scope_builder_cfg,
            &user_message,
            &decision,
            &complexity,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        .unwrap_or_default();
        let memories = load_recent_formula_memories(candidate_dir, 8).unwrap_or_default();
        let formula = select_formula_once(
            client,
            chat_url,
            &formula_cfg,
            &user_message,
            &decision,
            &complexity,
            &scope,
            &memories,
            &conversation_messages,
        )
        .await
        .unwrap_or_default();
        let (mut program, _) = match orchestrate_program_once(
            client,
            chat_url,
            &orchestrator_cfg,
            &user_message,
            &decision,
            &complexity,
            &scope,
            &formula,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await
        {
            Ok(v) => v,
            Err(_) => continue,
        };
        let _ = apply_capability_guard(&mut program, &decision);
        let program_eval = evaluate_program_for_scenario(&program, scenario);
        if !(program_eval.parsed
            && program_eval.shape_ok
            && program_eval.policy_ok
            && (scenario.route.eq_ignore_ascii_case("CHAT") || program_eval.executable_in_tune))
        {
            continue;
        }

        let session = ensure_session_layout(&tune_sessions_root)?;
        let (step_results, final_reply) = execute_program(
            args,
            client,
            chat_url,
            &session,
            &repo,
            &program,
            &planner_cfg,
            &planner_master_cfg,
            &decider_cfg,
            &summarizer_cfg,
            Some(&command_repair_cfg),
            Some(&evidence_compactor_cfg),
            Some(&artifact_classifier_cfg),
            &scope,
            &complexity,
            &formula,
            &program.objective,
            false,
            true,
        )
        .await?;
        let reply_instructions = final_reply.unwrap_or_else(|| {
            "Respond to the user in plain terminal text. Use any step outputs as evidence."
                .to_string()
        });
        response_total += 1;
        if let Ok((final_text, _)) = generate_final_answer_once(
            client,
            chat_url,
            &elma_cfg,
            &result_presenter_cfg,
            &claim_checker_cfg,
            &formatter_cfg,
            &system_content,
            &user_message,
            &decision,
            &step_results,
            &reply_instructions,
        )
        .await
        {
            plain_text_total += 1;
            if !looks_like_markdown(&final_text) {
                plain_text_correct += 1;
            }
            if let Ok(verdict) = judge_final_answer_once(
                client,
                chat_url,
                &calibration_judge_cfg,
                scenario,
                &user_message,
                &step_results,
                &final_text,
            )
            .await
            {
                if verdict.status.eq_ignore_ascii_case("pass")
                    && verdict.answered_request
                    && verdict.faithful_to_evidence
                    && verdict.plain_text
                {
                    response_correct += 1;
                }
            }
        }
    }

    let route_acc = metric_accuracy_or_neutral(route_correct, route_total);
    let response_acc = metric_accuracy_or_neutral(response_correct, response_total);
    let plain_text_acc = metric_accuracy_or_neutral(plain_text_correct, plain_text_total);
    let score = (route_acc * 0.15) + (response_acc * 0.70) + (plain_text_acc * 0.15);
    let hard_rejected = response_total == 0 || response_acc < 0.60 || plain_text_acc < 0.80;
    let note = format!(
        "response_score={score:.4}\nroute={route_acc:.3}\nresponse={response_acc:.3}\nplain_text={plain_text_acc:.3}\ncovered={response_total}\nhard_rejected={hard_rejected}\n"
    );
    Ok((score, hard_rejected, note))
}

async fn evaluate_candidate_dir(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    candidate_dir: &PathBuf,
    model_id: &str,
    emit_progress: bool,
) -> Result<CandidateScore> {
    sync_profile_dir_base_url_and_model(candidate_dir, base_url, model_id)?;
    let tune_cfg = load_agent_config(&candidate_dir.join("intention_tune.toml"))?;
    tune_model(
        args,
        client,
        chat_url,
        base_url,
        candidate_dir,
        model_id,
        &tune_cfg,
        emit_progress,
    )
    .await?;
    let report = load_calibration_report(&candidate_dir.join("calibration_report.json"))?;
    let efficiency_report =
        load_efficiency_report(&candidate_dir.join("efficiency_report.json")).ok();
    let efficiency_score = efficiency_report
        .as_ref()
        .map(score_efficiency_report)
        .unwrap_or(0.0);
    let score = (0.75 * score_calibration_report(&report)) + (0.25 * efficiency_score);
    let hard_rejected = hard_rejects_calibration_report(&report);
    Ok(CandidateScore {
        name: candidate_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "candidate".to_string()),
        dir: candidate_dir.clone(),
        report,
        score,
        hard_rejected,
    })
}

fn make_candidate_dir(run_root: &Path, name: &str) -> Result<PathBuf> {
    let safe = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();
    let dir = run_root.join("candidates").join(safe);
    std::fs::create_dir_all(&dir).with_context(|| format!("mkdir {}", dir.display()))?;
    Ok(dir)
}

fn select_top_beam(candidates: Vec<CandidateScore>, beam_width: usize) -> Vec<CandidateScore> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut out = Vec::new();
    for candidate in sorted {
        if candidate.hard_rejected {
            continue;
        }
        out.push(candidate);
        if out.len() >= beam_width {
            break;
        }
    }
    out
}

fn select_top_search_beam(
    candidates: Vec<SearchCandidate>,
    beam_width: usize,
) -> Vec<SearchCandidate> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.name.cmp(&b.name))
    });
    let mut out = Vec::new();
    for candidate in sorted {
        if candidate.hard_rejected {
            continue;
        }
        out.push(candidate);
        if out.len() >= beam_width {
            break;
        }
    }
    out
}

async fn optimize_model(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
) -> Result<CandidateScore> {
    let run_id = new_tune_run_id()?;
    let run_root = model_tune_runs_dir(model_cfg_dir).join(&run_id);
    std::fs::create_dir_all(run_root.join("candidates"))
        .with_context(|| format!("mkdir {}", run_root.display()))?;
    snapshot_active_profile_set(model_cfg_dir, &run_root.join("live_before"))?;
    save_tune_run_manifest(
        &run_root.join("run_manifest.toml"),
        &TuneRunManifest {
            version: 1,
            run_id: run_id.clone(),
            model: model_id.to_string(),
            mode: "tune".to_string(),
            started_unix_s: now_unix_s()?,
            activated: false,
            final_score: 0.0,
            certified: false,
        },
    )?;

    let baseline_dir = make_candidate_dir(&run_root, "00_baseline")?;
    snapshot_active_profile_set(model_cfg_dir, &baseline_dir)?;
    let (baseline_score, baseline_reject, baseline_note) =
        evaluate_routing_suite(client, chat_url, &baseline_dir, model_id).await?;
    save_stage_score_note(&baseline_dir, "stage1_routing", &baseline_note)?;
    let baseline = SearchCandidate {
        name: "00_baseline".to_string(),
        dir: baseline_dir,
        score: baseline_score,
        hard_rejected: baseline_reject,
    };
    let mut beam = vec![baseline.clone()];
    let mut best_search = baseline;
    let mut best_stage_score = best_search.score;
    let mut stagnant_rounds = 0usize;
    let beam_width = 3usize;

    let stage1_variants = [
        "none",
        "routing_bundle",
        "workflow_bundle",
        "response_bundle",
        "comprehensive_bundle",
    ];
    let mut stage1_scores = Vec::new();
    calibration_progress(
        args,
        &format!("tune stage 1/4: routing prompts for {model_id}"),
    );
    for variant in stage1_variants {
        let dir = make_candidate_dir(&run_root, &format!("10_prompt_{variant}"))?;
        copy_profile_set(&beam[0].dir, &dir)?;
        apply_prompt_bundle(&dir, variant)?;
        sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
        let (score, hard_rejected, note) =
            evaluate_routing_suite(client, chat_url, &dir, model_id).await?;
        save_stage_score_note(&dir, "stage1_routing", &note)?;
        let candidate = SearchCandidate {
            name: dir
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| variant.to_string()),
            dir,
            score,
            hard_rejected,
        };
        if candidate.score > best_search.score {
            best_search = candidate.clone();
        }
        stage1_scores.push(candidate);
    }
    beam = select_top_search_beam(stage1_scores, beam_width);
    if beam.is_empty() {
        beam.push(best_search.clone());
    }
    if let Some(top) = beam.first() {
        if top.score - best_stage_score < 0.02 {
            stagnant_rounds += 1;
        } else {
            stagnant_rounds = 0;
            best_stage_score = top.score;
        }
    }

    if stagnant_rounds < 2 {
        let router_variants = ["router_strict", "router_soft"];
        let mut stage2_scores = Vec::new();
        calibration_progress(
            args,
            &format!("tune stage 2/4: routing params for {model_id}"),
        );
        for parent in &beam {
            for variant in router_variants {
                let dir =
                    make_candidate_dir(&run_root, &format!("20_{}_{}", parent.name, variant))?;
                copy_profile_set(&parent.dir, &dir)?;
                apply_router_param_variant(&dir, variant)?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_routing_suite(client, chat_url, &dir, model_id).await?;
                save_stage_score_note(&dir, "stage2_routing", &note)?;
                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                };
                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage2_scores.push(candidate);
            }
        }
        beam = select_top_search_beam(stage2_scores, beam_width);
        if beam.is_empty() {
            beam.push(best_search.clone());
        }
        if let Some(top) = beam.first() {
            if top.score - best_stage_score < 0.02 {
                stagnant_rounds += 1;
            } else {
                stagnant_rounds = 0;
                best_stage_score = top.score;
            }
        }
    }

    if stagnant_rounds < 2 {
        let orch_variants = ["orch_conservative", "orch_balanced", "orch_creative"];
        let mut stage3_scores = Vec::new();
        calibration_progress(
            args,
            &format!("tune stage 3/4: workflow orchestration for {model_id}"),
        );
        for parent in &beam {
            for variant in orch_variants {
                let dir =
                    make_candidate_dir(&run_root, &format!("30_{}_{}", parent.name, variant))?;
                copy_profile_set(&parent.dir, &dir)?;
                apply_orchestrator_param_variant(&dir, variant)?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_workflow_suite(args, client, chat_url, &dir, model_id).await?;
                save_stage_score_note(&dir, "stage3_workflow", &note)?;
                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                };
                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage3_scores.push(candidate);
            }
        }
        beam = select_top_search_beam(stage3_scores, beam_width);
        if beam.is_empty() {
            beam.push(best_search.clone());
        }
        if let Some(top) = beam.first() {
            if top.score - best_stage_score < 0.02 {
                stagnant_rounds += 1;
            } else {
                stagnant_rounds = 0;
                best_stage_score = top.score;
            }
        }
    }

    if stagnant_rounds < 2 {
        let response_variants = ["response_stable", "response_balanced", "response_creative"];
        let mut stage4_scores = Vec::new();
        calibration_progress(
            args,
            &format!("tune stage 4/4: response quality for {model_id}"),
        );
        for parent in &beam {
            for variant in response_variants {
                let dir =
                    make_candidate_dir(&run_root, &format!("40_{}_{}", parent.name, variant))?;
                copy_profile_set(&parent.dir, &dir)?;
                apply_response_param_variant(&dir, variant)?;
                sync_profile_dir_base_url_and_model(&dir, base_url, model_id)?;
                let (score, hard_rejected, note) =
                    evaluate_response_suite(args, client, chat_url, &dir, model_id).await?;
                save_stage_score_note(&dir, "stage4_response", &note)?;
                let candidate = SearchCandidate {
                    name: dir
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| variant.to_string()),
                    dir,
                    score,
                    hard_rejected,
                };
                if candidate.score > best_search.score {
                    best_search = candidate.clone();
                }
                stage4_scores.push(candidate);
            }
        }
        let final_pool = select_top_search_beam(stage4_scores, beam_width);
        if let Some(top) = final_pool.first() {
            if top.score > best_search.score {
                best_search = top.clone();
            }
        }
    }

    let search_winner = beam.first().cloned().unwrap_or_else(|| best_search.clone());
    calibration_progress(
        args,
        &format!("tune final validation: {}", search_winner.name),
    );
    let best_overall = evaluate_candidate_dir(
        args,
        client,
        chat_url,
        base_url,
        &search_winner.dir,
        model_id,
        false,
    )
    .await?;

    let winner_dir = run_root.join("winner");
    snapshot_active_profile_set(&search_winner.dir, &winner_dir)?;
    activate_profile_set(
        model_cfg_dir,
        &search_winner.dir,
        base_url,
        model_id,
        "tune",
        Some(run_id.clone()),
        best_overall.score,
        best_overall.report.summary.certified,
    )?;
    save_tune_run_manifest(
        &run_root.join("run_manifest.toml"),
        &TuneRunManifest {
            version: 1,
            run_id,
            model: model_id.to_string(),
            mode: "tune".to_string(),
            started_unix_s: now_unix_s()?,
            activated: true,
            final_score: best_overall.score,
            certified: best_overall.report.summary.certified,
        },
    )?;
    Ok(best_overall)
}

async fn tune_model(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    base_url: &str,
    model_cfg_dir: &PathBuf,
    model_id: &str,
    intention_tune_cfg: &Profile,
    emit_progress: bool,
) -> Result<()> {
    set_trace_log_path(Some(model_cfg_dir.join("trace_debug.log")));
    if emit_progress {
        calibration_progress(args, &format!("calibrating {model_id}: router support"));
    }
    let elma_cfg = load_agent_config(&model_cfg_dir.join("_elma.config"))?;
    let router_cfg = load_agent_config(&model_cfg_dir.join("router.toml"))?;
    let mode_router_cfg = load_agent_config(&model_cfg_dir.join("mode_router.toml"))?;
    let speech_act_cfg = load_agent_config(&model_cfg_dir.join("speech_act.toml"))?;
    let planner_master_cfg = load_agent_config(&model_cfg_dir.join("planner_master.toml"))?;
    let planner_cfg = load_agent_config(&model_cfg_dir.join("planner.toml"))?;
    let decider_cfg = load_agent_config(&model_cfg_dir.join("decider.toml"))?;
    let summarizer_cfg = load_agent_config(&model_cfg_dir.join("summarizer.toml"))?;
    let formatter_cfg = load_agent_config(&model_cfg_dir.join("formatter.toml"))?;
    let complexity_cfg = load_agent_config(&model_cfg_dir.join("complexity_assessor.toml"))?;
    let formula_cfg = load_agent_config(&model_cfg_dir.join("formula_selector.toml"))?;
    let command_repair_cfg = load_agent_config(&model_cfg_dir.join("command_repair.toml"))?;
    let scope_builder_cfg = load_agent_config(&model_cfg_dir.join("scope_builder.toml"))?;
    let evidence_compactor_cfg = load_agent_config(&model_cfg_dir.join("evidence_compactor.toml"))?;
    let artifact_classifier_cfg =
        load_agent_config(&model_cfg_dir.join("artifact_classifier.toml"))?;
    let result_presenter_cfg = load_agent_config(&model_cfg_dir.join("result_presenter.toml"))?;
    let claim_checker_cfg = load_agent_config(&model_cfg_dir.join("claim_checker.toml"))?;
    let orchestrator_cfg = load_agent_config(&model_cfg_dir.join("orchestrator.toml"))?;
    let critic_cfg = load_agent_config(&model_cfg_dir.join("critic.toml"))?;
    let calibration_judge_cfg = load_agent_config(&model_cfg_dir.join("calibration_judge.toml"))?;

    // 1) Router calibration: check whether server returns logprobs for top_logprobs.
    // We can't perfectly guarantee inclusion in top_logprobs, but we can verify support and
    // choose an n_probs default that is "big enough".
    let routes = vec![
        "CHAT".to_string(),
        "WORKFLOW".to_string(),
        "INSPECT".to_string(),
        "EXECUTE".to_string(),
        "PLAN".to_string(),
        "MASTERPLAN".to_string(),
        "DECIDE".to_string(),
        "CAPABILITY_CHECK".to_string(),
        "INFO_REQUEST".to_string(),
        "ACTION_REQUEST".to_string(),
    ];
    let n_probs = 64u32;
    let cal_req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "Return exactly one digit: 1.\nNo other text.".to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: "ping".to_string(),
            },
        ],
        temperature: 0.0,
        top_p: 1.0,
        stream: false,
        max_tokens: 1,
        n_probs: Some(n_probs),
        repeat_penalty: None,
        reasoning_format: None,
    };
    let cal_resp = chat_once(client, chat_url, &cal_req).await?;
    let supports_logprobs = cal_resp
        .choices
        .get(0)
        .and_then(|c| c.logprobs.as_ref())
        .is_some();

    let cal = RouterCalibration {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        n_probs,
        supports_logprobs,
        routes,
    };
    let cal_path = model_cfg_dir.join("router_calibration.toml");
    save_router_calibration(&cal_path, &cal)?;
    trace(
        args,
        &format!("tune_router_calibration_saved={}", cal_path.display()),
    );

    // 2) Build intention_mapping.txt from scenario files.
    let scenario_paths = list_intention_scenario_paths()?;
    let mut lines: Vec<String> = Vec::new();
    let scenario_count = scenario_paths.len();
    for (index, p) in scenario_paths.into_iter().enumerate() {
        let txt = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
        let Some(expected) = read_expected_line(&txt) else {
            continue;
        };
        if emit_progress {
            calibration_progress(
                args,
                &format!(
                    "calibrating {model_id}: intention tags {}/{}",
                    index + 1,
                    scenario_count
                ),
            );
        }

        let req = ChatCompletionRequest {
            model: intention_tune_cfg.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: intention_tune_cfg.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: txt,
                },
            ],
            temperature: intention_tune_cfg.temperature,
            top_p: intention_tune_cfg.top_p,
            stream: false,
            max_tokens: intention_tune_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(intention_tune_cfg.repeat_penalty),
            reasoning_format: Some(intention_tune_cfg.reasoning_format.clone()),
        };

        let resp = chat_once(client, chat_url, &req).await?;
        let raw = resp
            .choices
            .get(0)
            .and_then(|c| {
                c.message
                    .content
                    .clone()
                    .or(c.message.reasoning_content.clone())
            })
            .unwrap_or_default();
        let tags = parse_three_tags(&raw);
        lines.push(format!(
            "{}: {}, {}, {}",
            expected, tags[0], tags[1], tags[2]
        ));
    }
    let mapping_path = model_cfg_dir.join("intention_mapping.txt");
    std::fs::write(&mapping_path, lines.join("\n") + "\n")
        .with_context(|| format!("write {}", mapping_path.display()))?;
    trace(
        args,
        &format!("tune_intention_mapping_saved={}", mapping_path.display()),
    );

    // 3) Golden-corpus calibration for runtime probabilistic control.
    let manifest = load_calibration_manifest()?;
    if manifest.version != 1 {
        anyhow::bail!(
            "Unsupported calibration manifest version {}",
            manifest.version
        );
    }
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    let mut system_content = elma_cfg.system_prompt.clone();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let tune_sessions_root = sessions_root_path(&args.sessions_root)?.join("_tune");
    let mut speech_pairs = Vec::new();
    let mut workflow_pairs = Vec::new();
    let mut mode_pairs = Vec::new();
    let mut route_pairs = Vec::new();
    let mut scenario_results = Vec::new();
    let mut speech_correct = 0usize;
    let mut workflow_correct = 0usize;
    let mut mode_correct = 0usize;
    let mut mode_total = 0usize;
    let mut route_correct = 0usize;
    let mut program_parse_correct = 0usize;
    let mut program_shape_correct = 0usize;
    let mut program_policy_correct = 0usize;
    let mut program_consistency_correct = 0usize;
    let mut execution_correct = 0usize;
    let mut execution_total = 0usize;
    let mut critic_correct = 0usize;
    let mut critic_total = 0usize;
    let mut response_correct = 0usize;
    let mut response_total = 0usize;
    let mut scope_correct = 0usize;
    let mut scope_total = 0usize;
    let mut compaction_correct = 0usize;
    let mut compaction_total = 0usize;
    let mut classification_correct = 0usize;
    let mut classification_total = 0usize;
    let mut claim_check_correct = 0usize;
    let mut claim_check_total = 0usize;
    let mut presentation_correct = 0usize;
    let mut presentation_total = 0usize;
    let mut all_ok_correct = 0usize;
    let mut efficiency_scenarios = Vec::new();

    let scenario_total = manifest.scenarios.len();
    for (scenario_index, scenario) in manifest.scenarios.into_iter().enumerate() {
        if emit_progress {
            calibration_progress(
                args,
                &format!(
                    "calibrating {model_id}: runtime suite {}/{} ({})",
                    scenario_index + 1,
                    scenario_total,
                    scenario.file
                ),
            );
        }
        let scenario_path = calibration_scenario_path(&repo, &scenario);
        let txt = std::fs::read_to_string(&scenario_path)
            .with_context(|| format!("read {}", scenario_path.display()))?;
        let (user_message, recent_messages) = parse_scenario_dialog(&txt);
        let mut conversation_messages = vec![ChatMessage {
            role: "system".to_string(),
            content: String::new(),
        }];
        conversation_messages.extend(recent_messages.clone());
        let decision = infer_route_prior(
            client,
            chat_url,
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &cal,
            &user_message,
            &ws,
            &ws_brief,
            &conversation_messages,
        )
        .await?;

        let speech_ok = decision
            .speech_act
            .choice
            .eq_ignore_ascii_case(&scenario.speech_act);
        let workflow_ok = decision
            .workflow
            .choice
            .eq_ignore_ascii_case(&scenario.workflow);
        let mode_ok = scenario
            .mode
            .as_ref()
            .map(|m| decision.mode.choice.eq_ignore_ascii_case(m));
        let route_ok = decision.route.eq_ignore_ascii_case(&scenario.route);
        let all_ok = speech_ok && workflow_ok && mode_ok.unwrap_or(true) && route_ok;

        if speech_ok {
            speech_correct += 1;
        }
        if workflow_ok {
            workflow_correct += 1;
        }
        if let Some(ok) = mode_ok {
            mode_total += 1;
            if ok {
                mode_correct += 1;
            }
        }
        if route_ok {
            route_correct += 1;
        }

        speech_pairs.push((
            scenario.speech_act.clone(),
            decision.speech_act.choice.clone(),
        ));
        workflow_pairs.push((scenario.workflow.clone(), decision.workflow.choice.clone()));
        if let Some(expected_mode) = scenario.mode.clone() {
            mode_pairs.push((expected_mode, decision.mode.choice.clone()));
        }
        route_pairs.push((scenario.route.clone(), decision.route.clone()));

        let (
            program_signature,
            actual_steps,
            program_parse_ok,
            program_parse_error,
            program_shape_ok,
            program_shape_reason,
            program_policy_ok,
            program_policy_reason,
            program_consistency_ok,
            executed_in_tune,
            execution_ok,
            critic_ok,
            critic_reason,
            response_ok,
            response_reason,
            response_plain_text,
            scope_ok,
            scope_reason,
            compaction_ok,
            compaction_reason,
            classification_ok,
            classification_reason,
            claim_check_ok,
            claim_check_reason,
            presentation_ok,
            presentation_reason,
            tool_economy,
            all_ok,
        ) = {
            let complexity = assess_complexity_once(
                client,
                chat_url,
                &complexity_cfg,
                &user_message,
                &decision,
                &ws,
                &ws_brief,
                &conversation_messages,
            )
            .await
            .unwrap_or_default();
            let scope = build_scope_once(
                client,
                chat_url,
                &scope_builder_cfg,
                &user_message,
                &decision,
                &complexity,
                &ws,
                &ws_brief,
                &conversation_messages,
            )
            .await
            .unwrap_or_default();
            let expected_scope = !scenario.expected_scope_terms.is_empty()
                || !scenario.forbidden_scope_terms.is_empty();
            let scope_eval_ok =
                scope_contains_expected_terms(&scope, &scenario.expected_scope_terms)
                    && scope_avoids_forbidden_terms(&scope, &scenario.forbidden_scope_terms);
            if expected_scope {
                scope_total += 1;
                if scope_eval_ok {
                    scope_correct += 1;
                }
            }
            let scope_eval_reason = if scope_eval_ok {
                "scope matches scenario expectations".to_string()
            } else {
                format!(
                        "scope mismatch: expected {:?}, forbidden {:?}, got focus_paths={:?} exclude={:?}",
                        scenario.expected_scope_terms,
                        scenario.forbidden_scope_terms,
                        scope.focus_paths,
                        scope.exclude_globs
                    )
            };
            let memories = load_recent_formula_memories(model_cfg_dir, 8).unwrap_or_default();
            let formula = select_formula_once(
                client,
                chat_url,
                &formula_cfg,
                &user_message,
                &decision,
                &complexity,
                &scope,
                &memories,
                &conversation_messages,
            )
            .await
            .unwrap_or_default();
            let mut program_opt: Option<Program> = None;
            let mut program_eval = ProgramEvaluation {
                parsed: false,
                parse_error: String::new(),
                shape_ok: false,
                shape_reason: "program not produced".to_string(),
                policy_ok: false,
                policy_reason: "program not produced".to_string(),
                executable_in_tune: false,
                signature: String::new(),
            };

            match orchestrate_program_once(
                client,
                chat_url,
                &orchestrator_cfg,
                &user_message,
                &decision,
                &complexity,
                &scope,
                &formula,
                &ws,
                &ws_brief,
                &conversation_messages,
            )
            .await
            {
                Ok((mut program, _raw)) => {
                    if apply_capability_guard(&mut program, &decision) {
                        trace(
                            args,
                            &format!("tune_guard=capability_reply_only file={}", scenario.file),
                        );
                    }
                    program_eval = evaluate_program_for_scenario(&program, &scenario);
                    program_opt = Some(program);
                }
                Err(e) => {
                    program_eval.parse_error = e.to_string();
                    program_eval.shape_reason = "program parse failed".to_string();
                    program_eval.policy_reason = "program parse failed".to_string();
                }
            }

            if program_eval.parsed {
                program_parse_correct += 1;
            }
            if program_eval.shape_ok {
                program_shape_correct += 1;
            }
            if program_eval.policy_ok {
                program_policy_correct += 1;
            }

            let mut consistency_ok = false;
            if let Some(ref program) = program_opt {
                if let Ok((mut second_program, _)) = orchestrate_program_once(
                    client,
                    chat_url,
                    &orchestrator_cfg,
                    &user_message,
                    &decision,
                    &complexity,
                    &scope,
                    &formula,
                    &ws,
                    &ws_brief,
                    &conversation_messages,
                )
                .await
                {
                    let _ = apply_capability_guard(&mut second_program, &decision);
                    consistency_ok =
                        program_signature(program) == program_signature(&second_program);
                }
            }
            if consistency_ok {
                program_consistency_correct += 1;
            }

            let mut executed_in_tune = false;
            let mut execution_ok = None;
            let mut critic_ok = None;
            let mut critic_reason = None;
            let mut response_ok = None;
            let mut response_reason = None;
            let mut response_plain_text = None;
            let mut compaction_ok = None;
            let mut compaction_reason = None;
            let mut classification_ok = None;
            let mut classification_reason = None;
            let mut claim_check_ok = None;
            let mut claim_check_reason = None;
            let mut presentation_ok = None;
            let mut presentation_reason = None;
            let tool_economy = tool_economy_score(
                program_opt
                    .as_ref()
                    .map(|p| p.steps.len())
                    .unwrap_or_default(),
                scenario.minimum_step_count,
                scenario.maximum_step_count,
            );

            if let Some(program) = program_opt.clone() {
                if program_eval.parsed
                    && program_eval.shape_ok
                    && program_eval.policy_ok
                    && program_eval.executable_in_tune
                {
                    executed_in_tune = true;
                    execution_total += 1;
                    let session = ensure_session_layout(&tune_sessions_root)?;
                    let (step_results, final_reply) = execute_program(
                        args,
                        client,
                        chat_url,
                        &session,
                        &repo,
                        &program,
                        &planner_cfg,
                        &planner_master_cfg,
                        &decider_cfg,
                        &summarizer_cfg,
                        Some(&command_repair_cfg),
                        Some(&evidence_compactor_cfg),
                        Some(&artifact_classifier_cfg),
                        &scope,
                        &complexity,
                        &formula,
                        &program.objective,
                        false,
                        true,
                    )
                    .await?;
                    let step_exec_ok = step_results.iter().all(|r| r.ok);
                    execution_ok = Some(step_exec_ok);
                    if step_exec_ok {
                        execution_correct += 1;
                    }

                    let shell_summaries = step_results
                        .iter()
                        .filter(|r| r.kind == "shell")
                        .map(|r| r.summary.clone())
                        .collect::<Vec<_>>();
                    if !shell_summaries.is_empty() {
                        compaction_total += 1;
                        let compact_good = shell_summaries
                            .iter()
                            .all(|s| !s.trim().is_empty() && s.lines().count() <= 24);
                        if compact_good {
                            compaction_correct += 1;
                        }
                        compaction_ok = Some(compact_good);
                        compaction_reason = Some(if compact_good {
                            "shell evidence was compacted to a focused summary".to_string()
                        } else {
                            "shell evidence remained too noisy or empty".to_string()
                        });
                    }
                    if !scenario.expected_categories.is_empty() {
                        classification_total += 1;
                        let classification_text = step_results
                            .iter()
                            .map(|r| r.summary.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        let classification_good = text_contains_keywords(
                            &classification_text,
                            &scenario.expected_categories,
                        );
                        if classification_good {
                            classification_correct += 1;
                        }
                        classification_ok = Some(classification_good);
                        classification_reason = Some(if classification_good {
                            "artifact categories were present in the evidence summary".to_string()
                        } else {
                            format!(
                                "missing expected categories {:?}",
                                scenario.expected_categories
                            )
                        });
                    }

                    let expected_critic_ok = step_exec_ok;
                    critic_total += 1;
                    match run_critic_once(
                        client,
                        chat_url,
                        &critic_cfg,
                        &user_message,
                        &decision,
                        &program,
                        &step_results,
                        0,
                    )
                    .await
                    {
                        Ok(verdict) => {
                            let ok = verdict.status.eq_ignore_ascii_case(if expected_critic_ok {
                                "ok"
                            } else {
                                "retry"
                            });
                            if ok {
                                critic_correct += 1;
                            }
                            critic_reason = Some(verdict.reason.clone());
                            critic_ok = Some(ok);
                        }
                        Err(e) => {
                            critic_reason = Some(format!("critic error: {e}"));
                            critic_ok = Some(false);
                        }
                    }

                    let reply_instructions = final_reply.clone().unwrap_or_else(|| {
                            "Respond to the user in plain terminal text. Use any step outputs as evidence."
                                .to_string()
                        });
                    response_total += 1;
                    match generate_final_answer_once(
                        client,
                        chat_url,
                        &elma_cfg,
                        &result_presenter_cfg,
                        &claim_checker_cfg,
                        &formatter_cfg,
                        &system_content,
                        &user_message,
                        &decision,
                        &step_results,
                        &reply_instructions,
                    )
                    .await
                    {
                        Ok((final_text, _)) => {
                            claim_check_total += 1;
                            match claim_check_once(
                                client,
                                chat_url,
                                &claim_checker_cfg,
                                &user_message,
                                &step_results,
                                &final_text,
                            )
                            .await
                            {
                                Ok(verdict) => {
                                    let ok = verdict.status.eq_ignore_ascii_case("ok");
                                    if ok {
                                        claim_check_correct += 1;
                                    }
                                    claim_check_ok = Some(ok);
                                    claim_check_reason = Some(verdict.reason);
                                }
                                Err(e) => {
                                    claim_check_ok = Some(false);
                                    claim_check_reason = Some(format!("claim checker error: {e}"));
                                }
                            }
                            match judge_final_answer_once(
                                client,
                                chat_url,
                                &calibration_judge_cfg,
                                &scenario,
                                &user_message,
                                &step_results,
                                &final_text,
                            )
                            .await
                            {
                                Ok(verdict) => {
                                    let keyword_ok = text_contains_keywords(
                                        &final_text,
                                        &scenario.expected_answer_keywords,
                                    ) && text_avoids_keywords(
                                        &final_text,
                                        &scenario.avoid_answer_keywords,
                                    );
                                    let ok = verdict.status.eq_ignore_ascii_case("pass")
                                        && verdict.answered_request
                                        && verdict.faithful_to_evidence
                                        && verdict.plain_text
                                        && keyword_ok;
                                    if ok {
                                        response_correct += 1;
                                    }
                                    response_plain_text = Some(verdict.plain_text);
                                    response_reason = Some(if keyword_ok {
                                        verdict.reason
                                    } else {
                                        "answer keywords did not match scenario expectations"
                                            .to_string()
                                    });
                                    response_ok = Some(ok);
                                    presentation_total += 1;
                                    let present_ok = verdict.plain_text && keyword_ok;
                                    if present_ok {
                                        presentation_correct += 1;
                                    }
                                    presentation_ok = Some(present_ok);
                                    presentation_reason = Some(if present_ok {
                                        "final answer was concise plain text and matched expected content".to_string()
                                    } else {
                                        "final answer formatting or content did not match expectations".to_string()
                                    });
                                }
                                Err(e) => {
                                    response_reason = Some(format!("judge error: {e}"));
                                    response_ok = Some(false);
                                    response_plain_text = Some(!looks_like_markdown(&final_text));
                                    presentation_total += 1;
                                    presentation_ok = Some(false);
                                    presentation_reason =
                                        Some("presentation judge failed".to_string());
                                }
                            }
                        }
                        Err(e) => {
                            response_reason = Some(format!("reply error: {e}"));
                            response_ok = Some(false);
                            response_plain_text = Some(false);
                            claim_check_total += 1;
                            claim_check_ok = Some(false);
                            claim_check_reason = Some(
                                "claim checker skipped because reply generation failed".to_string(),
                            );
                            presentation_total += 1;
                            presentation_ok = Some(false);
                            presentation_reason = Some("no final answer was produced".to_string());
                        }
                    }
                }
            }

            let all_ok = speech_ok
                && workflow_ok
                && mode_ok.unwrap_or(true)
                && route_ok
                && scope_eval_ok
                && program_eval.parsed
                && program_eval.shape_ok
                && program_eval.policy_ok
                && consistency_ok
                && compaction_ok.unwrap_or(true)
                && classification_ok.unwrap_or(true)
                && execution_ok.unwrap_or(true)
                && critic_ok.unwrap_or(true)
                && claim_check_ok.unwrap_or(true)
                && presentation_ok.unwrap_or(true)
                && response_ok.unwrap_or(true);
            if all_ok {
                all_ok_correct += 1;
            }

            (
                program_eval.signature,
                program_opt
                    .as_ref()
                    .map(|p| p.steps.len())
                    .unwrap_or_default(),
                program_eval.parsed,
                program_eval.parse_error,
                program_eval.shape_ok,
                program_eval.shape_reason,
                program_eval.policy_ok,
                program_eval.policy_reason,
                consistency_ok,
                executed_in_tune,
                execution_ok,
                critic_ok,
                critic_reason,
                response_ok,
                response_reason,
                response_plain_text,
                Some(scope_eval_ok),
                Some(scope_eval_reason),
                compaction_ok,
                compaction_reason,
                classification_ok,
                classification_reason,
                claim_check_ok,
                claim_check_reason,
                presentation_ok,
                presentation_reason,
                Some(tool_economy),
                all_ok,
            )
        };

        scenario_results.push(ScenarioCalibrationResult {
            suite: scenario.suite.clone(),
            file: scenario.file.clone(),
            notes: scenario.notes.clone(),
            speech_act_expected: scenario.speech_act.clone(),
            speech_act_predicted: decision.speech_act.choice.clone(),
            speech_act_probability: probability_of(
                &decision.speech_act.distribution,
                &scenario.speech_act,
            ),
            speech_act_ok: speech_ok,
            workflow_expected: scenario.workflow.clone(),
            workflow_predicted: decision.workflow.choice.clone(),
            workflow_probability: probability_of(
                &decision.workflow.distribution,
                &scenario.workflow,
            ),
            workflow_ok,
            mode_expected: scenario.mode.clone(),
            mode_predicted: scenario.mode.as_ref().map(|_| decision.mode.choice.clone()),
            mode_probability: scenario
                .mode
                .as_ref()
                .map(|m| probability_of(&decision.mode.distribution, m)),
            mode_ok,
            route_expected: scenario.route.clone(),
            route_predicted: decision.route.clone(),
            route_probability: probability_of(&decision.distribution, &scenario.route),
            route_ok,
            program_signature,
            program_parse_ok,
            program_parse_error,
            program_shape_ok,
            program_shape_reason,
            program_policy_ok,
            program_policy_reason,
            program_consistency_ok,
            executed_in_tune,
            execution_ok,
            critic_ok,
            critic_reason,
            response_ok,
            response_reason,
            response_plain_text,
            scope_ok,
            scope_reason,
            compaction_ok,
            compaction_reason,
            classification_ok,
            classification_reason,
            claim_check_ok,
            claim_check_reason,
            presentation_ok,
            presentation_reason,
            tool_economy_score: tool_economy,
            all_ok,
        });

        efficiency_scenarios.push(EfficiencyScenarioResult {
            suite: scenario.suite.clone(),
            file: scenario.file.clone(),
            task_success: all_ok,
            grounding_ok: response_ok,
            scope_ok,
            compaction_ok,
            classification_ok,
            claim_check_ok,
            presentation_ok,
            tool_economy_score: tool_economy.unwrap_or(0.0),
            actual_steps,
            expected_min_steps: scenario.minimum_step_count,
            expected_max_steps: scenario.maximum_step_count,
        });
    }

    let total = scenario_results.len();
    let summary = CalibrationSummary {
        total_cases: total,
        speech_act: calibration_metric(speech_correct, total),
        workflow: calibration_metric(workflow_correct, total),
        mode: calibration_metric(mode_correct, mode_total),
        route: calibration_metric(route_correct, total),
        program_parse: calibration_metric(program_parse_correct, total),
        program_shape: calibration_metric(program_shape_correct, total),
        program_policy: calibration_metric(program_policy_correct, total),
        program_consistency: calibration_metric(program_consistency_correct, total),
        execution: calibration_metric(execution_correct, execution_total),
        critic: calibration_metric(critic_correct, critic_total),
        response: calibration_metric(response_correct, response_total),
        scope: calibration_metric(scope_correct, scope_total),
        compaction: calibration_metric(compaction_correct, compaction_total),
        classification: calibration_metric(classification_correct, classification_total),
        claim_check: calibration_metric(claim_check_correct, claim_check_total),
        presentation: calibration_metric(presentation_correct, presentation_total),
        all_ok: calibration_metric(all_ok_correct, total),
        certified: total > 0
            && calibration_metric(speech_correct, total).accuracy >= 0.80
            && calibration_metric(workflow_correct, total).accuracy >= 0.85
            && calibration_metric(mode_correct, mode_total).accuracy >= 0.80
            && calibration_metric(route_correct, total).accuracy >= 0.85
            && calibration_metric(program_parse_correct, total).accuracy >= 0.95
            && calibration_metric(program_shape_correct, total).accuracy >= 0.85
            && calibration_metric(program_policy_correct, total).accuracy >= 0.95
            && calibration_metric(program_consistency_correct, total).accuracy >= 0.80
            && calibration_metric(execution_correct, execution_total).accuracy >= 0.80
            && calibration_metric(critic_correct, critic_total).accuracy >= 0.80
            && calibration_metric(response_correct, response_total).accuracy >= 0.80
            && calibration_metric(scope_correct, scope_total).accuracy >= 0.75
            && calibration_metric(compaction_correct, compaction_total).accuracy >= 0.75
            && calibration_metric(classification_correct, classification_total).accuracy >= 0.70
            && calibration_metric(claim_check_correct, claim_check_total).accuracy >= 0.75
            && calibration_metric(presentation_correct, presentation_total).accuracy >= 0.80,
        certification_rule: "speech_act>=0.80 workflow>=0.85 mode>=0.80 route>=0.85 parse>=0.95 shape>=0.85 policy>=0.95 consistency>=0.80 execution>=0.80 critic>=0.80 response>=0.80 scope>=0.75 compaction>=0.75 classification>=0.70 claim_check>=0.75 presentation>=0.80".to_string(),
    };
    let report = CalibrationReport {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        supports_logprobs,
        n_probs,
        summary,
        speech_act_confusions: build_confusions(&speech_pairs),
        workflow_confusions: build_confusions(&workflow_pairs),
        mode_confusions: build_confusions(&mode_pairs),
        route_confusions: build_confusions(&route_pairs),
        scenarios: scenario_results,
    };
    let report_path = model_cfg_dir.join("calibration_report.json");
    save_calibration_report(&report_path, &report)?;
    trace(
        args,
        &format!("tune_calibration_report_saved={}", report_path.display()),
    );

    let efficiency_total = efficiency_scenarios.len();
    let task_success_sum = efficiency_scenarios
        .iter()
        .map(|s| if s.task_success { 1.0 } else { 0.0 })
        .sum::<f64>();
    let grounding_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.grounding_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let grounding_total = efficiency_scenarios
        .iter()
        .filter(|s| s.grounding_ok.is_some())
        .count();
    let scope_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.scope_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let scope_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.scope_ok.is_some())
        .count();
    let compaction_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.compaction_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let compaction_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.compaction_ok.is_some())
        .count();
    let classification_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.classification_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let classification_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.classification_ok.is_some())
        .count();
    let claim_check_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.claim_check_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let claim_check_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.claim_check_ok.is_some())
        .count();
    let presentation_sum = efficiency_scenarios
        .iter()
        .filter_map(|s| s.presentation_ok.map(|ok| if ok { 1.0 } else { 0.0 }))
        .sum::<f64>();
    let presentation_metric_total = efficiency_scenarios
        .iter()
        .filter(|s| s.presentation_ok.is_some())
        .count();
    let tool_economy_sum = efficiency_scenarios
        .iter()
        .map(|s| s.tool_economy_score)
        .sum::<f64>();
    let efficiency_summary = EfficiencySummary {
        total_cases: efficiency_total,
        task_success_rate: efficiency_metric_from_score(task_success_sum, efficiency_total),
        grounding_rate: efficiency_metric_from_score(grounding_sum, grounding_total),
        scope_precision: efficiency_metric_from_score(scope_sum, scope_metric_total),
        compaction_rate: efficiency_metric_from_score(compaction_sum, compaction_metric_total),
        classification_rate: efficiency_metric_from_score(
            classification_sum,
            classification_metric_total,
        ),
        claim_check_rate: efficiency_metric_from_score(claim_check_sum, claim_check_metric_total),
        presentation_rate: efficiency_metric_from_score(
            presentation_sum,
            presentation_metric_total,
        ),
        tool_economy: efficiency_metric_from_score(tool_economy_sum, efficiency_total),
        overall_efficiency: (0.30
            * efficiency_metric_from_score(task_success_sum, efficiency_total).score)
            + (0.20 * efficiency_metric_from_score(grounding_sum, grounding_total).score)
            + (0.15 * efficiency_metric_from_score(scope_sum, scope_metric_total).score)
            + (0.05 * efficiency_metric_from_score(compaction_sum, compaction_metric_total).score)
            + (0.05
                * efficiency_metric_from_score(classification_sum, classification_metric_total)
                    .score)
            + (0.10
                * efficiency_metric_from_score(claim_check_sum, claim_check_metric_total).score)
            + (0.05
                * efficiency_metric_from_score(presentation_sum, presentation_metric_total).score)
            + (0.10 * efficiency_metric_from_score(tool_economy_sum, efficiency_total).score),
    };
    let efficiency_report = EfficiencyReport {
        version: 1,
        model: model_id.to_string(),
        base_url: base_url.to_string(),
        summary: efficiency_summary,
        scenarios: efficiency_scenarios,
    };
    let efficiency_path = model_cfg_dir.join("efficiency_report.json");
    save_efficiency_report(&efficiency_path, &efficiency_report)?;
    trace(
        args,
        &format!("tune_efficiency_report_saved={}", efficiency_path.display()),
    );
    if emit_progress {
        calibration_progress(
            args,
            &format!(
                "calibration finished for {model_id}: score {:.3}, certified={}",
                score_calibration_report(&report),
                report.summary.certified
            ),
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f64,
    top_p: f64,
    stream: bool,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_probs: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat_penalty: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_format: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    created: Option<i64>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    system_fingerprint: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    timings: Option<Timings>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ChoiceMessage,
    #[serde(default)]
    finish_reason: Option<String>,
    #[serde(default)]
    logprobs: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ChoiceMessage {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
    reasoning_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: Option<u64>,
    #[serde(default)]
    completion_tokens: Option<u64>,
    #[serde(default)]
    total_tokens: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct Timings {
    #[serde(default)]
    prompt_n: Option<u64>,
    #[serde(default)]
    prompt_ms: Option<f64>,
    #[serde(default)]
    predicted_n: Option<u64>,
    #[serde(default)]
    predicted_ms: Option<f64>,
    #[serde(default)]
    predicted_per_second: Option<f64>,
    #[serde(default)]
    cache_n: Option<u64>,
}

fn prompt_line(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush().ok();

    let mut line = String::new();
    let n = io::stdin().read_line(&mut line)?;
    if n == 0 {
        return Ok(None); // EOF
    }
    let line = line.trim_end_matches(['\n', '\r']).to_string();
    Ok(Some(line))
}

fn ansi_grey(s: &str) -> String {
    // 8-bit grey
    format!("\x1b[90m{s}\x1b[0m")
}

fn ansi_orange(s: &str) -> String {
    // 256-color "orange-ish" (208). Falls back to default if terminal doesn't support it.
    format!("\x1b[38;5;208m{s}\x1b[0m")
}

fn ansi_pale_yellow(s: &str) -> String {
    // 256-color pale yellow.
    format!("\x1b[38;5;229m{s}\x1b[0m")
}

fn ansi_paler_yellow(s: &str) -> String {
    // Pale dark golden (less bright than 229, less grey than 187).
    format!("\x1b[38;5;179m{s}\x1b[0m")
}

static TRACE_LOG_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();

fn trace_log_state() -> &'static Mutex<Option<PathBuf>> {
    TRACE_LOG_PATH.get_or_init(|| Mutex::new(None))
}

fn set_trace_log_path(path: Option<PathBuf>) {
    if let Ok(mut slot) = trace_log_state().lock() {
        *slot = path;
    }
}

fn append_trace_log_line(line: &str) {
    let path = trace_log_state()
        .lock()
        .ok()
        .and_then(|slot| (*slot).clone());
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{line}");
    }
}

fn trace(args: &Args, msg: &str) {
    let line = format!("trace: {msg}");
    append_trace_log_line(&line);
    if args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", ansi_paler_yellow(&line));
        }
    }
}

fn calibration_progress(args: &Args, msg: &str) {
    if args.tune || args.calibrate {
        let line = format!("tune> {msg}");
        append_trace_log_line(&line);
        eprintln!("{line}");
        let _ = io::stderr().flush();
    }
}

fn operator_trace(args: &Args, msg: &str) {
    let line = format!("working> {msg}");
    append_trace_log_line(&line);
    if !(args.tune || args.calibrate) || args.debug_trace {
        if args.no_color {
            eprintln!("{line}");
        } else {
            eprintln!("{}", ansi_grey(&line));
        }
    }
}

/// Strip <think>...</think> blocks. If an opening tag is found without a closing tag,
/// drop the rest to avoid leaking partial reasoning.
fn strip_think_tags(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("<think>") {
        out.push_str(&rest[..start]);
        let after_start = &rest[start + "<think>".len()..];
        if let Some(end) = after_start.find("</think>") {
            rest = &after_start[end + "</think>".len()..];
        } else {
            // Unclosed tag: drop rest.
            rest = "";
            break;
        }
    }
    out.push_str(rest);
    out.trim().to_string()
}

fn describe_operator_intent(
    route: &RouteDecision,
    complexity: &ComplexityAssessment,
    formula: &FormulaSelection,
) -> String {
    if route
        .speech_act
        .choice
        .eq_ignore_ascii_case("CAPABILITY_CHECK")
        && probability_of(&route.speech_act.distribution, "CAPABILITY_CHECK") >= 0.65
    {
        return "answering a capability question".to_string();
    }
    let pattern = if !formula.primary.trim().is_empty() {
        formula.primary.trim()
    } else if !complexity.suggested_pattern.trim().is_empty() {
        complexity.suggested_pattern.trim()
    } else {
        ""
    };
    match pattern {
        "inspect_decide_reply" => {
            "checking workspace evidence before making a decision".to_string()
        }
        "inspect_summarize_reply" => "inspecting workspace evidence and summarizing it".to_string(),
        "inspect_reply" => "looking at workspace evidence before answering".to_string(),
        "execute_reply" => "running a terminal action and preparing the answer".to_string(),
        "plan_reply" => "building a concrete plan".to_string(),
        "masterplan_reply" => "building an overall plan".to_string(),
        "reply" | "reply_only" => "answering directly".to_string(),
        _ => match route.route.as_str() {
            "SHELL" => "working through the workspace".to_string(),
            "PLAN" => "building a concrete plan".to_string(),
            "MASTERPLAN" => "building an overall plan".to_string(),
            "DECIDE" => "weighing the options".to_string(),
            _ => "answering directly".to_string(),
        },
    }
}

fn squash_blank_lines(s: &str) -> String {
    // For chat display: keep newlines, but remove empty-line breaks that look like "two messages".
    let mut out = String::with_capacity(s.len());
    let mut last_was_nl = false;
    let mut nl_run = 0u32;
    for ch in s.chars() {
        if ch == '\n' {
            nl_run += 1;
            if nl_run <= 1 {
                out.push('\n');
            } else {
                // drop extra newlines
            }
            last_was_nl = true;
        } else {
            nl_run = 0;
            if last_was_nl && ch == '\r' {
                continue;
            }
            out.push(ch);
            last_was_nl = false;
        }
    }
    out.trim().to_string()
}

const LLAMA_REASONING_START: &str = "<<<reasoning_content_start>>>";
const LLAMA_REASONING_END: &str = "<<<reasoning_content_end>>>";

fn split_llama_sentinel_reasoning(content: &str) -> (String, Option<String>) {
    // Mirrors llama.cpp WebUI parseReasoningContent(): split content into plain + reasoning parts
    // based on sentinel markers. Unterminated reasoning marker consumes rest.
    let mut plain_parts: Vec<&str> = Vec::new();
    let mut reasoning_parts: Vec<&str> = Vec::new();

    let mut cursor = 0usize;
    while cursor < content.len() {
        let Some(start_idx_rel) = content[cursor..].find(LLAMA_REASONING_START) else {
            plain_parts.push(&content[cursor..]);
            break;
        };
        let start_idx = cursor + start_idx_rel;
        plain_parts.push(&content[cursor..start_idx]);

        let reasoning_start = start_idx + LLAMA_REASONING_START.len();
        if reasoning_start >= content.len() {
            break;
        }

        let Some(end_idx_rel) = content[reasoning_start..].find(LLAMA_REASONING_END) else {
            reasoning_parts.push(&content[reasoning_start..]);
            break;
        };
        let end_idx = reasoning_start + end_idx_rel;
        reasoning_parts.push(&content[reasoning_start..end_idx]);
        cursor = end_idx + LLAMA_REASONING_END.len();
    }

    let plain = plain_parts.join("");
    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join("\n\n"))
    };
    (plain, reasoning)
}

/// Extract (thinking, final) from either structured fields or tagged output.
///
/// Mirrors the Open WebUI "compatible provider" strategy:
/// - Prefer `content` if non-empty, else fall back to `reasoning_content`.
/// - Strip `<think>...</think>` blocks from the final user-visible text.
/// - If tags are present, also return extracted thinking for display.
fn split_thinking_and_final(
    content: Option<&str>,
    reasoning_content: Option<&str>,
) -> (Option<String>, String) {
    let c0 = content.unwrap_or("").trim();
    let r = reasoning_content.unwrap_or("").trim();

    // First, strip llama.cpp sentinel reasoning blocks out of content if present.
    let (c_plain, c_reasoning_from_sentinels) =
        if !c0.is_empty() && c0.contains(LLAMA_REASONING_START) {
            split_llama_sentinel_reasoning(c0)
        } else {
            (c0.to_string(), None)
        };
    let c = c_plain.trim();

    // If both exist, treat reasoning_content as thinking and content as final.
    // Also treat sentinel-extracted reasoning as thinking when present.
    if !c.is_empty() && (!r.is_empty() || c_reasoning_from_sentinels.is_some()) {
        let thinking = if !r.is_empty() {
            Some(r.to_string())
        } else {
            c_reasoning_from_sentinels
        };
        return (thinking, strip_think_tags(c));
    }

    let text = if !c.is_empty() { c } else { r };
    if text.is_empty() {
        return (None, String::new());
    }

    // Parse <think> tags if present.
    if text.contains("<think>") {
        let mut thinking = String::new();
        let mut final_out = String::new();
        let mut rest = text;

        while let Some(s) = rest.find("<think>") {
            final_out.push_str(&rest[..s]);
            let after_start = &rest[s + "<think>".len()..];
            if let Some(e) = after_start.find("</think>") {
                let chunk = &after_start[..e];
                if !chunk.trim().is_empty() {
                    if !thinking.is_empty() {
                        thinking.push_str("\n\n");
                    }
                    thinking.push_str(chunk.trim());
                }
                rest = &after_start[e + "</think>".len()..];
            } else {
                // Unclosed tag: treat remaining as thinking and stop.
                let chunk = after_start;
                if !chunk.trim().is_empty() {
                    if !thinking.is_empty() {
                        thinking.push_str("\n\n");
                    }
                    thinking.push_str(chunk.trim());
                }
                rest = "";
                break;
            }
        }
        final_out.push_str(rest);
        let final_out = final_out.trim().to_string();

        let thinking_opt = if thinking.trim().is_empty() {
            None
        } else {
            Some(thinking)
        };
        return (thinking_opt, final_out);
    }

    (None, strip_think_tags(text))
}

fn extract_final_line(text: &str, prefix: &str) -> Option<String> {
    let p = prefix.trim();
    if p.is_empty() {
        return None;
    }
    // Find the last line that begins with the prefix (case-sensitive).
    let mut last: Option<String> = None;
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with(p) {
            let rest = l[p.len()..].trim();
            last = Some(rest.to_string());
        }
    }
    last.filter(|s| !s.trim().is_empty())
}

fn remove_final_lines(text: &str, prefix: &str) -> String {
    let p = prefix.trim();
    if p.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with(p) {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

async fn chat_once(
    client: &reqwest::Client,
    chat_url: &Url,
    req: &ChatCompletionRequest,
) -> Result<ChatCompletionResponse> {
    let mut last_error = String::new();
    for attempt in 0..3u32 {
        match client.post(chat_url.clone()).json(req).send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = resp.text().await.context("Failed to read response body")?;
                if !status.is_success() {
                    if status.is_server_error() && attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                        last_error = format!("Server returned HTTP {status}: {text}");
                        continue;
                    }
                    anyhow::bail!("Server returned HTTP {status}: {text}");
                }

                let parsed: ChatCompletionResponse =
                    serde_json::from_str(&text).context("Invalid JSON from server")?;
                return Ok(parsed);
            }
            Err(e) => {
                last_error = format!("{e:#}");
                if attempt < 2 {
                    tokio::time::sleep(Duration::from_millis(500 * (1 << attempt))).await;
                    continue;
                }
            }
        }
    }
    anyhow::bail!("POST /v1/chat/completions failed after retries: {last_error}")
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mode_flags = [
        args.tune,
        args.calibrate,
        args.restore_base,
        args.restore_last,
    ];
    if mode_flags.into_iter().filter(|v| *v).count() > 1 {
        anyhow::bail!("Choose only one of --tune, --calibrate, --restore-base, or --restore-last");
    }

    let cfg_root = config_root_path(&args.config_root)?;
    let (base_url, base_url_source) =
        resolve_base_url(&cfg_root, args.base_url.as_deref(), args.model.as_deref());

    let base = Url::parse(&base_url).context("Invalid --base-url")?;
    let chat_url = base
        .join("/v1/chat/completions")
        .context("Failed to build /v1/chat/completions URL")?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("Failed to build HTTP client")?;

    let model_id = if let Some(m) = args.model.as_ref().filter(|s| !s.trim().is_empty()) {
        m.trim().to_string()
    } else {
        fetch_first_model_id(&client, &base).await?
    };

    let model_cfg_dir = ensure_model_config_folder(&cfg_root, &base_url, &model_id)?;

    if args.restore_base {
        let baseline_dir = ensure_baseline_profile_set(&model_cfg_dir, &base_url, &model_id)?;
        activate_profile_set(
            &model_cfg_dir,
            &baseline_dir,
            &base_url,
            &model_id,
            "baseline",
            None,
            0.0,
            false,
        )?;
        eprintln!(
            "Restored baseline profiles for {} from {}",
            model_id,
            baseline_dir.display()
        );
        return Ok(());
    }

    if args.restore_last {
        let fallback_dir = model_fallback_last_active_dir(&model_cfg_dir);
        if !fallback_dir.exists() {
            anyhow::bail!(
                "No last-active profile snapshot found for {} at {}",
                model_id,
                fallback_dir.display()
            );
        }
        activate_profile_set(
            &model_cfg_dir,
            &fallback_dir,
            &base_url,
            &model_id,
            "fallback_last_active",
            None,
            0.0,
            false,
        )?;
        eprintln!(
            "Restored last active profiles for {} from {}",
            model_id,
            fallback_dir.display()
        );
        return Ok(());
    }

    if args.tune || args.calibrate {
        let model_ids = if args.all_models {
            fetch_all_model_ids(&client, &base).await?
        } else {
            vec![model_id.clone()]
        };
        for mid in model_ids {
            let dir = ensure_model_config_folder(&cfg_root, &base_url, &mid)?;
            if args.calibrate {
                let tune_cfg = load_agent_config(&dir.join("intention_tune.toml"))?;
                tune_model(
                    &args, &client, &chat_url, &base_url, &dir, &mid, &tune_cfg, true,
                )
                .await?;
            } else {
                let winner =
                    optimize_model(&args, &client, &chat_url, &base_url, &dir, &mid).await?;
                eprintln!(
                    "Activated tuned profiles for {} with score {:.3} (certified: {}).",
                    mid, winner.score, winner.report.summary.certified
                );
                eprintln!("Restore last: cargo run -- --model {} --restore-last", mid);
                eprintln!("Restore base: cargo run -- --model {} --restore-base", mid);
            }
        }
        return Ok(());
    }

    let elma_cfg_path = model_cfg_dir.join("_elma.config");
    let planner_master_cfg_path = model_cfg_dir.join("planner_master.toml");
    let planner_cfg_path = model_cfg_dir.join("planner.toml");
    let decider_cfg_path = model_cfg_dir.join("decider.toml");
    let summarizer_cfg_path = model_cfg_dir.join("summarizer.toml");
    let formatter_cfg_path = model_cfg_dir.join("formatter.toml");
    let complexity_cfg_path = model_cfg_dir.join("complexity_assessor.toml");
    let formula_cfg_path = model_cfg_dir.join("formula_selector.toml");
    let command_repair_cfg_path = model_cfg_dir.join("command_repair.toml");
    let scope_builder_cfg_path = model_cfg_dir.join("scope_builder.toml");
    let evidence_compactor_cfg_path = model_cfg_dir.join("evidence_compactor.toml");
    let artifact_classifier_cfg_path = model_cfg_dir.join("artifact_classifier.toml");
    let result_presenter_cfg_path = model_cfg_dir.join("result_presenter.toml");
    let claim_checker_cfg_path = model_cfg_dir.join("claim_checker.toml");
    let orchestrator_cfg_path = model_cfg_dir.join("orchestrator.toml");
    let critic_cfg_path = model_cfg_dir.join("critic.toml");
    let router_cfg_path = model_cfg_dir.join("router.toml");
    let mode_router_cfg_path = model_cfg_dir.join("mode_router.toml");
    let speech_act_cfg_path = model_cfg_dir.join("speech_act.toml");
    let router_cal_path = model_cfg_dir.join("router_calibration.toml");

    let mut elma_cfg = load_agent_config(&elma_cfg_path)?;
    let planner_master_cfg = load_agent_config(&planner_master_cfg_path)?;
    let planner_cfg = load_agent_config(&planner_cfg_path)?;
    let decider_cfg = load_agent_config(&decider_cfg_path)?;
    let summarizer_cfg = load_agent_config(&summarizer_cfg_path)?;
    let formatter_cfg = load_agent_config(&formatter_cfg_path)?;
    let complexity_cfg = load_agent_config(&complexity_cfg_path)?;
    let formula_cfg = load_agent_config(&formula_cfg_path)?;
    let command_repair_cfg = load_agent_config(&command_repair_cfg_path)?;
    let scope_builder_cfg = load_agent_config(&scope_builder_cfg_path)?;
    let evidence_compactor_cfg = load_agent_config(&evidence_compactor_cfg_path)?;
    let artifact_classifier_cfg = load_agent_config(&artifact_classifier_cfg_path)?;
    let result_presenter_cfg = load_agent_config(&result_presenter_cfg_path)?;
    let claim_checker_cfg = load_agent_config(&claim_checker_cfg_path)?;
    let mut orchestrator_cfg = load_agent_config(&orchestrator_cfg_path)?;
    let mut critic_cfg = load_agent_config(&critic_cfg_path)?;
    let mut router_cfg = load_agent_config(&router_cfg_path)?;
    let mut mode_router_cfg = load_agent_config(&mode_router_cfg_path)?;
    let mut speech_act_cfg = load_agent_config(&speech_act_cfg_path)?;
    let router_cal = load_router_calibration(&router_cal_path)?;

    // Ensure these configs track current base/model (user can still edit files manually).
    elma_cfg.base_url = base_url.clone();
    elma_cfg.model = model_id.clone();
    save_agent_config(&elma_cfg_path, &elma_cfg)?;

    if replace_system_prompt_if_missing(
        &mut router_cfg,
        "router",
        "2 = WORKFLOW",
        default_router_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=router.system_prompt");
        save_agent_config(&router_cfg_path, &router_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut mode_router_cfg,
        "mode_router",
        "1 = INSPECT",
        default_mode_router_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=mode_router.system_prompt");
        save_agent_config(&mode_router_cfg_path, &mode_router_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut speech_act_cfg,
        "speech_act",
        "1 = CAPABILITY_CHECK",
        default_speech_act_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=speech_act.system_prompt");
        save_agent_config(&speech_act_cfg_path, &speech_act_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut orchestrator_cfg,
        "orchestrator",
        "EVIDENCE-FIRST RULES",
        default_orchestrator_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=orchestrator.system_prompt");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if replace_system_prompt_if_missing(
        &mut critic_cfg,
        "critic",
        "there is no workspace evidence in the step results",
        default_critic_config(&base_url, &model_id).system_prompt,
    ) {
        trace(&args, "upgraded=critic.system_prompt");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "ROUTER PRIOR RULES:\n- You will receive a probabilistic route prior over CHAT, SHELL, PLAN, MASTERPLAN, and DECIDE.\n- Treat the route prior as evidence, not a hard rule.\n- If the route prior is uncertain or the user request is genuinely ambiguous, you may output a Program with a single reply step that asks one concise clarifying question.",
    ) {
        trace(&args, "upgraded=orchestrator.router_prior");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "- A shell step is for real workspace inspection or execution only. Never use shell steps to print prose, plan lines, or explanations.\n- If the user asks for a plan, prefer a plan or masterplan step plus an optional reply step. Do not emit plan text through shell commands.",
    ) {
        trace(&args, "upgraded=orchestrator.shell_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "- If the user asks for one concrete step-by-step plan, use a plan step.\n- If the user asks for a higher-level overall plan across phases, use a masterplan step.",
    ) {
        trace(&args, "upgraded=orchestrator.plan_distinction");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "STRUCTURE RULES:\n- Every step must include purpose and success_condition.\n- Use depends_on to reference earlier step ids when a later step consumes prior results.\n- For summarize steps that summarize earlier outputs, leave text empty and set depends_on.\n- Keep programs minimal. Remove any step that does not directly advance the objective.",
    ) {
        trace(&args, "upgraded=orchestrator.structure_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "PLAN EXAMPLE:\nUser: Create a step-by-step plan to add a new config file to this Rust project.\nOutput:\n{\"objective\":\"create a concrete plan for adding a config file\",\"steps\":[{\"id\":\"p1\",\"type\":\"plan\",\"goal\":\"Add a new config file to this Rust project.\",\"purpose\":\"plan\",\"depends_on\":[],\"success_condition\":\"a concrete step-by-step plan is saved\"},{\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"Tell the user a step-by-step plan was created and summarize it briefly in plain text.\",\"purpose\":\"answer\",\"depends_on\":[\"p1\"],\"success_condition\":\"the user receives a concise plain-text summary of the saved plan\"}]}",
    ) {
        trace(&args, "upgraded=orchestrator.plan_example");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "MINIMALITY RULES:\n- For a step-by-step plan request, default to one plan step plus an optional reply step.\n- Do not inspect src/main.rs, config files, or prompt files just because examples mention them.\n- Only add shell inspection to a plan request when the plan truly depends on current workspace evidence.",
    ) {
        trace(&args, "upgraded=orchestrator.minimality_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut router_cfg,
        "router",
        "Important distinctions:\n- Greetings or general knowledge questions are usually 1.\n- Questions about the current project, files, code, or tasks that need planning or decisions are usually 2.",
    ) {
        trace(&args, "upgraded=router.examples");
        save_agent_config(&router_cfg_path, &router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut router_cfg,
        "router",
        "- Output must be exactly one digit from 1 to 2.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents whether Elma should enter workflow mode.",
    ) {
        trace(&args, "upgraded=router.workflow_rules");
        save_agent_config(&router_cfg_path, &router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut mode_router_cfg,
        "mode_router",
        "Important distinctions:\n- \"What is my current project about?\", \"read Cargo.toml and summarize it\", and \"find where fetch_ctx_max is defined\" are usually 1.\n- \"list files\", \"run tests\", and \"build the project\" are usually 2.\n- \"Create a step-by-step plan\" is 3, not 4.\n- Only choose 4 when the user truly wants an overall master plan.",
    ) {
        trace(&args, "upgraded=mode_router.examples");
        save_agent_config(&mode_router_cfg_path, &mode_router_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut speech_act_cfg,
        "speech_act",
        "Important distinctions:\n- \"Are you able to list files here?\" is usually 1.\n- \"What is my current project about?\" is usually 2.\n- \"Can you list files?\" and \"Could you run the tests?\" are usually 3 in normal English, because they are indirect requests.",
    ) {
        trace(&args, "upgraded=speech_act.examples");
        save_agent_config(&speech_act_cfg_path, &speech_act_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "- If a shell step only prints prose or plan text instead of inspecting or executing something real in the workspace, choose retry.",
    ) {
        trace(&args, "upgraded=critic.shell_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "- If the user asked for a step-by-step plan and there is no plan step result, choose retry and provide a corrected Program that uses type \"plan\".\n- If the user asked for an overall or master plan and there is no masterplan step result, choose retry and provide a corrected Program that uses type \"masterplan\".",
    ) {
        trace(&args, "upgraded=critic.plan_distinction");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "EVALUATION RULES:\n- Judge whether each step's purpose and success_condition actually advanced the objective.\n- If a step has depends_on, verify the dependent outputs were meaningfully used.\n- For planning requests, reject shell steps unless they gather clearly necessary workspace evidence.\n- Prefer the simplest valid program that can satisfy the request.",
    ) {
        trace(&args, "upgraded=critic.evaluation_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "PLAN VALIDATION HINTS:\n- If any successful step_result has type \"plan\", the step-by-step plan requirement is satisfied.\n- If any successful step_result has type \"masterplan\", the master plan requirement is satisfied.\n- For a step-by-step plan request, reject unnecessary shell inspection and prefer a corrected program with only a plan step and an optional reply step.",
    ) {
        trace(&args, "upgraded=critic.plan_validation_hints");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "SPEECH-ACT RULES:\n- You will receive a probabilistic speech-act prior over CAPABILITY_CHECK, INFO_REQUEST, and ACTION_REQUEST.\n- If CAPABILITY_CHECK dominates, prefer a reply step that answers whether Elma can do it. Do not execute commands unless the user also asked for action now.\n- INFO_REQUEST may still require workspace inspection before answering.\n- ACTION_REQUEST may use shell, plan, masterplan, or decide steps as needed.",
    ) {
        trace(&args, "upgraded=orchestrator.speech_act_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut orchestrator_cfg,
        "orchestrator",
        "COMPLEXITY AND FORMULA PRIORS:\n- You may receive a complexity prior and a formula prior.\n- Treat them as guidance, not hard rules.\n- For cleanup, safety review, or comparison requests about the workspace, prefer inspect_decide_reply.\n- If a shell command fails because of regex, glob, quoting, or parser issues, repair it once and continue if safe.",
    ) {
        trace(&args, "upgraded=orchestrator.complexity_formula_rules");
        save_agent_config(&orchestrator_cfg_path, &orchestrator_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "CLEANUP VALIDATION:\n- If the user asked what is safe to clean up and there is no inspected workspace evidence, choose retry.\n- If a cleanup answer classifies files after a failed shell step, choose retry.\n- If a cleanup task used DECIDE without prior inspection, choose retry.",
    ) {
        trace(&args, "upgraded=critic.cleanup_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }
    if maybe_upgrade_system_prompt(&mut elma_cfg, "_elma", prompt_patch_elma_grounding()) {
        trace(&args, "upgraded=elma.grounding_rules");
        save_agent_config(&elma_cfg_path, &elma_cfg)?;
    }
    if maybe_upgrade_system_prompt(
        &mut critic_cfg,
        "critic",
        "SPEECH-ACT VALIDATION:\n- If speech_act is CAPABILITY_CHECK and the program executed shell or planning actions without explicit user intent to do so now, choose retry and replace it with a reply-only program.\n- If speech_act is ACTION_REQUEST, reject answers that only talk about capability without attempting the task when it is allowed.",
    ) {
        trace(&args, "upgraded=critic.speech_act_rules");
        save_agent_config(&critic_cfg_path, &critic_cfg)?;
    }

    let ctx_max = fetch_ctx_max(&client, &base).await.unwrap_or(None);

    let sessions_root = sessions_root_path(&args.sessions_root)?;
    let session = ensure_session_layout(&sessions_root)?;
    set_trace_log_path(Some(session.root.join("trace_debug.log")));

    // Workspace intel unit: gather real facts about where we are and inject them
    // into Elma's context so she doesn't hallucinate access constraints.
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    let ws_brief = gather_workspace_brief(&repo);
    if !ws.is_empty() {
        let p = session.root.join("workspace.txt");
        std::fs::write(&p, ws.trim().to_string() + "\n")
            .with_context(|| format!("write {}", p.display()))?;
        trace(&args, &format!("workspace_context_saved={}", p.display()));
    }
    if !ws_brief.is_empty() {
        let p = session.root.join("workspace_brief.txt");
        std::fs::write(&p, ws_brief.trim().to_string() + "\n")
            .with_context(|| format!("write {}", p.display()))?;
        trace(&args, &format!("workspace_brief_saved={}", p.display()));
    }
    trace(
        &args,
        &format!("base_url_source={base_url_source} value={base_url}"),
    );

    let mut system_content = elma_cfg.system_prompt.clone();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    if !ws_brief.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE BRIEF:\n");
        system_content.push_str(ws_brief.trim());
    }
    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: system_content.clone(),
    }];

    eprintln!("Connected target: {chat_url}");
    eprintln!("Model: {model_id}");
    eprintln!("Config: {}", model_cfg_dir.display());
    eprintln!("Session: {}", session.root.display());
    eprintln!("Type /exit to quit, /reset to clear history.\n");
    // No explicit slash workflows for now; formulas should be orchestrated automatically.

    loop {
        let Some(line) = prompt_line("you> ")? else {
            break;
        };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "/exit" || line == "/quit" {
            break;
        }
        if line == "/reset" {
            messages.truncate(1); // keep system
            eprintln!("(history reset)");
            continue;
        }

        // Explicit slash workflows removed; formulas are executed automatically.

        messages.push(ChatMessage {
            role: "user".to_string(),
            content: line.to_string(),
        });

        let route_decision = infer_route_prior(
            &client,
            &chat_url,
            &speech_act_cfg,
            &router_cfg,
            &mode_router_cfg,
            &router_cal,
            line,
            &ws,
            &ws_brief,
            &messages,
        )
        .await?;
        trace(
            &args,
            &format!(
                "speech_act_dist={}",
                format_route_distribution(&route_decision.speech_act.distribution)
            ),
        );
        trace(
            &args,
            &format!(
                "speech_act={} p={:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.speech_act.choice,
                route_decision
                    .speech_act
                    .distribution
                    .first()
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0),
                route_decision.speech_act.margin,
                route_decision.speech_act.entropy,
                route_decision.speech_act.source
            ),
        );
        trace(
            &args,
            &format!(
                "workflow_dist={}",
                format_route_distribution(&route_decision.workflow.distribution)
            ),
        );
        trace(
            &args,
            &format!(
                "workflow={} p={:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.workflow.choice,
                route_decision
                    .workflow
                    .distribution
                    .first()
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0),
                route_decision.workflow.margin,
                route_decision.workflow.entropy,
                route_decision.workflow.source
            ),
        );
        trace(
            &args,
            &format!(
                "mode_dist={}",
                format_route_distribution(&route_decision.mode.distribution)
            ),
        );
        trace(
            &args,
            &format!(
                "mode={} p={:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.mode.choice,
                route_decision
                    .mode
                    .distribution
                    .first()
                    .map(|(_, p)| *p)
                    .unwrap_or(0.0),
                route_decision.mode.margin,
                route_decision.mode.entropy,
                route_decision.mode.source
            ),
        );
        trace(
            &args,
            &format!(
                "route_dist={}",
                format_route_distribution(&route_decision.distribution)
            ),
        );
        let route_p = route_decision
            .distribution
            .first()
            .map(|(_, p)| *p)
            .unwrap_or(0.0);
        trace(
            &args,
            &format!(
                "route={} p={route_p:.2} margin={:.2} entropy={:.2} source={}",
                route_decision.route,
                route_decision.margin,
                route_decision.entropy,
                route_decision.source
            ),
        );
        let complexity = assess_complexity_once(
            &client,
            &chat_url,
            &complexity_cfg,
            line,
            &route_decision,
            &ws,
            &ws_brief,
            &messages,
        )
        .await
        .unwrap_or_default();
        trace(
            &args,
            &format!(
                "complexity={} pattern={} risk={}",
                if complexity.complexity.is_empty() {
                    "UNKNOWN"
                } else {
                    &complexity.complexity
                },
                if complexity.suggested_pattern.is_empty() {
                    "unknown"
                } else {
                    &complexity.suggested_pattern
                },
                if complexity.risk.is_empty() {
                    "UNKNOWN"
                } else {
                    &complexity.risk
                }
            ),
        );
        let scope = build_scope_once(
            &client,
            &chat_url,
            &scope_builder_cfg,
            line,
            &route_decision,
            &complexity,
            &ws,
            &ws_brief,
            &messages,
        )
        .await
        .unwrap_or_default();
        if !scope.reason.trim().is_empty() || !scope.focus_paths.is_empty() {
            operator_trace(
                &args,
                &format!(
                    "narrowing the scope{}",
                    if scope.focus_paths.is_empty() {
                        String::new()
                    } else {
                        format!(" to {}", scope.focus_paths.join(", "))
                    }
                ),
            );
        }
        trace(
            &args,
            &format!(
                "scope focus={} include={} exclude={} query={} reason={}",
                if scope.focus_paths.is_empty() {
                    "-".to_string()
                } else {
                    scope.focus_paths.join(",")
                },
                if scope.include_globs.is_empty() {
                    "-".to_string()
                } else {
                    scope.include_globs.join(",")
                },
                if scope.exclude_globs.is_empty() {
                    "-".to_string()
                } else {
                    scope.exclude_globs.join(",")
                },
                if scope.query_terms.is_empty() {
                    "-".to_string()
                } else {
                    scope.query_terms.join(",")
                },
                scope.reason
            ),
        );
        let memories = load_recent_formula_memories(&model_cfg_dir, 8).unwrap_or_default();
        let formula = select_formula_once(
            &client,
            &chat_url,
            &formula_cfg,
            line,
            &route_decision,
            &complexity,
            &scope,
            &memories,
            &messages,
        )
        .await
        .unwrap_or_default();
        trace(
            &args,
            &format!(
                "formula={} alt={} reason={}",
                if formula.primary.is_empty() {
                    "unknown"
                } else {
                    &formula.primary
                },
                if formula.alternatives.is_empty() {
                    "-".to_string()
                } else {
                    formula.alternatives.join(",")
                },
                if formula.memory_id.trim().is_empty() {
                    formula.reason.clone()
                } else {
                    format!("{} memory={}", formula.reason, formula.memory_id)
                }
            ),
        );
        operator_trace(
            &args,
            &describe_operator_intent(&route_decision, &complexity, &formula),
        );

        let mut program = match orchestrate_program_once(
            &client,
            &chat_url,
            &orchestrator_cfg,
            line,
            &route_decision,
            &complexity,
            &scope,
            &formula,
            &ws,
            &ws_brief,
            &messages,
        )
        .await
        {
            Ok((p, _raw)) => p,
            Err(e) => {
                trace(&args, &format!("orchestrator_repair_parse_error={e}"));
                Program {
                    objective: "fallback_chat".to_string(),
                    steps: vec![Step::Reply {
                        id: "r1".to_string(),
                        instructions: "Reply to the user in plain terminal text. Do not invent workspace facts you did not inspect.".to_string(),
                        common: StepCommon::default(),
                    }],
                }
            }
        };
        if apply_capability_guard(&mut program, &route_decision) {
            trace(&args, "guard=capability_reply_only");
        }

        let workdir = repo_root()?;
        let (mut step_results, mut final_reply) = execute_program(
            &args,
            &client,
            &chat_url,
            &session,
            &workdir,
            &program,
            &planner_cfg,
            &planner_master_cfg,
            &decider_cfg,
            &summarizer_cfg,
            Some(&command_repair_cfg),
            Some(&evidence_compactor_cfg),
            Some(&artifact_classifier_cfg),
            &scope,
            &complexity,
            &formula,
            &program.objective,
            false,
            false,
        )
        .await?;

        // Critic repair loop (1 retry max).
        let mut replied = false;
        for attempt in 0..=1u32 {
            if replied {
                break;
            }
            let verdict: CriticVerdict = match run_critic_once(
                &client,
                &chat_url,
                &critic_cfg,
                line,
                &route_decision,
                &program,
                &step_results,
                attempt,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    trace(&args, &format!("critic_parse_error={e}"));
                    CriticVerdict {
                        status: "ok".to_string(),
                        reason: "critic_parse_error".to_string(),
                        program: None,
                    }
                }
            };
            trace(
                &args,
                &format!("critic_status={} reason={}", verdict.status, verdict.reason),
            );

            if verdict.status.eq_ignore_ascii_case("retry") {
                if let Some(p) = verdict.program {
                    program = p;
                    if apply_capability_guard(&mut program, &route_decision) {
                        trace(&args, "guard=capability_reply_only_retry");
                    }
                    let (retry_results, retry_reply) = execute_program(
                        &args,
                        &client,
                        &chat_url,
                        &session,
                        &workdir,
                        &program,
                        &planner_cfg,
                        &planner_master_cfg,
                        &decider_cfg,
                        &summarizer_cfg,
                        Some(&command_repair_cfg),
                        Some(&evidence_compactor_cfg),
                        Some(&artifact_classifier_cfg),
                        &scope,
                        &complexity,
                        &formula,
                        &program.objective,
                        false,
                        false,
                    )
                    .await?;
                    step_results.extend(retry_results);
                    if retry_reply.is_some() {
                        final_reply = retry_reply;
                    }
                    continue;
                }
            }

            // Produce final response via Elma using tool outputs.
            let reply_instructions = final_reply.clone().unwrap_or_else(|| {
                "Respond to the user in plain terminal text. Use any step outputs as evidence."
                    .to_string()
            });
            let (final_text, final_usage_total) = match generate_final_answer_once(
                &client,
                &chat_url,
                &elma_cfg,
                &result_presenter_cfg,
                &claim_checker_cfg,
                &formatter_cfg,
                &system_content,
                line,
                &route_decision,
                &step_results,
                &reply_instructions,
            )
            .await
            {
                Ok(v) => v,
                Err(e) => {
                    trace(&args, &format!("reply_generation_error={e}"));
                    (
                        "I ran into a reply-generation error after executing the workflow."
                            .to_string(),
                        None,
                    )
                }
            };
            println!(
                "{}",
                if args.no_color {
                    format!("bot> {final_text}")
                } else {
                    ansi_orange(&format!("bot> {final_text}"))
                }
            );

            if let Some(ctx) = ctx_max {
                if let Some(total) = final_usage_total {
                    let pct = (total as f64 / ctx as f64) * 100.0;
                    let used_k = {
                        let k = ((total as f64) / 1000.0).round() as u64;
                        if total > 0 {
                            k.max(1)
                        } else {
                            0
                        }
                    };
                    let ctx_k = ((ctx as f64) / 1000.0).round() as u64;
                    let line = format!("ctx: {used_k}k/{ctx_k}k [{pct:.1}%]");
                    println!(
                        "{}",
                        if args.no_color {
                            line
                        } else {
                            ansi_pale_yellow(&line)
                        }
                    );
                }
            }
            println!();

            if !final_text.is_empty() {
                if step_results.iter().all(|r| r.ok)
                    && !route_decision.route.eq_ignore_ascii_case("CHAT")
                    && formula.memory_id.trim().is_empty()
                {
                    let now = now_unix_s()?;
                    let record = FormulaMemoryRecord {
                        id: format!("fm_{now}"),
                        created_unix_s: now,
                        user_message: line.to_string(),
                        route: route_decision.route.clone(),
                        complexity: complexity.complexity.clone(),
                        formula: if formula.primary.trim().is_empty() {
                            complexity.suggested_pattern.clone()
                        } else {
                            formula.primary.clone()
                        },
                        objective: program.objective.clone(),
                        title: if !scope.objective.trim().is_empty() {
                            scope.objective.clone()
                        } else {
                            line.to_string()
                        },
                        program_signature: program_signature(&program),
                    };
                    if let Ok(path) = save_formula_memory(&model_cfg_dir, &record) {
                        trace(&args, &format!("formula_memory_saved={}", path.display()));
                    }
                }
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: final_text,
                });
            }
            replied = true;
        }

        continue;
    }

    Ok(())
}
