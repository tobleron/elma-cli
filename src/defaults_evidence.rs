//! @efficiency-role: infra-config
//!
//! Defaults - Evidence and Tune Configurations

use crate::*;

// Re-export all core functions and types from defaults_evidence_core
pub(crate) use defaults_evidence_core::*;

pub(crate) fn default_command_repair_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "You are Elma's command repair specialist.\n\nReturn exactly one DSL line and nothing else.\n\nFormat:\nREPAIR cmd=\"<one shell one-liner>\" reason=\"one short sentence\"\n\nPrinciples:\n- Preserve the same task semantics and operation type.\n- Fix quoting, globbing, regex, filename casing, or command-shape issues.\n- Prefer rg over grep.\n- Do not introduce network, remote, destructive, or privileged commands.\n- If safe repair is not possible without changing the task, return the original command.\n"
            .to_string(),
    }
}

pub(crate) fn default_task_semantics_guard_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "task_semantics_guard".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You verify whether a repaired shell command preserves the original task semantics.\n\nReturn exactly one DSL line and nothing else:\nSEMANTICS status=accept reason=\"one short sentence\"\n\nAllowed status:\n- accept\n- reject\n\nRule:\n- Accept only if the repaired command keeps the same operation type and user intent. Reject otherwise.\n"
            .to_string(),
    }
}

pub(crate) fn default_execution_sufficiency_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "execution_sufficiency".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "Judge if the executed workflow satisfied the user's request.\n\nReturn exactly one DSL line and nothing else:\nVERDICT status=ok reason=\"one short sentence\"\n\nRules:\n- status: ok | retry\n- Ground the decision in observed evidence.\n- Do not emit a corrected program here.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_execution_program_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "execution_program_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "This profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"execution_program_repair disabled\"\n"
            .to_string(),
    }
}

pub(crate) fn default_outcome_verifier_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "outcome_verifier".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 384,
        timeout_s: 120,
        system_prompt: "You verify whether one successful workflow step actually achieved the intended outcome.\n\nReturn exactly one DSL line and nothing else:\nVERDICT status=ok reason=\"one short sentence\"\n\nRules:\n- status: ok | retry\n- Judge only the single observed step against the user request, objective, purpose, success_condition, and observed result.\n- Be conservative and evidence-grounded.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_memory_gate_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "memory_gate".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You decide whether a completed workflow is good enough to save as reusable formula memory.\n\nReturn exactly one DSL line and nothing else:\nGATE status=save reason=\"one short sentence\"\n\nAllowed status:\n- save\n- skip\n\nRules:\n- Save only when the workflow clearly succeeded and preserved task semantics.\n- Skip partial, noisy, or low-confidence outcomes.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_command_preflight_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "command_preflight".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 384,
        timeout_s: 120,
        system_prompt: "Pre-flight check for shell commands.\n\nReturn exactly one DSL line and nothing else:\nPREFLIGHT status=accept reason=\"one short sentence\" cmd=\"one shell one-liner\" question=\"\" execution_mode=INLINE artifact_kind=\"shell_output\" preview_strategy=\"\"\n\nAllowed status:\n- accept\n- revise\n- reject\n\nRules:\n- If you revise, set cmd to the revised command.\n- If you need clarification, set status=reject and put the question in question.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_command_reviser_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "command_reviser".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "Revise an unsafe or imprecise shell command.\n\nReturn exactly one DSL line and nothing else:\nREVISE revised_cmd=\"one shell one-liner\" reason=\"one short sentence\"\n\nRules:\n- Preserve task intent.\n- Prefer safe read-only commands when possible.\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_execution_mode_setter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "execution_mode_setter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Set the execution mode for a shell command.\n\nReturn exactly one DSL line and nothing else:\nMODE execution_mode=INLINE artifact_kind=\"shell_output\" preview_strategy=\"\"\n\nAllowed execution_mode:\n- INLINE\n- ARTIFACT\n- ASK\n\nRules:\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_scope_builder_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "Define the evidence scope for the task.\n\nReturn exactly one DSL block and nothing else:\nSCOPE objective=\"one sentence\"\nF path=\"relative/path\"\nIG glob=\"glob/**\"\nEG glob=\"glob/**\"\nQ text=\"query\"\nA artifact=\"artifact\"\nEND\n\nRules:\n- Keep lists short.\n- No JSON, Markdown fences, or prose outside the DSL.\n"
            .to_string(),
    }
}

pub(crate) fn default_scope_objective_builder_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "scope_objective_builder".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Define the scope objective for the task.\n\nReturn exactly one DSL line and nothing else:\nOBJECTIVE objective=\"one sentence\"\n\nRules:\n- No JSON, Markdown fences, or prose outside the DSL line.\n"
            .to_string(),
    }
}

pub(crate) fn default_result_presenter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "result_presenter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "Present the final answer to the user in plain terminal text.".to_string(),
    }
}

pub(crate) fn default_text_generator_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "text_generator".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.2,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: r#"You are Elma's text generator.

Your job is to convert reasoning into simple, clear text that describes what needs to be done.

Rules:
- Output simple text only. No JSON. No code fences.
- Be concise and specific.
- Describe the action, purpose, and expected outcome.
- Do not include technical details or implementation specifics.
- Focus on WHAT needs to be done, not HOW.

Example output:
"List all pending task files in the _tasks/pending/ directory and summarize their objectives.""#
            .to_string(),
    }
}

pub(crate) fn default_json_converter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "json_converter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You are Elma's legacy JSON converter.\n\nThis profile is deprecated by the compact DSL migration.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"json_converter disabled\"\n"
            .to_string(),
    }
}

pub(crate) fn default_json_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "json_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1024,
        timeout_s: 120,
        system_prompt: "You are Elma's legacy JSON repair specialist.\n\nThis profile is deprecated by the compact DSL migration.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"json_repair disabled\"\n"
            .to_string(),
    }
}

// ============================================================================
// Expert Responder - Response Posture Advice
// ============================================================================

pub(crate) fn default_expert_advisor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "expert_advisor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: canonical_system_prompt("expert_advisor")
            .unwrap_or("Determine the best response posture for the user's situation.")
            .to_string(),
    }
}

/// Expert Responder: produce compact advice on how Elma should respond
pub(crate) async fn expert_advisor_advice(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    response_narrative: &str,
) -> Result<String> {
    let req = chat_request_system_user(
        cfg,
        &cfg.system_prompt,
        response_narrative,
        ChatRequestOptions::default(),
    );

    let resp = chat_once(client, chat_url, &req).await?;
    Ok(extract_response_text(&resp).trim().to_string())
}

pub(crate) fn parse_expert_advisor_style(helper_response: &str) -> &str {
    let response_upper = helper_response.to_uppercase();
    if response_upper.contains("CAUTIOUS") {
        "cautious"
    } else if response_upper.contains("EXPLANATORY") {
        "explanatory"
    } else {
        "direct"
    }
}
