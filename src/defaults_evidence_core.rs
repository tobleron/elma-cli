//! @efficiency-role: infra-config
//!
//! Defaults - Core Functions and Profile Management

use crate::*;

pub(crate) fn default_evidence_mode_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "evidence_mode".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "You decide how Elma should present shell evidence.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"mode\": \"RAW\" | \"COMPACT\" | \"RAW_PLUS_COMPACT\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- RAW: use when the user explicitly asks to run/execute/show a command (e.g., \"run tree\", \"run cargo test\", \"show files\"). Also use when the command output is short (<50 lines) and the user wants to see exact output.\n- COMPACT: use when the user wants explanation, summary, analysis, comparison, or when raw output would be very noisy (>100 lines). Also use for pure chat/conversational turns with no actual command execution.\n- RAW_PLUS_COMPACT: use when exact output matters but a short explanation also helps. Use when step has artifact_path. Use when user asks for both output AND summary.\n\nCRITICAL RULE FOR COMMAND EXECUTION:\n- If user message contain \"run <command>\", \"execute\", \"show output\", or names a specific command (tree, cargo, ls, git, etc.), prefer RAW or RAW_PLUS_COMPACT.\n- If step_results show a command was actually executed (command field is not null), prefer RAW or RAW_PLUS_COMPACT unless output is extremely long.\n- If step_results show only a reply step with no command execution, use COMPACT.\n\nDecision priority:\n1. User explicitly asks for raw output → RAW\n2. User asks for command execution → RAW or RAW_PLUS_COMPACT\n3. Command was executed with short output → RAW\n4. Command was executed with long output → RAW_PLUS_COMPACT\n5. User wants summary/analysis → COMPACT\n6. No command executed (reply only) → COMPACT\n\nBe strict and concise.\n"
            .to_string(),
    }
}

pub(crate) fn default_evidence_compactor_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_artifact_classifier_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_claim_checker_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "claim_checker".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "Verify the answer is supported by evidence. Return JSON: {\"status\":\"ok\"|\"revise\",\"reason\":\"...\",\"unsupported_claims\":[]}"
            .to_string(),
    }
}

pub(crate) fn default_claim_revision_advisor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "claim_revision_advisor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "Provide revision guidance for unsupported claims. Return JSON: {\"missing_points\":[],\"rewrite_instructions\":\"...\"}"
            .to_string(),
    }
}

pub(crate) fn default_verify_checker_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "verify_checker".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: r#"You are Elma's JSON verify checker.

Your job is to check if JSON output is well-formed and identify any problems.

Return ONLY one valid JSON object. No prose.

Schema:
{
  "status": "ok" | "problems",
  "problems": ["list of specific problems found, or empty array if ok"]
}

Rules:
- Check for missing required fields.
- Check for invalid field types.
- Check for empty required strings.
- Check for invalid enum values.
- Check for structural issues (wrong nesting, missing brackets, etc.).
- List each problem specifically and clearly.
- If no problems, return status "ok" with empty problems array.

Example output with problems:
{"status":"problems","problems":["Missing required field 'status'","Field 'reason' is empty"]}

Example output without problems:
{"status":"ok","problems":[]}"#
            .to_string(),
    }
}

pub(crate) fn default_intention_tune_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn managed_profile_specs(base_url: &str, model: &str) -> Vec<(String, Profile)> {
    // Load all seed profiles from config/defaults/.
    // Canonical system prompts for managed intel units are enforced later
    // by startup sync through prompt_constants.rs.
    let defaults_dir = std::path::PathBuf::from("config/defaults");
    let mut specs = Vec::new();
    let excluded_files = ["model_behavior.toml"];

    for entry in std::fs::read_dir(&defaults_dir).ok().into_iter().flatten() {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "toml") {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if excluded_files.contains(&filename) {
                        continue;
                    }
                    if let Ok(mut profile) = load_agent_config(&path) {
                        if !base_url.is_empty() {
                            profile.base_url = base_url.to_string();
                        }
                        if !model.is_empty() {
                            profile.model = model.to_string();
                        }
                        apply_canonical_system_prompt(&mut profile);
                        specs.push((filename.to_string(), profile));
                    }
                }
            }
        }
    }

    specs
}

