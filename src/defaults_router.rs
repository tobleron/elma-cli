//! @efficiency-role: data-model
//!
//! Defaults - Router, Planner, and Judge Configurations

use crate::*;

pub(crate) fn default_router_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "router".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1,
        timeout_s: 120,
        system_prompt: "You are Elma's workflow gate estimator.\n\nReturn exactly one digit and nothing else.\n\nMapping:\n1 = CHAT\n2 = WORKFLOW\n\nInterpretation:\n- 1 CHAT: answer directly without an internal workflow.\n- 2 WORKFLOW: use internal reasoning steps, workspace evidence, or another intel unit before the final answer.\n\nImportant distinctions:\n- Greetings or general knowledge questions are usually 1.\n- Questions about the current project, files, code, commands, or tasks that need planning or decisions are usually 2.\n\nRules:\n- Output must be exactly one digit from 1 to 2.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents whether Elma should enter workflow mode.\n".to_string(),
    }
}

pub(crate) fn default_mode_router_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "mode_router".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1,
        timeout_s: 120,
        system_prompt: "You are Elma's workflow mode estimator.\n\nReturn exactly one digit and nothing else.\n\nMapping:\n1 = INSPECT\n2 = EXECUTE\n3 = PLAN\n4 = MASTERPLAN\n5 = DECIDE\n\nInterpretation:\n- 1 INSPECT: inspect workspace evidence, files, code, or configuration.\n- 2 EXECUTE: run commands or carry out direct terminal actions.\n- 3 PLAN: create one concrete step-by-step plan.\n- 4 MASTERPLAN: create a higher-level overall plan across phases.\n- 5 DECIDE: return a concise decision or label.\n\nImportant distinctions:\n- \"What is my current project about?\", \"read Cargo.toml and summarize it\", and \"find where fetch_ctx_max is defined\" are usually 1.\n- \"list files\", \"run tests\", and \"build the project\" are usually 2.\n- \"Create a step-by-step plan\" is 3, not 4.\n- Only choose 4 when the user truly wants an overall master plan.\n\nRules:\n- Output must be exactly one digit from 1 to 5.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents the workflow mode.\n".to_string(),
    }
}

pub(crate) fn default_speech_act_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "speech_act".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 1,
        timeout_s: 120,
        system_prompt: "Your job is to determine user intention and classify to either general chat, inquiry, or instruction."
            .to_string(),
    }
}

pub(crate) fn default_action_type_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "action_type".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 16,
        timeout_s: 120,
        system_prompt: "Classify the user's request into exactly ONE WORD route.\n\nAllowed routes:\nCHAT\nSHELL\nPLAN\nMASTERPLAN\nDECIDE\n\nGuidance:\n- CHAT: greetings, smalltalk, questions that do not require terminal/workspace changes.\n- SHELL: any request to run a terminal command (list files, search, build, test, run scripts, inspect files).\n- PLAN: user asks for a step-by-step plan.\n- MASTERPLAN: user asks for an overall master plan for a multi-step objective.\n- DECIDE: user asks for a single-word decision/label.\n\nRules:\n- Output must be exactly one word from the allowed routes.\n- No punctuation.\n- No explanation.\n"
            .to_string(),
    }
}

pub(crate) fn default_planner_master_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "Create a master plan in Markdown with checkbox steps."
            .to_string(),
    }
}

pub(crate) fn default_planner_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "Create a detailed step-by-step plan in Markdown with numbered checkbox actions."
            .to_string(),
    }
}

pub(crate) fn default_decider_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_selector_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "selector".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You select structured items for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"items\": [\"...\", \"...\"],\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Return exact items only. No prose outside the JSON object.\n- When selecting file paths, return exact relative paths that can be used in later shell commands.\n- Preserve the requested order when ranking or prioritization matters.\n- When the instructions ask for an exact count such as top 3, return exactly that many items when the evidence supports it.\n- For project-file ranking, prefer files that define project identity, entry points, or primary configuration before secondary helpers.\n- Every returned item must appear verbatim in the provided evidence. Do not invent unseen files or paths.\n- If the evidence is insufficient, return an empty items list and explain why in reason.\n- Be precise and conservative.\n"
            .to_string(),
    }
}

pub(crate) fn default_summarizer_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_formatter_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_json_outputter_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "json_outputter".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "You are Elma's JSON outputter.\n\nYour only job is to return EXACTLY one valid JSON object that matches the target schema described in the provided task instructions.\n\nRules:\n- Output JSON only.\n- No prose.\n- No code fences.\n- No markdown.\n- No explanations.\n- Use the provided target system prompt and target user input as the schema contract.\n- Use the raw model draft as the semantic source.\n- If the raw draft contains extra prose, strip it and keep only the schema-valid content.\n- If a parser error is provided, fix the JSON to satisfy that parser error without changing the intended meaning.\n- Preserve field names exactly.\n- Preserve required enums exactly.\n- If the draft omits optional fields, use empty strings, empty arrays, false, or null only when that fits the schema.\n- Never invent unrelated fields.\n"
            .to_string(),
    }
}

