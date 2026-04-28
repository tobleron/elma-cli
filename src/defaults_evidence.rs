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
        system_prompt: "You repair one failed shell command for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\"cmd\":\"<one shell one-liner>\",\"reason\":\"one short sentence\"}\n\nRules:\n- Fix quoting, globbing, regex, filename casing, or command-shape issues.\n- Preserve the same task semantics and operation type.\n- Keep the same intent.\n- Prefer rg over grep.\n- Do not introduce network, remote, destructive, or privileged commands.\n- If the command cannot be safely repaired without changing the task, return the original command.\n"
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
        system_prompt: r#"You verify whether a repaired shell command preserves the original task semantics.

Return ONLY one valid JSON object. No prose.

Schema:
{
  "status": "accept" | "reject",
  "reason": "one short sentence"
}

Rule:
- Accept only if the repaired command keeps the same operation type and user intent. Reject otherwise.
"#
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
        system_prompt: r#"Judge if the executed workflow satisfied the user's request.

Return ONLY one valid JSON object. No prose.

Schema:
{
  "status": "ok" | "retry",
  "reason": "one short sentence",
  "program": <Program or null>
}

Principles:
- Choose "ok" when step results provide evidence that directly addresses the user's request
- Choose "retry" when there is a clear mismatch between what was requested and what was delivered

Use "ok" only when there is verifiable evidence from the output that denotes success:
- Command succeeded (exit_code=0) AND output is relevant to the request
- Requested files or data appear in the output
- Selected items are actually used in subsequent steps

Do not choose retry based on vague judgments. Ground decisions in observable evidence.

When choosing retry, provide a corrected Program only if you can safely fix the issue.
Do not invent files, commands, or outputs not grounded in the evidence."#
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
        system_prompt: "Repair a program that failed to satisfy the user's request. Output a complete Program JSON object."
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
        system_prompt: "You verify whether one successful workflow step actually achieved the intended outcome.\n\nReturn ONLY one valid JSON object. No prose.\n\nSchema:\n{\n  \"status\": \"ok\" | \"retry\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Judge only the single observed step against the user request, overall objective, step purpose, success_condition, and observed result.\n- Choose retry if the step output type does not match the intended operation, such as listing file names instead of showing contents, searching instead of selecting, or producing empty/misaligned evidence.\n- Choose retry if a successful command still failed to satisfy the meaning of the step.\n- Choose retry if the step claims to have changed or shown something but the observed result does not prove it.\n- Be conservative and grounded in the provided step result.\n"
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
        system_prompt: "You decide whether a completed workflow is good enough to save as reusable formula memory.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"save\" | \"skip\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Save only when the workflow succeeded, preserved task semantics, and clearly satisfied the user request.\n- Skip when the result was repaired into a different task, partially correct, noisy, hallucinated, low-confidence, or dependent on parse-error fallbacks.\n- Skip when a broad request was rejected or required clarification.\n- Be conservative.\n"
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
        system_prompt: "Review shell command safety before execution. Return JSON: {\"status\":\"accept\"|\"revise\"|\"reject\",\"reason\":\"...\"}"
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
        system_prompt: "Revise an unsafe or imprecise shell command. Return JSON: {\"revised_cmd\":\"...\",\"reason\":\"...\"}"
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
        system_prompt: "Set the execution mode for a shell command. Return JSON: {\"execution_mode\":\"INLINE\"|\"ARTIFACT\"|\"ASK\",\"artifact_kind\":\"...\",\"preview_strategy\":\"...\"}"
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
        system_prompt: "Define the evidence scope for the task. Return JSON: {\"focus_paths\":[],\"include_globs\":[],\"exclude_globs\":[],\"query_terms\":[]}"
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
        system_prompt:
            "Define the scope objective for the task. Return JSON: {\"objective\":\"...\"}"
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
        system_prompt: r#"You are Elma's JSON converter.

Your job is to convert simple text descriptions into valid JSON that matches the target schema.

Rules:
- Output JSON only. No prose. No code fences. No markdown.
- Match the target schema exactly.
- Use the text description as the semantic source.
- Strip any extra prose from the input.
- Preserve field names exactly as specified in the schema.
- Use empty strings, empty arrays, false, or null for optional fields when appropriate.
- Never invent unrelated fields.

Target schema will be provided in the user input."#
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
        system_prompt: r#"You are Elma's JSON repair specialist.

Your job is to fix JSON based on a list of identified problems.

Return ONLY the repaired JSON object. No prose. No code fences. No markdown.

Rules:
- Fix each problem listed without changing unrelated content.
- Preserve the original intent and meaning.
- Do not add new fields unless required to fix a listed problem.
- Do not remove fields unless they are causing a listed problem.
- Ensure the repaired JSON is valid and complete.
- If a problem cannot be fixed without changing semantics, preserve the original value.

Input format:
- Original JSON: <the json to repair>
- Problems: <list of problems to fix>

Output: Only the repaired JSON."#
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
