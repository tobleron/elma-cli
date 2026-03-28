use anyhow::{Context, Result};
use clap::Parser;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Parser, Debug)]
#[command(name = "elma-cli", version, about = "Minimal chat CLI for llama.cpp /v1/chat/completions")]
struct Args {
    /// Base URL of the server (example: http://192.168.1.186:8080)
    #[arg(long, env = "LLAMA_BASE_URL", default_value = "http://localhost:8080")]
    base_url: String,

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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RouterCalibration {
    version: u32,
    model: String,
    base_url: String,
    n_probs: u32,
    supports_logprobs: bool,
    routes: Vec<String>,
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
    let url = base_url.join("/v1/models").context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp.text().await.context("Failed to read /v1/models body")?;
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
    let url = base_url.join("/v1/models").context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp.text().await.context("Failed to read /v1/models body")?;
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
    let url = base_url.join("/v1/models").context("Failed to build /v1/models URL")?;
    let resp = client.get(url).send().await.context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp.text().await.context("Failed to read /v1/models body")?;
    if !status.is_success() {
        return Ok(None);
    }
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    Ok(v
        .get("data")
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
        save_agent_config(&intention_path, &default_intention_config(base_url, model_id))?;
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
        save_agent_config(&tune_path, &default_intention_tune_config(base_url, model_id))?;
    }
    let action_type_path = dir.join("action_type.toml");
    if !action_type_path.exists() {
        save_agent_config(&action_type_path, &default_action_type_config(base_url, model_id))?;
    }
    let summarizer_path = dir.join("summarizer.toml");
    if !summarizer_path.exists() {
        save_agent_config(&summarizer_path, &default_summarizer_config(base_url, model_id))?;
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
                n_probs: 32,
                supports_logprobs: false,
                routes: vec![
                    "CHAT".to_string(),
                    "SHELL".to_string(),
                    "PLAN".to_string(),
                    "MASTERPLAN".to_string(),
                    "DECIDE".to_string(),
                ],
            },
        )?;
    }

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
    let sw_vers = cmd_out("command -v sw_vers >/dev/null 2>&1 && sw_vers || true", repo_root);
    let whoami = cmd_out("whoami", repo_root);
    let pwd = cmd_out("pwd", repo_root);
    let tty = cmd_out("tty || true", repo_root);

    let mut s = String::new();
    s.push_str(&format!("cwd: {}\n", if !pwd.is_empty() { pwd } else { repo_root.display().to_string() }));
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
            return Some(tok.trim_matches(|c: char| c == '"' || c == '\'' || c == '`').to_string());
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

    std::fs::create_dir_all(&shell_dir).with_context(|| format!("mkdir {}", shell_dir.display()))?;
    std::fs::create_dir_all(&plans_dir).with_context(|| format!("mkdir {}", plans_dir.display()))?;
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
    for ent in std::fs::read_dir(shell_dir)
        .with_context(|| format!("read_dir {}", shell_dir.display()))?
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
    let banned = [
        "curl",
        "wget",
        "ssh",
        "scp",
        "rsync",
        "nc ",
        "netcat",
        "ping",
        "rm -rf",
        "sudo",
        "shutdown",
        "reboot",
    ];
    !banned.iter().any(|b| lower.contains(b))
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
    for ent in std::fs::read_dir(plans_dir)
        .with_context(|| format!("read_dir {}", plans_dir.display()))?
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
    for ent in std::fs::read_dir(tune_dir).with_context(|| format!("read_dir {}", tune_dir.display()))?
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
        let Some((expected, tags)) = l.split_once(':') else { continue };
        let expected = expected.trim().to_string();
        let parts: Vec<String> = tags
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 3 {
            out.push((expected, [parts[0].clone(), parts[1].clone(), parts[2].clone()]));
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

async fn tune_model(
    args: &Args,
    client: &reqwest::Client,
    chat_url: &Url,
    model_cfg_dir: &PathBuf,
    model_id: &str,
    intention_tune_cfg: &Profile,
) -> Result<()> {
    // 1) Build intention_mapping.txt from scenario files.
    let scenario_paths = list_intention_scenario_paths()?;
    let mut lines: Vec<String> = Vec::new();
    for p in scenario_paths {
        let txt = std::fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
        let Some(expected) = read_expected_line(&txt) else { continue };

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
            .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
            .unwrap_or_default();
        let tags = parse_three_tags(&raw);
        lines.push(format!("{}: {}, {}, {}", expected, tags[0], tags[1], tags[2]));
    }
    let mapping_path = model_cfg_dir.join("intention_mapping.txt");
    std::fs::write(&mapping_path, lines.join("\n") + "\n")
        .with_context(|| format!("write {}", mapping_path.display()))?;
    trace(args, &format!("tune_intention_mapping_saved={}", mapping_path.display()));

    // 2) Router calibration: check whether server returns logprobs for top_logprobs.
    // We can't perfectly guarantee inclusion in top_logprobs, but we can verify support and
    // choose an n_probs default that is "big enough".
    let routes = vec![
        "CHAT".to_string(),
        "SHELL".to_string(),
        "PLAN".to_string(),
        "MASTERPLAN".to_string(),
        "DECIDE".to_string(),
    ];
    let n_probs = 64u32;
    let cal_req = ChatCompletionRequest {
        model: model_id.to_string(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: "Return exactly one token: CHAT.\nNo other text.".to_string(),
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
        base_url: args.base_url.clone(),
        n_probs,
        supports_logprobs,
        routes,
    };
    let cal_path = model_cfg_dir.join("router_calibration.toml");
    save_router_calibration(&cal_path, &cal)?;
    trace(args, &format!("tune_router_calibration_saved={}", cal_path.display()));

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

fn trace(args: &Args, msg: &str) {
    let line = format!("trace: {msg}");
    if args.no_color {
        eprintln!("{line}");
    } else {
        eprintln!("{}", ansi_paler_yellow(&line));
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
    let (c_plain, c_reasoning_from_sentinels) = if !c0.is_empty() && c0.contains(LLAMA_REASONING_START)
    {
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
    let resp = client
        .post(chat_url.clone())
        .json(req)
        .send()
        .await
        .context("POST /v1/chat/completions failed")?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response body")?;
    if !status.is_success() {
        anyhow::bail!("Server returned HTTP {status}: {text}");
    }

    let parsed: ChatCompletionResponse =
        serde_json::from_str(&text).context("Invalid JSON from server")?;
    Ok(parsed)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let base = Url::parse(&args.base_url).context("Invalid --base-url")?;
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

    let cfg_root = config_root_path(&args.config_root)?;
    let model_cfg_dir = ensure_model_config_folder(&cfg_root, &args.base_url, &model_id)?;

    let elma_cfg_path = model_cfg_dir.join("_elma.config");
    let gate_cfg_path = model_cfg_dir.join("gate.toml");
    let gate_why_cfg_path = model_cfg_dir.join("gate_why.toml");
    let intention_cfg_path = model_cfg_dir.join("intention.toml");
    let tooler_cfg_path = model_cfg_dir.join("tooler.toml");
    let planner_master_cfg_path = model_cfg_dir.join("planner_master.toml");
    let planner_cfg_path = model_cfg_dir.join("planner.toml");
    let decider_cfg_path = model_cfg_dir.join("decider.toml");
    let intention_tune_cfg_path = model_cfg_dir.join("intention_tune.toml");
    let action_type_cfg_path = model_cfg_dir.join("action_type.toml");
    let summarizer_cfg_path = model_cfg_dir.join("summarizer.toml");

    let mut elma_cfg = load_agent_config(&elma_cfg_path)?;
    let _gate_cfg = load_agent_config(&gate_cfg_path)?;
    let gate_why_cfg = load_agent_config(&gate_why_cfg_path)?;
    let intention_cfg = load_agent_config(&intention_cfg_path)?;
    let tooler_cfg = load_agent_config(&tooler_cfg_path)?;
    let planner_master_cfg = load_agent_config(&planner_master_cfg_path)?;
    let planner_cfg = load_agent_config(&planner_cfg_path)?;
    let decider_cfg = load_agent_config(&decider_cfg_path)?;
    let intention_tune_cfg = load_agent_config(&intention_tune_cfg_path)?;
    let mut action_type_cfg = load_agent_config(&action_type_cfg_path)?;
    let summarizer_cfg = load_agent_config(&summarizer_cfg_path)?;

    if args.tune {
        let model_ids = fetch_all_model_ids(&client, &base).await?;
        for mid in model_ids {
            let dir = ensure_model_config_folder(&cfg_root, &args.base_url, &mid)?;
            let tune_cfg = load_agent_config(&dir.join("intention_tune.toml"))?;
            tune_model(&args, &client, &chat_url, &dir, &mid, &tune_cfg).await?;
        }
        return Ok(());
    }

    // Ensure these configs track current base/model (user can still edit files manually).
    elma_cfg.base_url = args.base_url.clone();
    elma_cfg.model = model_id.clone();
    save_agent_config(&elma_cfg_path, &elma_cfg)?;

    // Router prompt upgrade: ensure CHAT exists as an allowed route.
    if !action_type_cfg.system_prompt.contains("Allowed routes:")
        || !action_type_cfg.system_prompt.contains("\nCHAT\n")
    {
        action_type_cfg.system_prompt = default_action_type_config(&args.base_url, &model_id).system_prompt;
        trace(&args, "upgraded=action_type.system_prompt");
        save_agent_config(&action_type_cfg_path, &action_type_cfg)?;
    }

    let ctx_max = fetch_ctx_max(&client, &base).await.unwrap_or(None);

    let sessions_root = sessions_root_path(&args.sessions_root)?;
    let session = ensure_session_layout(&sessions_root)?;

    // Workspace intel unit: gather real facts about where we are and inject them
    // into Elma's context so she doesn't hallucinate access constraints.
    let repo = repo_root()?;
    let ws = gather_workspace_context(&repo);
    if !ws.is_empty() {
        let p = session.root.join("workspace.txt");
        std::fs::write(&p, ws.trim().to_string() + "\n")
            .with_context(|| format!("write {}", p.display()))?;
        trace(&args, &format!("workspace_context_saved={}", p.display()));
    }

    let mut system_content = elma_cfg.system_prompt.clone();
    if !ws.trim().is_empty() {
        system_content.push_str("\n\nWORKSPACE CONTEXT (facts):\n");
        system_content.push_str(ws.trim());
    }
    let mut messages: Vec<ChatMessage> = vec![ChatMessage {
        role: "system".to_string(),
        content: system_content,
    }];

    eprintln!("Connected target: {chat_url}");
    eprintln!("Model: {model_id}");
    eprintln!("Config: {}", model_cfg_dir.display());
    eprintln!("Session: {}", session.root.display());
    eprintln!("Type /exit to quit, /reset to clear history.\n");
    // No explicit slash workflows for now; formulas should be orchestrated automatically.

    loop {
        let Some(line) = prompt_line("you> ")? else { break };
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

        // Always run freeform one-word intent tag (used for scenario helper + better gating).
        let classify_req = ChatCompletionRequest {
            model: intention_cfg.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: intention_cfg.system_prompt.clone(),
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: line.to_string(),
                },
            ],
            temperature: intention_cfg.temperature,
            top_p: intention_cfg.top_p,
            stream: false,
            max_tokens: intention_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(intention_cfg.repeat_penalty),
            reasoning_format: Some(intention_cfg.reasoning_format.clone()),
        };
        let classify = chat_once(&client, &chat_url, &classify_req)
            .await
            .ok()
            .and_then(|r| {
                r.choices
                    .get(0)
                    .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
            })
            .unwrap_or_default();
        let intent_tag = classify.trim().split_whitespace().next().unwrap_or("");

        // Scenario helper (best-effort) based on the freeform intent tag.
        let mapping = load_intention_mapping(&model_cfg_dir).unwrap_or_default();
        let (scenario, conf) = scenario_helper(intent_tag, &mapping);
        if conf >= 0.65 {
            if let Some(s) = scenario.as_ref() {
                trace(&args, &format!("scenario=\"{}\" conf={:.0}%", s, conf * 100.0));
            }
        } else {
            trace(&args, &format!("scenario=(none) conf={:.0}%", conf * 100.0));
        }

        // Route: single router decides CHAT vs action types.
        let router_system = format!(
            "{}\n\nContext:\nIntent tag: {intent_tag}\nBest scenario: {}\nScenario confidence: {:.0}%",
            action_type_cfg.system_prompt,
            scenario.clone().unwrap_or_else(|| "(none)".to_string()),
            conf * 100.0
        );
        let at_req = ChatCompletionRequest {
            model: action_type_cfg.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content: router_system,
                },
                ChatMessage {
                    role: "user".to_string(),
                    content: line.to_string(),
                },
            ],
            temperature: action_type_cfg.temperature,
            top_p: action_type_cfg.top_p,
            stream: false,
            max_tokens: action_type_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(action_type_cfg.repeat_penalty),
            reasoning_format: Some(action_type_cfg.reasoning_format.clone()),
        };
        let at_resp = chat_once(&client, &chat_url, &at_req).await?;
        let route_out = at_resp
            .choices
            .get(0)
            .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
            .unwrap_or_default();
        let route = route_out.trim().split_whitespace().next().unwrap_or("CHAT");
        trace(&args, &format!("route={route}"));

        if route.eq_ignore_ascii_case("CHAT") && conf >= 0.65 {
            // Ask the model why it picked CHAT (debug loop).
            let why_req = ChatCompletionRequest {
                model: gate_why_cfg.model.clone(),
                messages: vec![
                    ChatMessage {
                        role: "system".to_string(),
                        content: gate_why_cfg.system_prompt.clone(),
                    },
                    ChatMessage {
                        role: "user".to_string(),
                        content: format!(
                            "User message: {line}\nIntent tag: {intent_tag}\nBest scenario: {}\nConfidence: {:.0}%",
                            scenario.clone().unwrap_or_else(|| "(none)".to_string()),
                            conf * 100.0
                        ),
                    },
                ],
                temperature: gate_why_cfg.temperature,
                top_p: gate_why_cfg.top_p,
                stream: false,
                max_tokens: gate_why_cfg.max_tokens,
                n_probs: None,
                repeat_penalty: Some(gate_why_cfg.repeat_penalty),
                reasoning_format: Some(gate_why_cfg.reasoning_format.clone()),
            };
            if let Ok(why_resp) = chat_once(&client, &chat_url, &why_req).await {
                let why = why_resp
                    .choices
                    .get(0)
                    .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
                    .unwrap_or_default();
                if !why.trim().is_empty() {
                    let p = write_gate_why(&session.tune_dir, &why)?;
                    trace(&args, &format!("route_why=\"{}\" saved={}", why.trim(), p.display()));
                }
            }
        }

        if !route.eq_ignore_ascii_case("CHAT") {
            trace(&args, &format!("intent_tag={}", intent_tag));

            // Low scenario confidence should never block ACTION execution; we still proceed.
            if conf < 0.65 {
                trace(&args, "scenario_confidence=low (continuing)");
            }

            if route.eq_ignore_ascii_case("SHELL") {
                // If the user asked to "read X and summarize", do it as a 2-step formula:
                // 1) read the file content via shell
                // 2) summarize it via the summarizer intel unit
                let lower = line.to_ascii_lowercase();
                let wants_summary = lower.contains("summarize") || lower.contains("summary");
                let maybe_path = extract_first_path_from_user_text(line);
                if wants_summary {
                    if let Some(p) = maybe_path.clone() {
                        // Basic sanity: prevent injection by reusing the tooler+policy pipeline.
                        let read_cmd = format!("cat {}", shell_quote(&p));
                        if is_command_sane(&read_cmd) && is_command_allowed(&read_cmd) {
                            let path = write_shell_action(&session.shell_dir, &read_cmd)?;
                            trace(&args, &format!("shell_saved={}", path.display()));
                            let (code, output) = run_shell_one_liner(&read_cmd, &repo_root()? )?;
                            let out_path = write_shell_output(&session.shell_dir, &path, &output)?;
                            trace(&args, &format!("shell_output_saved={}", out_path.display()));
                            trace(&args, &format!("exec_exit_code={code}"));

                            // Summarize.
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
                                            "User request:\n{line}\n\nFile path:\n{p}\n\nFile contents:\n{}",
                                            output
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
                            let sum_resp = chat_once(&client, &chat_url, &sum_req).await?;
                            let sum_text = sum_resp
                                .choices
                                .get(0)
                                .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
                                .unwrap_or_default();
                            let sum_text = sum_text.trim().to_string();
                            println!(
                                "{}",
                                if args.no_color {
                                    format!("bot> {sum_text}")
                                } else {
                                    ansi_orange(&format!("bot> {sum_text}"))
                                }
                            );
                            println!();
                            // Store summary in chat history as assistant reply.
                            if !sum_text.trim().is_empty() {
                                messages.push(ChatMessage {
                                    role: "assistant".to_string(),
                                    content: sum_text.trim().to_string(),
                                });
                            }
                            continue;
                        }
                    }
                }

                let tool_req = ChatCompletionRequest {
                    model: tooler_cfg.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: tooler_cfg.system_prompt.clone(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: line.to_string(),
                        },
                    ],
                    temperature: tooler_cfg.temperature,
                    top_p: tooler_cfg.top_p,
                    stream: false,
                    max_tokens: tooler_cfg.max_tokens,
                    n_probs: None,
                    repeat_penalty: Some(tooler_cfg.repeat_penalty),
                    reasoning_format: Some(tooler_cfg.reasoning_format.clone()),
                };
                let tool_resp = chat_once(&client, &chat_url, &tool_req).await?;
                let tool_msg = tool_resp
                    .choices
                    .get(0)
                    .context("No choices[0] in tooler response")?
                    .message
                    .content
                    .as_deref()
                    .filter(|s| !s.trim().is_empty())
                    .or(tool_resp
                        .choices
                        .get(0)
                        .and_then(|c| c.message.reasoning_content.as_deref())
                        .filter(|s| !s.trim().is_empty()))
                    .unwrap_or("")
                    .trim()
                    .to_string();
                println!("tool> {tool_msg}\n");
                trace(&args, &format!("tooler_json={}", tool_msg.replace('\n', " ")));

                let cmd_line = serde_json::from_str::<serde_json::Value>(&tool_msg)
                    .ok()
                    .and_then(|v| v.get("cmd").and_then(|c| c.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| tool_msg.clone());

                let cmd_line = cmd_line.trim().to_string();
                let cmd_line = normalize_shell_cmd(&cmd_line);
                if !is_command_sane(&cmd_line) {
                    println!("elma> Tooler produced an invalid command; skipping execution.");
                    continue;
                }
                if !is_command_allowed(&cmd_line) {
                    println!("elma> Command blocked by local policy (no network/destructive).");
                    trace(&args, "exec=blocked");
                    continue;
                }

                let path = write_shell_action(&session.shell_dir, &cmd_line)?;
                trace(&args, &format!("shell_saved={}", path.display()));
                let (code, output) = run_shell_one_liner(&cmd_line, &repo_root()? )?;
                let out_path = write_shell_output(&session.shell_dir, &path, &output)?;
                trace(&args, &format!("shell_output_saved={}", out_path.display()));
                println!("elma> exit_code={code}\n{output}");
                trace(&args, &format!("exec_exit_code={code}"));
                continue;
            }

            if route.eq_ignore_ascii_case("PLAN") {
                // Route to /plan workflow using the goal as-is.
                let goal = line;
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
                let resp = chat_once(&client, &chat_url, &req).await?;
                let text = resp
                    .choices
                    .get(0)
                    .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
                    .unwrap_or_default();
                let plan_path = write_plan_file(&session.plans_dir, &(text.trim().to_string() + "\n"))?;
                append_master_link(&session.plans_dir, &plan_path, goal)?;
                println!("workflow> plan saved: {}\n", plan_path.display());
                continue;
            }

            if route.eq_ignore_ascii_case("MASTERPLAN") {
                let goal = line;
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
                let resp = chat_once(&client, &chat_url, &req).await?;
                let text = resp
                    .choices
                    .get(0)
                    .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
                    .unwrap_or_default();
                let p = session.plans_dir.join("_master.md");
                std::fs::write(&p, squash_blank_lines(text.trim()).trim().to_string() + "\n")
                    .with_context(|| format!("write {}", p.display()))?;
                println!("workflow> masterplan saved: {}\n", p.display());
                continue;
            }

            if route.eq_ignore_ascii_case("DECIDE") {
                let req = ChatCompletionRequest {
                    model: decider_cfg.model.clone(),
                    messages: vec![
                        ChatMessage {
                            role: "system".to_string(),
                            content: decider_cfg.system_prompt.clone(),
                        },
                        ChatMessage {
                            role: "user".to_string(),
                            content: line.to_string(),
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
                let resp = chat_once(&client, &chat_url, &req).await?;
                let word = resp
                    .choices
                    .get(0)
                    .and_then(|c| c.message.content.clone().or(c.message.reasoning_content.clone()))
                    .unwrap_or_default();
                let word = word.trim().split_whitespace().next().unwrap_or("").to_string();
                let path = write_decision(&session.decisions_dir, &word)?;
                println!("workflow> decision: {word} (saved: {})\n", path.display());
                continue;
            }
        }

        let req = ChatCompletionRequest {
            model: elma_cfg.model.clone(),
            messages: messages.clone(),
            temperature: elma_cfg.temperature,
            top_p: elma_cfg.top_p,
            stream: false,
            max_tokens: elma_cfg.max_tokens,
            n_probs: None,
            repeat_penalty: Some(elma_cfg.repeat_penalty),
            reasoning_format: Some(elma_cfg.reasoning_format.clone()),
        };

        let parsed = chat_once(&client, &chat_url, &req).await?;

        let msg = &parsed
            .choices
            .get(0)
            .context("No choices[0] in response")?
            .message;

        let final_text = msg.content.as_deref().unwrap_or("").trim();
        let thinking_text = msg.reasoning_content.as_deref().unwrap_or("").trim();

        let (tag_thinking, effective_final) =
            split_thinking_and_final(msg.content.as_deref(), msg.reasoning_content.as_deref());

        let paint_grey = |s: String| if args.no_color { s } else { ansi_grey(&s) };
        let paint_orange = |s: String| if args.no_color { s } else { ansi_orange(&s) };

        // Display "thinking" only when it is separable (structured or tagged). If the backend
        // provides only reasoning_content, we treat that as the visible output and don't
        // duplicate it as both think+final.
        let thinking_for_display = if !final_text.is_empty() && !thinking_text.is_empty() {
            Some(thinking_text.to_string())
        } else {
            tag_thinking
        };

        if args.show_thinking {
            if let Some(t) = thinking_for_display.as_deref().filter(|t| !t.trim().is_empty()) {
                println!("{}", paint_grey(format!("think> {t}")));
            }
        }

        let final_for_display = squash_blank_lines(effective_final.trim());
        let final_for_display = final_for_display.trim().to_string();
        if !final_for_display.is_empty() {
            println!("{}", paint_orange(format!("bot> {final_for_display}")));
        } else {
            println!("{}", paint_orange("bot> (empty response)".to_string()));
        }

        // Display only the ctx usage line (like llama.cpp WebUI "context used/total"),
        // formatted compactly in "k" units.
        if let (Some(ctx), Some(u)) = (ctx_max, parsed.usage.as_ref()) {
            if let Some(total) = u.total_tokens {
                let pct = (total as f64 / ctx as f64) * 100.0;
                let used_k = {
                    let k = ((total as f64) / 1000.0).round() as u64;
                    if total > 0 { k.max(1) } else { 0 }
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

        // Store assistant visible text in history (not the thinking).
        if !final_for_display.is_empty() {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: final_for_display.to_string(),
            });
        }
    }

    Ok(())
}
