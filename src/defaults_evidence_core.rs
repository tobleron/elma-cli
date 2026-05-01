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
        system_prompt: "You are Elma's evidence mode classifier.\nReturn the most probable answer as a single DSL line.\n\nChoice rules:\n1 = RAW: the user needs exact raw output\n2 = COMPACT: the user needs concise summarized evidence\n3 = RAW_PLUS_COMPACT: the user benefits from both exact output and concise explanation\n\nOutput format:\nMODE choice=1 label=RAW reason=\"ultra concise justification\" entropy=0.1\n\nRules:\n- Output exactly one MODE line.\n- Choose RAW only when exact output matters.\n- Choose COMPACT when summary is sufficient or raw output would be noisy.\n- Choose RAW_PLUS_COMPACT when exact evidence matters but interpretation also helps.\n- No JSON, Markdown fences, or prose outside the DSL.\n"
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
        system_prompt: "You compact raw workspace evidence for Elma.\n\nReturn a single DSL RESULT line.\n\nOutput format:\nRESULT summary=\"plain text summary\" key_facts=\"fact1,fact2\" noise=\"noise1\"\n\nRules:\n- Preserve only facts that help solve the user's task.\n- Prefer exact paths, signatures, versions, and short facts.\n- Omit repetitive listings and irrelevant build artifacts.\n- For key_facts and noise, use comma-separated strings.\n"
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
        system_prompt: "You classify workspace artifacts for Elma.\n\nReturn exactly one DSL line and nothing else:\nARTIFACT safe=\"a,b\" maybe=\"c\" keep=\"d\" ignore=\"e\" reason=\"one short sentence\"\n\nRules:\n- Use comma-separated short phrases for the list fields.\n- Be conservative.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
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
        system_prompt: "Verify the answer is supported by evidence.\n\nReturn exactly one DSL line and nothing else:\nVERDICT status=ok reason=\"one short sentence\" unsupported_claims=\"claim1,claim2\"\n\nAllowed status:\n- ok\n- revise\n\nRules:\n- unsupported_claims is a comma-separated list (may be empty).\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
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
        system_prompt: "Provide revision guidance for unsupported claims.\n\nReturn exactly one DSL line and nothing else:\nADVICE missing_points=\"point1,point2\" rewrite_instructions=\"one short paragraph\"\n\nRules:\n- missing_points is a comma-separated list.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
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
        system_prompt: "You are Elma's legacy verify checker.\n\nThis profile is deprecated by the compact DSL migration.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"verify_checker disabled\"\n"
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

Rewrite the user's latest request into one short intent sentence that describes what the user is asking.

Rules:
- Output plain text only.
- Keep it to one sentence.
- Use descriptive framing: "The user is <describe user's intention>"
- Preserve the user's objective without adding new work.
- Do not answer the user's question.
- Do not invent facts, configuration values, URLs, tool names, file contents, or outcomes.
- Use only information explicitly present in the user's message or conversation history.
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
    // Build narrative input with conversation history (exclude system messages).
    // Deduplicate: exclude the last user message if it matches the current
    // message to prevent phantom duplicated entries.
    let mut input = String::new();
    input.push_str("CONVERSATION HISTORY:\n");
    let mut last_user_msg: Option<&str> = None;
    for msg in conversation_history {
        // Skip system messages
        if msg.role == "system" {
            continue;
        }
        let role = if msg.role == "user" {
            last_user_msg = Some(&msg.content);
            "User"
        } else {
            "Elma"
        };
        // Truncate long messages to avoid token explosion
        let content = if msg.content.chars().count() > 200 {
            format!("{}...", msg.content.chars().take(200).collect::<String>())
        } else {
            msg.content.clone()
        };
        input.push_str(&format!("{}: {}\n", role, content));
    }
    // Skip appending current message if it appears identical to last history entry.
    let user_trimmed = user_message.trim();
    if !user_trimmed.is_empty() {
        let is_duplicate = last_user_msg
            .map(|last| last.trim() == user_trimmed)
            .unwrap_or(false);
        if !is_duplicate {
            input.push_str(&format!("\nUser: {}\n", user_message));
        } else {
            input.push_str("\n");
        }
    }

    let req = chat_request_system_user(
        cfg,
        &cfg.system_prompt,
        &input,
        ChatRequestOptions::default(),
    );

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
