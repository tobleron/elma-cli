//! @efficiency-role: data-model
//!
//! Defaults - Evidence and Tune Configurations

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
        system_prompt: "You decide how Elma should present shell evidence.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"mode\": \"RAW\" | \"COMPACT\" | \"RAW_PLUS_COMPACT\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- RAW: use when the user explicitly asks to run/execute/show a command (e.g., \"run tree\", \"run cargo test\", \"show files\"). Also use when the command output is short (<50 lines) and the user wants to see exact output.\n- COMPACT: use when the user wants explanation, summary, analysis, comparison, or when raw output would be very noisy (>100 lines). Also use for pure chat/conversational turns with no actual command execution.\n- RAW_PLUS_COMPACT: use when exact output matters but a short explanation also helps. Use when step has artifact_path. Use when user asks for both output AND summary.\n\nCRITICAL RULE FOR COMMAND EXECUTION:\n- If user message contains \"run <command>\", \"execute\", \"show output\", or names a specific command (tree, cargo, ls, git, etc.), prefer RAW or RAW_PLUS_COMPACT.\n- If step_results show a command was actually executed (command field is not null), prefer RAW or RAW_PLUS_COMPACT unless output is extremely long.\n- If step_results show only a reply step with no command execution, use COMPACT.\n\nDecision priority:\n1. User explicitly asks for raw output → RAW\n2. User asks for command execution → RAW or RAW_PLUS_COMPACT\n3. Command was executed with short output → RAW\n4. Command was executed with long output → RAW_PLUS_COMPACT\n5. User wants summary/analysis → COMPACT\n6. No command executed (reply only) → COMPACT\n\nBe strict and concise.\n"
            .to_string(),
    }
}

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
        temperature: 0.4,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: "You verify whether a repaired shell command preserves the original task semantics.\n\nReturn ONLY one valid JSON object.\n\nSchema:\n{\n  \"status\": \"accept\" | \"reject\",\n  \"reason\": \"one short sentence\"\n}\n\nRules:\n- Accept only if the repaired command keeps the same operation type and user intent.\n- Reject if the repaired command changes the task into listing instead of reading, searching instead of editing, printing names instead of contents, or any other material semantic shift.\n- Use the objective and step purpose to preserve the stage of the workflow. A candidate-inspection or file-listing step must not become a content-reading or selection step.\n- If the original command depends on selected items or placeholder-based inputs, the repaired command must preserve that dependency instead of replacing it with guessed filenames or broader searches.\n- Portability, quoting, and syntax fixes are acceptable only when the task meaning stays the same.\n- Be strict.\n"
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
        system_prompt: "Judge if the executed workflow satisfied the user's request. Return JSON: {\"status\":\"ok\"|\"retry\",\"reason\":\"...\"}"
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
        system_prompt: "Define the scope objective for the task. Return JSON: {\"objective\":\"...\"}"
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
        system_prompt: "Present the final answer to the user in plain terminal text."
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

pub(crate) fn default_status_message_generator_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "status_message_generator".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.3,
        top_p: 0.95,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 64,
        timeout_s: 120,
        system_prompt: "Generate an ultra-concise status message explaining what Elma is doing now. Return JSON: {\"status\":\"one line, max 10 words\"}"
            .to_string(),
    }
}

