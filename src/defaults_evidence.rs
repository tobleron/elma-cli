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
        ("rephrase_intention.toml", default_rephrase_intention_config(base_url, model)),
        ("angel_helper.toml", default_angel_helper_config(base_url, model)),
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
        // JSON Pipeline Intel Units (Task 008 Phase 3)
        ("text_generator.toml", default_text_generator_config(base_url, model)),
        ("json_converter.toml", default_json_converter_config(base_url, model)),
        ("verify_checker.toml", default_verify_checker_config(base_url, model)),
        ("json_repair.toml", default_json_repair_config(base_url, model)),
    ]
}

pub(crate) fn managed_profile_file_names() -> Vec<&'static str> {
    managed_profile_specs("", "").into_iter().map(|(name, _)| name).collect()
}

// ============================================================================
// JSON Pipeline Intel Units (Task 008 Phase 3)
// ============================================================================

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
// Intel Functions for JSON Pipeline (Task 008 Phase 3)
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
                content: format!("Convert this reasoning into simple action text:\n\n{}", reasoning),
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
        problems.iter().map(|p| format!("- {}", p)).collect::<Vec<_>>().join("\n")
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
    pub status: String,  // "ok" or "problems"
    pub problems: Vec<String>,
}

// ============================================================================
// Angel Helper - Intention Clarification (Task 010)
// ============================================================================

pub(crate) fn default_angel_helper_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "angel_helper".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.7,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 256,
        timeout_s: 120,
        system_prompt: r#"Determine user intention and express what is the most appropriate way to respond.
"#
            .to_string(),
    }
}

/// Angel Helper: Inspire Elma on how to respond
pub(crate) async fn angel_helper_intention(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    rephrased_objective: &str,  // Takes rephrased intention as input
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
                content: rephrased_objective.to_string(),  // Use rephrased objective
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

/// Parse helper response to extract intention type
pub(crate) fn parse_helper_intention(helper_response: &str) -> &str {
    let response_upper = helper_response.to_uppercase();
    if response_upper.starts_with("ACTION:") {
        "ACTION"
    } else if response_upper.starts_with("INFO:") {
        "INFO"
    } else if response_upper.starts_with("CHAT:") {
        "CHAT"
    } else {
        "UNKNOWN"
    }
}

// ============================================================================
// Rephrase Intention - Clarify User Input (Task 010 Phase 2)
// ============================================================================

pub(crate) fn default_rephrase_intention_config(base_url: &str, model: &str) -> Profile {
    Profile {
        version: 1,
        name: "rephrase_intention".to_string(),
        base_url: base_url.to_string(),
        model: model.to_string(),
        temperature: 0.3,
        top_p: 1.0,
        repeat_penalty: 1.0,
        reasoning_format: "none".to_string(),
        max_tokens: 128,
        timeout_s: 120,
        system_prompt: r#"You rephrase user messages as clear objective statements.

Principles:
- Express what the user wants to achieve, not how
- Use action verbs for requests (list, show, find, create, edit, delete)
- Use knowledge verbs for questions (explain, describe, summarize)
- Keep it concise and specific
- Preserve the original intent faithfully

Output format:
- One clear sentence
- No markdown
- No explanations
"#
            .to_string(),
    }
}

/// Rephrase user intention as clear objective
pub(crate) async fn rephrase_user_intention(
    client: &reqwest::Client,
    chat_url: &Url,
    cfg: &Profile,
    user_message: &str,
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
                content: user_message.to_string(),
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
        _ => "simplify"
    }
}
