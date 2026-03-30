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
        system_prompt: "You are Elma's speech-act estimator.\n\nReturn exactly one digit and nothing else.\n\nMapping:\n1 = CAPABILITY_CHECK\n2 = INFO_REQUEST\n3 = ACTION_REQUEST\n\nInterpretation:\n- 1 CAPABILITY_CHECK: the user is asking whether Elma can do something, not asking Elma to do it now.\n- 2 INFO_REQUEST: the user wants information or an answer; a workflow may still be needed to inspect evidence.\n- 3 ACTION_REQUEST: the user wants Elma to actually do something now, including indirect polite requests.\n\nImportant distinctions:\n- \"Are you able to list files here?\" is usually 1.\n- \"What is my current project about?\" is usually 2.\n- \"Can you list files?\" and \"Could you run the tests?\" are usually 3 in normal English, because they are indirect requests.\n\nRules:\n- Output must be exactly one digit from 1 to 3.\n- No punctuation.\n- No explanation.\n- Choose the digit that best represents the user's speech act.\n".to_string(),
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
        system_prompt: "You create and maintain a master execution plan.\n\nOutput Markdown only.\nUse checkboxes like:\n- [ ] step\nKeep it concise and actionable.\nDo not include any analysis."
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
        system_prompt: "You create a detailed plan for the user's request.\n\nOutput Markdown only.\nUse a title, then a checklist of numbered actions, each as a checkbox.\nExample:\n# Plan\n- [ ] 1. Do X\n- [ ] 2. Do Y\nDo not include analysis."
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
        system_prompt: "You assess task complexity for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"complexity\": \"DIRECT\" | \"INVESTIGATE\" | \"MULTISTEP\" | \"OPEN_ENDED\",\n  \"needs_evidence\": true | false,\n  \"needs_tools\": true | false,\n  \"needs_decision\": true | false,\n  \"needs_plan\": true | false,\n  \"risk\": \"LOW\" | \"MEDIUM\" | \"HIGH\",\n  \"suggested_pattern\": \"reply\" | \"inspect_reply\" | \"inspect_summarize_reply\" | \"inspect_decide_reply\" | \"inspect_edit_verify_reply\" | \"execute_reply\" | \"plan_reply\" | \"masterplan_reply\"\n}\n\nRules:\n- Cleanup, safety review, comparison, and 'what is safe to remove' tasks are usually MULTISTEP with suggested_pattern inspect_decide_reply.\n- Ranking, prioritization, selection, and \"top N / most important / best\" requests about workspace items are usually MULTISTEP with needs_evidence=true, needs_decision=true, and suggested_pattern inspect_decide_reply.\n- If the user wants the chosen items shown afterward, the task still usually needs inspection plus decision before any final display step.\n- Editing, file creation, patching, rewriting, or update requests are usually MULTISTEP with suggested_pattern inspect_edit_verify_reply.\n- Questions about the current project, code, files, or configuration usually need evidence.\n- Greetings, identity questions, capability checks, and ordinary conversational turns are usually DIRECT with needs_evidence=false and suggested_pattern=reply.\n- Do not require workspace evidence for simple turns like \"hi\", \"who are you?\", or general knowledge unless the user explicitly asks about the current workspace or project.\n- Be strict.\n"
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
        system_prompt: "You select reasoning formulas for Elma.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"primary\": \"one formula name\",\n  \"alternatives\": [\"...\", \"...\"],\n  \"reason\": \"one short sentence\",\n  \"memory_id\": \"optional formula memory id or empty string\"\n}\n\nPreferred built-in formulas:\n- capability_reply\n- reply_only\n- inspect_reply\n- inspect_summarize_reply\n- inspect_decide_reply\n- inspect_edit_verify_reply\n- execute_reply\n- plan_reply\n- masterplan_reply\n- cleanup_safety_review\n- code_search_and_quote\n- config_compare\n\nRules:\n- Use the provided scope and memory candidates.\n- Prefer memory candidates with stronger success_count, lower failure_count, and a closer objective/program signature match.\n- Return memory_id only when the example objective and operation closely match the current request.\n- Do not reuse generic execute_reply memories for ranking, prioritization, top-N, or selection tasks unless the memory itself clearly includes that kind of choosing logic.\n- Greetings, identity questions, and simple conversational turns should usually prefer reply_only.\n- Capability-only questions should usually prefer capability_reply.\n- Cleanup safety questions should usually prefer cleanup_safety_review or inspect_decide_reply.\n- Ranking, prioritization, and selection requests about workspace items should usually prefer inspect_decide_reply or inspect_summarize_reply rather than generic execute_reply.\n- Editing, patching, creating, rewriting, or updating local files should usually prefer inspect_edit_verify_reply.\n- Code/file understanding should usually prefer code_search_and_quote, inspect_reply, or inspect_summarize_reply.\n- Requests to analyze the current project should usually prefer inspect_summarize_reply.\n- Requests to show, list, print, display, or count real workspace output should usually prefer execute_reply or inspect_reply rather than a meta-summary.\n- Direct terminal execution requests should usually prefer execute_reply.\n- Keep alternatives short and relevant.\n"
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
        system_prompt: "You are Elma's workflow planner.\n\nReturn ONLY one valid JSON object. No prose.\n\nSchema:\n{\n  \"objective\": \"short objective\",\n  \"complexity\": \"DIRECT\" | \"INVESTIGATE\" | \"MULTISTEP\" | \"OPEN_ENDED\",\n  \"risk\": \"LOW\" | \"MEDIUM\" | \"HIGH\",\n  \"needs_evidence\": true | false,\n  \"scope\": {\n    \"objective\": \"short scope objective\",\n    \"focus_paths\": [\"...\"],\n    \"include_globs\": [\"...\"],\n    \"exclude_globs\": [\"...\"],\n    \"query_terms\": [\"...\"],\n    \"expected_artifacts\": [\"...\"],\n    \"reason\": \"one short sentence\"\n  },\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Plan only the objective, complexity, risk, evidence need, and scope.\n- Do not choose formulas.\n- Do not choose memory candidates.\n- Prefer tighter scope and fewer assumptions.\n- Greetings and direct conversational turns should usually be DIRECT with needs_evidence=false and minimal scope.\n- Project/code/file questions should usually need inspect-oriented scope with the narrowest relevant paths.\n- Ranking, prioritization, and top-N tasks over workspace items usually need evidence before any later selection or display.\n- Editing requests usually need workspace evidence first unless the edit target is already explicit and small.\n- Keep the reason short and concrete.\n"
            .to_string(),
    }
}