pub(crate) fn managed_profile_file_names() -> Vec<String> {
    managed_profile_specs("", "")
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

// ============================================================================
// JSON Pipeline Intel Units (Task 008 Phase 3)
// ============================================================================

/// Generate simple text from reasoning
pub(crate) async fn generate_text_from_reasoning(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    reasoning: &str,
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
                content: format!(
                    "Convert this reasoning into simple action text:\n\n{}",
                    reasoning
                ),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };

    let resp = chat_once_with_timeout(client, chat_url, &req, cfg.timeout_s).await?;
    Ok(extract_response_text(&resp).trim().to_string())
}

/// Convert text to JSON using schema
pub(crate) async fn convert_text_to_json(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    text: &str,
    schema_description: &str,
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
                content: format!(
                    "Convert this text to JSON matching the schema:\n\nSchema:\n{}\n\nText:\n{}",
                    schema_description, text
                ),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };

    let resp = chat_once(client, chat_url, &req).await?;
    Ok(extract_response_text(&resp).trim().to_string())
}

/// Verify JSON and list problems
pub(crate) async fn verify_json(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    json: &str,
) -> Result<VerifyCheckResult> {
    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!("Verify this JSON:\n\n{}", json),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };

    chat_json_with_repair(client, chat_url, &req).await
}

/// Repair JSON based on problems
pub(crate) async fn repair_json(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    json: &str,
    problems: &[String],
) -> Result<String> {
    let problems_text = if problems.is_empty() {
        "No problems found".to_string()
    } else {
        problems
            .iter()
            .map(|p| format!("- {}", p))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: format!(
                    "Original JSON:\n{}\n\nProblems to fix:\n{}",
                    json, problems_text
                ),
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };

    let resp = chat_once(client, chat_url, &req).await?;
    Ok(extract_response_text(&resp).trim().to_string())
}

/// Result of JSON verification check
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct VerifyCheckResult {
    pub status: String, // "ok" or "problems"
    pub problems: Vec<String>,
}

// ============================================================================
// Intent Helper - Annotate User Intent (Task 048)
// ============================================================================

pub(crate) fn default_intent_helper_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "intent_helper".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: r#"You are Elma's intent helper.

Rewrite the user's latest request into one short intent sentence that clarifies what they want Elma to accomplish.

Rules:
- Output plain text only.
- Keep it to one sentence.
- Preserve the user's objective without adding new work.
- Do not answer the user's question.
- Do not invent facts, configuration values, URLs, tool names, file contents, or outcomes.
- Use only information explicitly present in the user's message or conversation history.
- If the user asks for facts Elma must provide later, describe that they want those facts instead of stating them.
"#
        .to_string(),
    }
}

/// Annotate user intention with helper annotation, considering conversation context
pub(crate) async fn annotate_user_intent(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
    conversation_history: &[ChatMessage],
) -> Result<String> {
    // Build narrative input with conversation history (exclude system messages)
    let mut input = String::new();
    input.push_str("CONVERSATION HISTORY:\n");
    for msg in conversation_history {
        // Skip system messages
        if msg.role == "system" {
            continue;
        }
        let role = if msg.role == "user" { "User" } else { "Elma" };
        // Truncate long messages to avoid token explosion
        let content = if msg.content.len() > 200 {
            format!("{}...", &msg.content[..200])
        } else {
            msg.content.clone()
        };
        input.push_str(&format!("{}: {}\n", role, content));
    }
    input.push_str(&format!("\nUser: {}\n", user_message));

    let req = ChatCompletionRequest {
        model: cfg.model.clone(),
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: cfg.system_prompt.clone(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: input,
            },
        ],
        temperature: cfg.temperature,
        top_p: cfg.top_p,
        stream: false,
        max_tokens: cfg.max_tokens,
        n_probs: None,
        repeat_penalty: Some(cfg.repeat_penalty),
        reasoning_format: Some(cfg.reasoning_format.clone()),
        grammar: None,
    };

    let resp = chat_once(client, chat_url, &req).await?;
    Ok(extract_response_text(&resp).trim().to_string())
}

pub(crate) fn get_retry_prompt_variant(attempt: u32) -> &'static str {
    match attempt {
        0 => "standard",
        1 => "step-by-step",
        2 => "challenge",
        _ => "simplify",
    }
}
