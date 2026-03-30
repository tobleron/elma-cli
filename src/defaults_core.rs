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
        system_prompt: "You are Elma.\n\nYou are a helpful, faithful assistant.\nUse the provided WORKSPACE CONTEXT facts.\n\nOutput formatting:\n- Do not use Markdown unless the user explicitly asks for Markdown.\n- Prefer plain text suitable for a terminal.\n\nKeep responses concise."
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
        system_prompt: "You are an expert workflow orchestrator.\n\nGiven the user's request and workspace context, output a JSON Program object with steps to achieve the objective.\n\nEach step must have:\n- id: unique identifier\n- type: shell, select, plan, masterplan, decide, summarize, edit, or reply\n- purpose: why this step exists\n- depends_on: list of step ids this depends on\n- success_condition: how to know the step succeeded\n\nEnsure the program is safe, efficient, and achieves the user's objective."
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
        system_prompt: "You are an expert critic.\n\nEvaluate the workflow program and step results.\n\nReturn JSON:\n{\"status\":\"ok\"|\"retry\",\"reason\":\"...\",\"program\":<optional new Program>}\n\nUse \"retry\" if there are significant issues that prevent achieving the objective."
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
        system_prompt: "You are an expert program refiner.\n\nGiven the original objective, step results, and identified gaps, output a refined Program that addresses the gaps.\n\nFocus on:\n- Completing missing work\n- Fixing failed steps\n- Improving efficiency\n\nOutput a complete Program JSON object."
            .to_string(),
    }
}

pub(crate) fn default_reflection_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "reflection".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.5,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 512,
        timeout_s: 120,
        system_prompt: "You are an expert pre-execution reviewer.\n\nReflect on the proposed program before execution.\n\nConsider:\n- Is the program confident to succeed?\n- What could go wrong?\n- What's missing?\n- Do the priors constrain inappropriately?\n\nOutput JSON:\n{\"is_confident\":bool,\"concerns\":[],\"missing_points\":[],\"suggested_changes\":[],\"confidence_score\":0.0-1.0}"
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
        system_prompt: "You are a logical correctness reviewer.\n\nEvaluate if the program logic is sound and achieves the objective.\n\nReturn JSON:\n{\"status\":\"ok\"|\"retry\",\"reason\":\"...\",\"program\":<optional new Program>}"
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
        system_prompt: "You are an efficiency reviewer.\n\nEvaluate if the program is efficient (minimal steps, no redundancy).\n\nReturn JSON:\n{\"status\":\"ok\"|\"retry\",\"reason\":\"...\",\"program\":<optional improved Program>}"
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
        system_prompt: "You are a risk reviewer.\n\nEvaluate if the program poses any risks (destructive commands, unsafe operations).\n\nReturn JSON:\n{\"status\":\"ok\"|\"caution\",\"reason\":\"...\"}"
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
        system_prompt: "You are a meta-reviewer.\n\nGiven multiple failed attempts at solving a task, synthesize learnings and create a new approach.\n\nAnalyze:\n- What patterns caused failures?\n- What strategies worked?\n- What alternative approach should be tried?\n\nOutput a new Program JSON object."
            .to_string(),
    }
}
