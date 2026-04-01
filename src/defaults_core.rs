//! @efficiency-role: data-model
//!
//! Defaults - Core and Reviewer Configurations

use crate::*;

pub(crate) fn default_elma_config(base_url: &str, model: &str) -> Profile {
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
        system_prompt: "You are Elma, a helpful and faithful assistant."
            .to_string(),
    }
}

pub(crate) fn default_intention_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_gate_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "gate".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 8,
        timeout_s: 120,
        system_prompt: "Classify the user's message into exactly one token.\n\nReturn exactly one of:\nCHAT\nACTION\n\nGuidance:\n- ACTION if the user wants any terminal/workspace action (commands, file operations, build/test, search, etc).\n- CHAT otherwise.\n\nNo other text."
            .to_string(),
    }
}

pub(crate) fn default_gate_why_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "gate_why".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 64,
        timeout_s: 120,
        system_prompt: "Explain in exactly ONE short sentence why you classified the user message as CHAT (not ACTION). Do not include any extra lines."
            .to_string(),
    }
}

pub(crate) fn default_tooler_config(base_url: &str, model: &str) -> Profile {
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

pub(crate) fn default_orchestrator_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "orchestrator".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.0,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "Create a JSON Program object with steps to achieve the user's objective.\n\nStep types and their required fields:\n- shell: {\"id\":\"s1\",\"type\":\"shell\",\"cmd\":\"command\",\"purpose\":\"why\",\"depends_on\":[],\"success_condition\":\"done when\"}\n- reply: {\"id\":\"r1\",\"type\":\"reply\",\"instructions\":\"what to say\",\"purpose\":\"why\",\"depends_on\":[],\"success_condition\":\"done when\"}\n- plan: {\"id\":\"p1\",\"type\":\"plan\",\"goal\":\"objective\",\"purpose\":\"why\",\"depends_on\":[],\"success_condition\":\"done when\"}\n- select: {\"id\":\"sel1\",\"type\":\"select\",\"instructions\":\"what to select\",\"purpose\":\"why\",\"depends_on\":[],\"success_condition\":\"done when\"}\n- decide: {\"id\":\"d1\",\"type\":\"decide\",\"prompt\":\"question to answer\",\"purpose\":\"why\",\"depends_on\":[],\"success_condition\":\"done when\"}\n- edit: {\"id\":\"e1\",\"type\":\"edit\",\"path\":\"file\",\"operation\":\"create|update|delete\",\"content\":\"new content\",\"purpose\":\"why\",\"depends_on\":[],\"success_condition\":\"done when\"}\n\nRules:\n- Use ONLY the fields specified for each step type.\n- Do NOT use 'goal' for reply steps - use 'instructions'.\n- Do NOT mix fields from different step types.\n- Every step must have: id, type, purpose, depends_on, success_condition.\n- Output valid JSON only. No prose."
            .to_string(),
    }
}

/// GBNF grammar for JSON Program output - ensures valid JSON structure
pub(crate) fn json_program_grammar() -> String {
    r#"
root ::= program
program ::= "{" ws "\"objective\"" ws ":" ws string ws "," ws "\"steps\"" ws ":" ws "[" ws (step (ws "," ws step)*)? ws "]" ws "}"
step ::= "{" ws "\"id\"" ws ":" ws string ws "," ws "\"type\"" ws ":" ws step_type ws "," ws "\"purpose\"" ws ":" ws string ws "," ws "\"depends_on\"" ws ":" ws "[" ws (string (ws "," ws string)*)? ws "]" ws "," ws "\"success_condition\"" ws ":" ws string ws (step_fields)* ws "}"
step_type ::= "\"shell\"" | "\"reply\"" | "\"plan\"" | "\"select\"" | "\"decide\"" | "\"summarize\"" | "\"edit\"" | "\"masterplan\""
step_fields ::= (step_field_shell | step_field_reply | step_field_plan | step_field_select | step_field_decide | step_field_edit)
step_field_shell ::= "," ws "\"cmd\"" ws ":" ws string
step_field_reply ::= "," ws "\"instructions\"" ws ":" ws string
step_field_plan ::= "," ws "\"goal\"" ws ":" ws string
step_field_select ::= "," ws "\"instructions\"" ws ":" ws string
step_field_decide ::= "," ws "\"prompt\"" ws ":" ws string
step_field_edit ::= "," ws "\"path\"" ws ":" ws string "," ws "\"operation\"" ws ":" ws edit_op "," ws "\"content\"" ws ":" ws string
edit_op ::= "\"create\"" | "\"update\"" | "\"delete\""
string ::= "\"" char* "\""
char ::= [^"\\\r\n] | "\\" escape
escape ::= ["\\bfnrt]
ws ::= [ \t\n\r]*
"#.to_string()
}

pub(crate) fn default_critic_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "critic".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: r#"Evaluate if the workflow program and step results achieve the objective.

Return ONLY one valid JSON object. No prose. No thinking tokens. No code fences.

Schema:
{
  "status": "ok" | "retry",
  "reason": "one short sentence"
}

Rules:
- Output MUST be valid JSON only
- Do not include thinking tokens or reasoning outside JSON
- Do not use markdown code fences
- Keep reason concise (one sentence)
- If uncertain, return status="ok" with conservative reason
"#
            .to_string(),
    }
}