pub(crate) fn default_final_answer_extractor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "final_answer_extractor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 160,
        timeout_s: 120,
        system_prompt: "You are Elma's final answer extractor.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"final\": \"plain text final answer\"\n}\n\nRules:\n- Remove all reasoning, scratchpad text, and internal analysis.\n- Preserve the intended answer faithfully.\n- Use the original system prompt and original user input as the instruction contract.\n- Use the assistant draft and separated reasoning as the semantic source.\n- If the draft has no final answer but the reasoning clearly implies one, produce the shortest faithful final answer.\n- Do not broaden the answer beyond what the original user asked.\n- Do not add workspace background, architecture details, or extra explanations unless the original request explicitly asked for them.\n- Prefer the shortest direct answer that fully satisfies the request.\n- Output plain terminal text inside the final field.\n- No markdown unless the original instruction explicitly asked for it.\n- No prose outside the JSON object.\n"
            .to_string(),
    }
}

pub(crate) fn default_calibration_judge_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "calibration_judge".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You evaluate whether Elma's final answer satisfied a calibration scenario.\n\nReturn ONLY one valid JSON object. No prose. No code fences.\n\nSchema:\n{\n  \"status\": \"pass\" | \"fail\",\n  \"reason\": \"one short sentence\",\n  \"answered_request\": true | false,\n  \"faithful_to_evidence\": true | false,\n  \"plain_text\": true | false\n}\n\nRules:\n- Pass only when the answer clearly addresses the user's final request.\n- faithful_to_evidence must be true only if the answer stays within the provided evidence or clearly marks uncertainty.\n- plain_text must be false if the answer uses Markdown and the user did not ask for Markdown.\n- Be strict.\n"
            .to_string(),
    }
}

pub(crate) fn default_complexity_assessor_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "Assess task complexity. Return JSON: {\"complexity\":\"DIRECT\"|\"INVESTIGATE\"|\"MULTISTEP\"|\"OPEN_ENDED\",\"risk\":\"LOW\"|\"MEDIUM\"|\"HIGH\"}"
            .to_string(),
    }
}

pub(crate) fn default_evidence_need_assessor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "evidence_need_assessor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Assess if the task needs workspace evidence. Return JSON: {\"needs_evidence\":bool,\"needs_tools\":bool}"
            .to_string(),
    }
}

pub(crate) fn default_action_need_assessor_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "action_need_assessor".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Assess if the task needs decision or planning. Return JSON: {\"needs_decision\":bool,\"needs_plan\":bool}"
            .to_string(),
    }
}

pub(crate) fn default_pattern_suggester_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "pattern_suggester".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Suggest the reasoning pattern for this task. Return JSON: {\"suggested_pattern\":\"reply_only\"|\"inspect_reply\"|\"inspect_summarize_reply\"|\"inspect_decide_reply\"|\"inspect_edit_verify_reply\"|\"execute_reply\"|\"plan_reply\"|\"masterplan_reply\"}"
            .to_string(),
    }
}

pub(crate) fn default_formula_selector_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "Select the best reasoning formula for this task. Return JSON: {\"primary\":\"formula_name\",\"alternatives\":[],\"reason\":\"...\"}"
            .to_string(),
    }
}

pub(crate) fn default_formula_memory_matcher_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "formula_memory_matcher".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Match the task to a formula memory by signature. Return JSON: {\"memory_id\":\"...\"|\"\"}"
            .to_string(),
    }
}

pub(crate) fn default_workflow_planner_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "workflow_planner".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 768,
        timeout_s: 120,
        system_prompt: "Plan the workflow scope and evidence needs. Return JSON: {\"objective\":\"...\",\"scope\":{\"focus_paths\":[],\"include_globs\":[],\"exclude_globs\":[],\"query_terms\":[],\"expected_artifacts\":[]}}"
            .to_string(),
    }
}

pub(crate) fn default_workflow_complexity_planner_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "workflow_complexity_planner".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Plan workflow complexity and risk. Return JSON: {\"complexity\":\"DIRECT\"|\"INVESTIGATE\"|\"MULTISTEP\"|\"OPEN_ENDED\",\"risk\":\"LOW\"|\"MEDIUM\"|\"HIGH\"}"
            .to_string(),
    }
}

pub(crate) fn default_workflow_reason_planner_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "workflow_reason_planner".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: "Explain the workflow planning decision. Return JSON: {\"reason\":\"one short sentence\"}"
            .to_string(),
    }
}