pub(crate) fn managed_profile_specs(base_url: &str, model: &str) -> Vec<(&'static str, Profile)> {
    vec![
        ("_elma.config", default_elma_config(base_url, model)),
        ("intention.toml", default_intention_config(base_url, model)),
        ("gate.toml", default_gate_config(base_url, model)),
        ("gate_why.toml", default_gate_why_config(base_url, model)),
        ("tooler.toml", default_tooler_config(base_url, model)),
        ("action_type.toml", default_action_type_config(base_url, model)),
        ("planner_master.toml", default_planner_master_config(base_url, model)),
        ("planner.toml", default_planner_config(base_url, model)),
        ("decider.toml", default_decider_config(base_url, model)),
        ("selector.toml", default_selector_config(base_url, model)),
        ("summarizer.toml", default_summarizer_config(base_url, model)),
        ("formatter.toml", default_formatter_config(base_url, model)),
        ("json_outputter.toml", default_json_outputter_config(base_url, model)),
        ("final_answer_extractor.toml", default_final_answer_extractor_config(base_url, model)),
        ("calibration_judge.toml", default_calibration_judge_config(base_url, model)),
        ("complexity_assessor.toml", default_complexity_assessor_config(base_url, model)),
        ("evidence_need_assessor.toml", default_evidence_need_assessor_config(base_url, model)),
        ("action_need_assessor.toml", default_action_need_assessor_config(base_url, model)),
        ("pattern_suggester.toml", default_pattern_suggester_config(base_url, model)),
        ("formula_selector.toml", default_formula_selector_config(base_url, model)),
        ("formula_memory_matcher.toml", default_formula_memory_matcher_config(base_url, model)),
        ("workflow_planner.toml", default_workflow_planner_config(base_url, model)),
        ("workflow_complexity_planner.toml", default_workflow_complexity_planner_config(base_url, model)),
        ("workflow_reason_planner.toml", default_workflow_reason_planner_config(base_url, model)),
        ("evidence_mode.toml", default_evidence_mode_config(base_url, model)),
        ("command_repair.toml", default_command_repair_config(base_url, model)),
        ("command_reviser.toml", default_command_reviser_config(base_url, model)),
        ("execution_mode_setter.toml", default_execution_mode_setter_config(base_url, model)),
        ("task_semantics_guard.toml", default_task_semantics_guard_config(base_url, model)),
        ("execution_sufficiency.toml", default_execution_sufficiency_config(base_url, model)),
        ("execution_program_repair.toml", default_execution_program_repair_config(base_url, model)),
        ("outcome_verifier.toml", default_outcome_verifier_config(base_url, model)),
        ("memory_gate.toml", default_memory_gate_config(base_url, model)),
        ("command_preflight.toml", default_command_preflight_config(base_url, model)),
        ("scope_builder.toml", default_scope_builder_config(base_url, model)),
        ("scope_objective_builder.toml", default_scope_objective_builder_config(base_url, model)),
        ("evidence_compactor.toml", default_evidence_compactor_config(base_url, model)),
        ("artifact_classifier.toml", default_artifact_classifier_config(base_url, model)),
        ("result_presenter.toml", default_result_presenter_config(base_url, model)),
        ("claim_checker.toml", default_claim_checker_config(base_url, model)),
        ("claim_revision_advisor.toml", default_claim_revision_advisor_config(base_url, model)),
        ("critic.toml", default_critic_config(base_url, model)),
        ("program_repair.toml", default_program_repair_config(base_url, model)),
        ("orchestrator.toml", default_orchestrator_config(base_url, model)),
        ("refinement.toml", default_refinement_config(base_url, model)),
        ("reflection.toml", default_reflection_config(base_url, model)),
        ("logical_reviewer.toml", default_logical_reviewer_config(base_url, model)),
        ("logical_program_repair.toml", default_logical_program_repair_config(base_url, model)),
        ("efficiency_reviewer.toml", default_efficiency_reviewer_config(base_url, model)),
        ("efficiency_program_repair.toml", default_efficiency_program_repair_config(base_url, model)),
        ("risk_reviewer.toml", default_risk_reviewer_config(base_url, model)),
        ("meta_review.toml", default_meta_review_config(base_url, model)),
        ("router.toml", default_router_config(base_url, model)),
        ("mode_router.toml", default_mode_router_config(base_url, model)),
        ("speech_act.toml", default_speech_act_config(base_url, model)),
        ("intention_tune.toml", default_intention_tune_config(base_url, model)),
        ("status_message_generator.toml", default_status_message_generator_config(base_url, model)),
    ]
}

pub(crate) fn managed_profile_file_names() -> Vec<&'static str> {
    managed_profile_specs("", "").into_iter().map(|(name, _)| name).collect()
}

pub(crate) fn get_retry_prompt_variant(attempt: u32) -> &'static str {
    match attempt {
        0 => "standard",
        1 => "step-by-step",
        2 => "challenge",
        _ => "simplify"
    }
}