pub(crate) fn default_program_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "program_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "Repair a flawed program based on evaluation feedback. Output a complete Program JSON object."
            .to_string(),
    }
}

pub(crate) fn default_refinement_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "refinement".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "Fill gaps in the program to complete the objective. Output a complete Program JSON object."
            .to_string(),
    }
}

pub(crate) fn default_reflection_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "reflection".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,  // Increased from 0.5 for more balanced assessment
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "Identify pre-execution risks in the proposed program. Return JSON: {\"is_confident\":bool,\"concerns\":[],\"missing_points\":[]}"
            .to_string(),
    }
}

pub(crate) fn default_logical_reviewer_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "logical_reviewer".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: r#"Evaluate if the program logic is sound.

Return ONLY one valid JSON object. No prose. No thinking tokens. No code fences.

Schema:
{
  "status": "ok" | "retry",
  "reason": "one short sentence"
}

Rules:
- Output MUST be valid JSON only
- Do not include thinking tokens or reasoning outside JSON
- Do not use markdown code fences
- Keep reason concise (one sentence)
- If uncertain, return status="ok" with conservative reason
"#
            .to_string(),
    }
}

pub(crate) fn default_logical_program_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "logical_program_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "Repair a program with logical flaws. Output a complete Program JSON object."
            .to_string(),
    }
}

pub(crate) fn default_efficiency_reviewer_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "efficiency_reviewer".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: r#"Evaluate if the program uses minimal steps without redundancy.

Return ONLY one valid JSON object. No prose. No thinking tokens. No code fences.

Schema:
{
  "status": "ok" | "retry",
  "reason": "one short sentence"
}

Rules:
- Output MUST be valid JSON only
- Do not include thinking tokens or reasoning outside JSON
- Do not use markdown code fences
- Keep reason concise (one sentence)
- If uncertain, return status="ok" with conservative reason
"#
            .to_string(),
    }
}

pub(crate) fn default_efficiency_program_repair_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "efficiency_program_repair".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "Repair a program to improve efficiency. Output a complete Program JSON object."
            .to_string(),
    }
}

pub(crate) fn default_risk_reviewer_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "risk_reviewer".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.4,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: r#"Evaluate if the program contains risky commands.

Return ONLY one valid JSON object. No prose. No thinking tokens. No code fences.

Schema:
{
  "status": "ok" | "caution",
  "reason": "one short sentence"
}

Rules:
- Output MUST be valid JSON only
- Do not include thinking tokens or reasoning outside JSON
- Do not use markdown code fences
- Keep reason concise (one sentence)
- If uncertain, return status="ok" with conservative reason
"#
            .to_string(),
    }
}

pub(crate) fn default_meta_review_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "meta_review".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.6,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 2048,
        timeout_s: 120,
        system_prompt: "Synthesize a new approach from multiple failed attempts. Output a new Program JSON object."
            .to_string(),
    }
}
