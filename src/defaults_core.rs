//! @efficiency-role: infra-config
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
        system_prompt: "You are Elma, a helpful and faithful assistant.".to_string(),
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
        system_prompt: "You are Elma's legacy tooler.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"tooler is disabled; use action DSL tool loop\""
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
        system_prompt: "You are Elma's legacy program orchestrator.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"orchestrator JSON program generation is disabled; use action DSL tool loop\""
            .to_string(),
    }
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
        system_prompt: r#"You are Elma's workflow reviewer.
Return a single DSL line.
Output format:
OK reason="workflow claim supported by evidence"
or
RETRY reason="workflow claim not supported by evidence"
or
CAUTION reason="minor concern: missing error handling"
Principles:
- Return retry when the workflow claim is not supported by the provided evidence or when the workflow is materially flawed for its purpose.
- Return ok when the evidence clearly supports the workflow result."#
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
        system_prompt: "You are Elma's legacy program repair specialist.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"program repair JSON is disabled; use action DSL repair loop\""
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
        system_prompt: "You are Elma's legacy refinement specialist.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"refinement program JSON is disabled; use action DSL repair loop\""
            .to_string(),
    }
}

pub(crate) fn default_reflection_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "reflection".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7, // Increased from 0.5 for more balanced assessment
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: r#"You are Elma's pre-execution reflection unit.
Return a single DSL line.
Output format:
REFLECT confidence=0.85 justification="program likely succeeds"
Principles:
- Score confidence in whether the proposed program will achieve the objective reliably.
- Be honest and critical.
- Keep justification short and decision-relevant."#
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

Return one verdict DSL line:
OK reason="short reason"
RETRY reason="short reason"

Rules:
- Output exactly one verdict line.
- Keep reason concise.
- Do not use JSON, markdown, or prose outside the DSL.
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
        system_prompt: "You are Elma's legacy program repair specialist.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"logical program repair JSON is disabled; use action DSL repair loop\""
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

Return one verdict DSL line:
OK reason="short reason"
RETRY reason="short reason"

Rules:
- Output exactly one verdict line.
- Keep reason concise.
- Do not use JSON, markdown, or prose outside the DSL.
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
        system_prompt: "You are Elma's legacy program repair specialist.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"efficiency program repair JSON is disabled; use action DSL repair loop\""
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

Return one verdict DSL line:
OK reason="short reason"
CAUTION reason="short reason"

Rules:
- Output exactly one verdict line.
- Keep reason concise.
- Do not use JSON, markdown, or prose outside the DSL.
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
        system_prompt: "You are Elma's legacy meta-review synthesizer.\n\nThis profile is deprecated by the compact DSL action protocol.\n\nReturn exactly one DSL line and nothing else:\nDEPRECATED reason=\"meta review program JSON synthesis is disabled; use action DSL repair loop\""
            .to_string(),
    }
}
